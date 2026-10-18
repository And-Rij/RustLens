//! Runs each selected fragment through OCR, local translation, and history storage.
//! New requests invalidate previous work; only the current request may publish a result.

use crate::{capture::SelectionPreview, ocr::OcrService, ollama::OllamaService};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub selection_id: u64,
    pub request_id: u64,
    pub phase: &'static str,
    pub source_text: String,
    pub translation: String,
    pub error: Option<String>,
    pub model: String,
    pub ocr_ms: Option<u128>,
    pub translation_ms: Option<u128>,
    pub history_id: Option<i64>,
    pub selected_for_study: bool,
    pub storage_error: Option<String>,
}

#[derive(Default)]
pub struct Pipeline {
    // Incremented on start and cancel to reject stale asynchronous results.
    revision: AtomicU64,
    progress: Mutex<Option<Progress>>,
    pub cancelled: tokio::sync::Notify,
}

impl Pipeline {
    pub fn current(&self) -> Result<Option<Progress>, String> {
        self.progress
            .lock()
            .map(|p| p.clone())
            .map_err(|_| "Не вдалося отримати переклад.".into())
    }

    pub fn cancel(&self) {
        let revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        self.cancelled.notify_waiters();
        if let Ok(mut progress) = self.progress.lock()
            && let Some(progress) = progress.as_mut()
            && progress.phase != "done"
        {
            progress.request_id = revision;
            progress.phase = "error";
            progress.error = Some("Переклад скасовано. Можна повторити запит.".into());
        }
    }

    pub fn start(&self, selection_id: u64, text: Option<String>, model: String) -> Progress {
        let request_id = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        self.cancelled.notify_waiters();
        let progress = Progress {
            selection_id,
            request_id,
            phase: if text.is_some() {
                "translating"
            } else {
                "recognizing"
            },
            source_text: text.unwrap_or_default(),
            translation: String::new(),
            error: None,
            model,
            ocr_ms: None,
            translation_ms: None,
            history_id: None,
            selected_for_study: false,
            storage_error: None,
        };
        if let Ok(mut current) = self.progress.lock() {
            *current = Some(progress.clone());
        }
        progress
    }

    pub fn mark_study(&self, id: i64, selected: bool) -> Result<Option<Progress>, String> {
        let mut current = self
            .progress
            .lock()
            .map_err(|_| "Не вдалося оновити переклад.")?;
        if let Some(progress) = current.as_mut()
            && progress.history_id == Some(id)
        {
            progress.selected_for_study = selected;
            return Ok(Some(progress.clone()));
        }
        Ok(None)
    }

    pub fn is_current(&self, request: u64) -> bool {
        self.revision.load(Ordering::SeqCst) == request
    }

    fn update(&self, progress: &Progress) -> bool {
        if let Ok(mut current) = self.progress.lock() {
            if !self.is_current(progress.request_id) {
                return false;
            }
            *current = Some(progress.clone());
            return true;
        }
        false
    }

    fn publish(&self, app: &tauri::AppHandle, progress: &Progress) {
        if self.update(progress) {
            let _ = app.emit("translation-progress", progress);
        }
    }
}

fn models_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("models");
        if directory.is_dir() {
            return Ok(directory);
        }
    }
    app.path()
        .resource_dir()
        .map(|p| p.join("models"))
        .map_err(|e| e.to_string())
}

/// Start background work and return immediately so the desktop UI stays responsive.
/// Providing edited text skips OCR; a new capture supplies an image instead.
pub fn launch(
    app: tauri::AppHandle,
    selection: SelectionPreview,
    text: Option<String>,
) -> Result<(), String> {
    let pipeline = app.state::<Pipeline>();
    let progress = pipeline.start(selection.id, text, app.state::<OllamaService>().model()?);
    pipeline.publish(&app, &progress);
    tauri::async_runtime::spawn(async move {
        let mut progress = progress;
        let pipeline = app.state::<Pipeline>();
        if !pipeline.is_current(progress.request_id) {
            return;
        }
        let result = async {
            // CPU-bound OCR runs on a worker. Cancellation discards its result afterward.
            if progress.phase == "recognizing" {
                let directory = models_path(&app)?;
                let ocr_app = app.clone();
                let start = Instant::now();
                let source = tauri::async_runtime::spawn_blocking(move || {
                    let encoded = selection
                        .image_url
                        .strip_prefix("data:image/png;base64,")
                        .ok_or("Некоректне зображення фрагмента.")?;
                    let bytes = STANDARD.decode(encoded).map_err(|e| e.to_string())?;
                    let image = image::load_from_memory(&bytes)
                        .map_err(|e| e.to_string())?
                        .into_rgba8();
                    ocr_app.state::<OcrService>().recognize(image, &directory)
                })
                .await
                .map_err(|e| e.to_string())??;
                if !pipeline.is_current(progress.request_id) {
                    return Ok(());
                }
                progress.ocr_ms = Some(start.elapsed().as_millis());
                progress.source_text = source;
                progress.phase = "translating";
                pipeline.publish(&app, &progress);
            }
            let ollama = app.state::<OllamaService>();
            let translation_guard = ollama.gate.lock().await;
            if !pipeline.is_current(progress.request_id) {
                return Ok(());
            }
            // Register before checking the revision again to avoid losing a cancellation
            // between the state check and the first poll of the HTTP request.
            let cancelled = pipeline.cancelled.notified();
            tokio::pin!(cancelled);
            cancelled.as_mut().enable();
            if !pipeline.is_current(progress.request_id) {
                return Ok(());
            }
            let start = Instant::now();
            let translation = tokio::select! {
                result = ollama.translate(&progress.source_text, &progress.model) => result?,
                _ = &mut cancelled => return Ok(()),
            };
            progress.translation = translation;
            progress.translation_ms = Some(start.elapsed().as_millis());
            drop(translation_guard);
            // Storage failure is reported separately: the translated text remains usable.
            let save_app = app.clone();
            let save_progress = progress.clone();
            let saved = tauri::async_runtime::spawn_blocking(move || {
                if !save_app
                    .state::<Pipeline>()
                    .is_current(save_progress.request_id)
                {
                    return Ok(None);
                }
                save_app
                    .state::<crate::history::HistoryStore>()
                    .record(
                        &save_progress.source_text,
                        &save_progress.translation,
                        &save_progress.model,
                    )
                    .map(Some)
            })
            .await
            .map_err(|e| e.to_string())
            .and_then(|value| value);
            match saved {
                Ok(Some(saved)) => {
                    progress.history_id = Some(saved.id);
                    progress.selected_for_study = saved.selected_for_study;
                    let _ = app.emit("history-changed", ());
                }
                Ok(None) => return Ok(()),
                Err(error) => progress.storage_error = Some(error),
            }
            progress.phase = "done";
            pipeline.publish(&app, &progress);
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            progress.phase = "error";
            progress.error = Some(error);
            pipeline.publish(&app, &progress);
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_and_superseded_requests_are_not_current() {
        let pipeline = Pipeline::default();
        let old = pipeline.start(1, None, "model".into());
        let new = pipeline.start(2, None, "model".into());
        assert!(!pipeline.is_current(old.request_id));
        assert!(!pipeline.update(&old));
        assert_eq!(pipeline.current().unwrap().unwrap().selection_id, 2);
        assert!(pipeline.is_current(new.request_id));
        pipeline.cancel();
        assert!(!pipeline.is_current(new.request_id));
    }
}
