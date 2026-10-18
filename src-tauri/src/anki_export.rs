//! Anki-compatible UTF-8 TSV export with HTML fields and a native save dialog.

use crate::history::{Entry, HistoryStore, source_key};
use serde::Serialize;
use std::{io::Write, path::Path};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub count: usize,
}

fn escape_anki_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "<br>")
}
/// Import headers configure Tab, HTML, Front/Back fields, and the tags column.
/// CSV quoting protects literal tabs; HTML escaping preserves visible source text.
pub fn format_entries(entries: &[Entry]) -> Result<Vec<u8>, String> {
    let mut output =
        b"#separator:Tab\n#html:true\n#columns:Front\tBack\tTags\n#tags column:3\n".to_vec();
    {
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .from_writer(&mut output);
        for entry in entries {
            writer
                .write_record([
                    escape_anki_html(&source_key(&entry.source_text)),
                    escape_anki_html(&entry.translation),
                    "rustlens".into(),
                ])
                .map_err(|e| e.to_string())?;
        }
        writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(output)
}
/// Write beside the destination and replace it atomically after flushing.
pub fn save_entries(entries: &[Entry], path: &Path) -> Result<(), String> {
    if entries.is_empty() {
        return Err("Познач хоча б одну фразу для навчання.".into());
    }
    let bytes = format_entries(entries)?;
    let directory = path
        .parent()
        .ok_or("Не вдалося визначити папку експорту.")?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path)
        .map_err(|e| format!("Не вдалося зберегти файл Anki: {e}"))?;
    Ok(())
}
#[tauri::command]
pub async fn export_anki(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Option<ExportResult>, String> {
    if window.label() != "main" {
        return Err("Експорт доступний у головному вікні історії.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let entries = app.state::<HistoryStore>().selected()?;
        if entries.is_empty() {
            return Err("Познач хоча б одну фразу для навчання.".into());
        }
        let file = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Експорт фраз в Anki")
            .add_filter("Anki text", &["txt"])
            .set_file_name("RustLens-Anki.txt")
            .blocking_save_file();
        let Some(file) = file else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|e| e.to_string())?;
        save_entries(&entries, &path)?;
        Ok(Some(ExportResult {
            path: path.display().to_string(),
            count: entries.len(),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(source: &str, translation: &str) -> Entry {
        Entry {
            id: 1,
            source_text: source.into(),
            translation: translation.into(),
            model: "model".into(),
            created_at: String::new(),
            updated_at: String::new(),
            selected_for_study: true,
        }
    }

    #[test]
    fn utf8_html_and_tsv_fields_round_trip() {
        let output = format_entries(&[entry(
            "Say \"hello\"\t<player>",
            "Привіт & дякую!\nДругий\tрядок",
        )])
        .unwrap();
        let text = String::from_utf8(output).unwrap();
        let records = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .comment(Some(b'#'))
            .from_reader(text.as_bytes())
            .records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].len(), 3);
        assert_eq!(&records[0][0], "Say &quot;hello&quot; &lt;player&gt;");
        assert_eq!(&records[0][1], "Привіт &amp; дякую!<br>Другий\tрядок");
        assert_eq!(&records[0][2], "rustlens");
    }

    #[test]
    fn export_contains_only_selected_and_overwriting_is_safe() {
        let dir = tempfile::tempdir().unwrap();
        let store = HistoryStore::open(&dir.path().join("history.sqlite")).unwrap();
        let chosen = store
            .record("Open the gate.", "Відчиніть ворота.", "model")
            .unwrap();
        store.record("Not chosen", "Не вибрано", "model").unwrap();
        store.set_study(chosen.id, true).unwrap();
        let path = dir.path().join("anki.txt");
        save_entries(&store.selected().unwrap(), &path).unwrap();
        let first = std::fs::read(&path).unwrap();
        save_entries(&store.selected().unwrap(), &path).unwrap();
        assert_eq!(first, std::fs::read(path).unwrap());
        assert!(!String::from_utf8(first).unwrap().contains("Not chosen"));
    }

    #[test]
    fn empty_selection_does_not_create_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        assert!(save_entries(&[], &path).is_err());
        assert!(!path.exists());
    }

    #[test]
    #[ignore = "Writes an import fixture to RUSTLENS_TEST_EXPORT"]
    fn writes_anki_import_fixture() {
        let path = std::env::var("RUSTLENS_TEST_EXPORT").expect("Set a temporary fixture path");
        save_entries(
            &[
                entry(
                    "Keep\nan eye on the gate.",
                    "Стежте за воротами.\nЦитати & <теги>.",
                ),
                entry("Say \"hello\"\t<player>", "Скажи \"привіт\"."),
            ],
            Path::new(&path),
        )
        .unwrap();
    }
}
