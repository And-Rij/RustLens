//! Persistent translation history, server-side search, and study bookmarks.
//!
//! A single SQLite connection is protected by a mutex. Command handlers run
//! database operations on blocking workers so they do not block the async runtime.

use rusqlite::{Connection, Row, params};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

pub const PAGE_SIZE: u32 = 20;

pub struct HistoryStore {
    // Keep initialization errors here so translation can work without history.
    db: Mutex<Result<Connection, String>>,
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: i64,
    pub source_text: String,
    pub translation: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub selected_for_study: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<Entry>,
    pub total: u32,
    // All bookmarks, including entries excluded by the current search/filter.
    pub selected_count: u32,
    pub page: u32,
    pub page_size: u32,
    pub storage_path: String,
}

#[derive(Clone)]
pub struct SavedTranslation {
    pub id: i64,
    pub selected_for_study: bool,
}

/// Ignore OCR whitespace differences while preserving case and punctuation.
/// The same key also keeps the first Anki field stable across repeat exports.
pub fn source_key(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl HistoryStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let db = Connection::open(path).map_err(|error| error.to_string())?;
        db.busy_timeout(Duration::from_secs(3))
            .map_err(|error| error.to_string())?;
        db.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS translations (
                id INTEGER PRIMARY KEY,
                source_key TEXT NOT NULL UNIQUE,
                source_text TEXT NOT NULL,
                translation TEXT NOT NULL,
                model TEXT NOT NULL,
                search_text TEXT NOT NULL,
                selected_for_study INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
            );

            CREATE INDEX IF NOT EXISTS translations_updated
                ON translations(updated_at DESC, id DESC);
            CREATE INDEX IF NOT EXISTS translations_study
                ON translations(selected_for_study);
            "#,
        )
        .map_err(|error| error.to_string())?;

        Ok(Self {
            db: Mutex::new(Ok(db)),
            path: path.to_path_buf(),
        })
    }

    pub fn unavailable(error: String) -> Self {
        Self {
            db: Mutex::new(Err(error)),
            path: PathBuf::new(),
        }
    }

    /// Serialize access and turn SQLite or initialization errors into UI messages.
    fn access<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    ) -> Result<T, String> {
        let guard = self
            .db
            .lock()
            .map_err(|_| "Історія недоступна. Перезапусти RustLens.")?;
        let db = guard
            .as_ref()
            .map_err(|error| format!("Не вдалося відкрити історію: {error}"))?;

        operation(db).map_err(|error| format!("Помилка історії: {error}"))
    }

    /// Insert a phrase, or update its translation without changing its bookmark.
    pub fn record(
        &self,
        source: &str,
        translation: &str,
        model: &str,
    ) -> Result<SavedTranslation, String> {
        let key = source_key(source);
        if key.is_empty() || translation.trim().is_empty() {
            return Err("Порожній переклад не зберігається в історію.".into());
        }

        // SQLite's default case folding is ASCII-only; normalize Unicode in Rust.
        let search = format!("{source}\n{translation}\n{model}").to_lowercase();
        self.access(|db| {
            db.query_row(
                r#"
                INSERT INTO translations
                    (source_key, source_text, translation, model, search_text)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(source_key) DO UPDATE SET
                    source_text = excluded.source_text,
                    translation = excluded.translation,
                    model = excluded.model,
                    search_text = excluded.search_text,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
                RETURNING id, selected_for_study
                "#,
                params![key, source.trim(), translation.trim(), model, search],
                |row| {
                    Ok(SavedTranslation {
                        id: row.get(0)?,
                        selected_for_study: row.get(1)?,
                    })
                },
            )
        })
    }

    /// Search and paginate in SQLite instead of loading all records into React.
    pub fn query(&self, page: u32, search: &str, only_study: bool) -> Result<HistoryPage, String> {
        if search.chars().count() > 500 {
            return Err("Залиш до 500 символів у пошуку.".into());
        }

        // Treat LIKE wildcards as literal text entered in the search field.
        let escaped = search
            .trim()
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");

        self.access(|db| {
            let total: u32 = db.query_row(
                r#"
                SELECT count(*) FROM translations
                WHERE search_text LIKE ?1 ESCAPE '\'
                    AND (?2 = 0 OR selected_for_study = 1)
                "#,
                params![pattern, only_study],
                |row| row.get(0),
            )?;
            let selected_count: u32 = db.query_row(
                "SELECT count(*) FROM translations WHERE selected_for_study = 1",
                [],
                |row| row.get(0),
            )?;

            // Changing the search or study filter may invalidate the requested page.
            let pages = total.div_ceil(PAGE_SIZE).max(1);
            let page = page.clamp(1, pages);
            let offset = (page - 1) * PAGE_SIZE;
            let mut statement = db.prepare(
                r#"
                SELECT id, source_text, translation, model,
                    created_at, updated_at, selected_for_study
                FROM translations
                WHERE search_text LIKE ?1 ESCAPE '\'
                    AND (?2 = 0 OR selected_for_study = 1)
                ORDER BY updated_at DESC, id DESC
                LIMIT ?3 OFFSET ?4
                "#,
            )?;
            let entries = statement
                .query_map(
                    params![pattern, only_study, PAGE_SIZE, offset],
                    entry_from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(HistoryPage {
                entries,
                total,
                selected_count,
                page,
                page_size: PAGE_SIZE,
                storage_path: self.path.display().to_string(),
            })
        })
    }

    pub fn set_study(&self, id: i64, selected: bool) -> Result<(), String> {
        let changed = self.access(|db| {
            db.execute(
                "UPDATE translations SET selected_for_study = ?1 WHERE id = ?2",
                params![selected, id],
            )
        })?;

        if changed == 0 {
            return Err("Запис історії не знайдено. Онови список.".into());
        }
        Ok(())
    }

    /// Return every bookmark for export, independently of the visible history page.
    pub fn selected(&self) -> Result<Vec<Entry>, String> {
        self.access(|db| {
            let mut statement = db.prepare(
                r#"
                SELECT id, source_text, translation, model,
                    created_at, updated_at, selected_for_study
                FROM translations
                WHERE selected_for_study = 1
                ORDER BY id
                "#,
            )?;
            statement.query_map([], entry_from_row)?.collect()
        })
    }
}

fn entry_from_row(row: &Row<'_>) -> Result<Entry, rusqlite::Error> {
    Ok(Entry {
        id: row.get(0)?,
        source_text: row.get(1)?,
        translation: row.get(2)?,
        model: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        selected_for_study: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn survives_reopening_and_preserves_bookmark_on_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let store = HistoryStore::open(&path).unwrap();
        let first = store
            .record("Keep\nan eye on the gate.", "Стежте за воротами.", "model")
            .unwrap();
        store.set_study(first.id, true).unwrap();
        drop(store);
        let store = HistoryStore::open(&path).unwrap();
        let saved = store
            .record("Keep an eye on the gate.", "Пильнуйте ворота.", "other")
            .unwrap();
        assert_eq!(saved.id, first.id);
        assert!(saved.selected_for_study);
        let page = store.query(1, "", false).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].translation, "Пильнуйте ворота.");
    }

    #[test]
    fn pagination_filtering_and_unicode_search() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("db")).unwrap();
        for index in 0..45 {
            store
                .record(
                    &format!("Sentence {index}"),
                    "Український ПЕРЕКЛАД",
                    "model",
                )
                .unwrap();
        }
        let page = store.query(2, "уКРаЇнСьКИй", false).unwrap();
        assert_eq!(page.total, 45);
        assert_eq!(page.entries.len(), 20);
        assert_eq!(store.query(3, "", false).unwrap().entries.len(), 5);
        store.set_study(page.entries[0].id, true).unwrap();
        assert_eq!(store.query(1, "", true).unwrap().total, 1);
        assert_eq!(store.selected().unwrap().len(), 1);
    }

    #[test]
    fn search_wildcards_are_literal_and_missing_id_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("db")).unwrap();
        store.record("100% sure_thing", "Напевно", "model").unwrap();
        store.record("Another sentence", "Інше", "model").unwrap();
        assert_eq!(store.query(1, "%", false).unwrap().total, 1);
        assert_eq!(store.query(1, "_", false).unwrap().total, 1);
        assert!(store.set_study(999, true).is_err());
    }
}
