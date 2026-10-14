//! Application entry point: services, windows, shortcuts, and command registration.

mod anki_export;
mod capture;
mod commands;
mod geometry;
mod history;
mod ocr;
mod ollama;
mod pipeline;
mod windows;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::CaptureState::default())
        .manage(ocr::OcrService::default())
        .manage(ollama::OllamaService::default())
        .manage(pipeline::Pipeline::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = commands::begin_capture(&app).await {
                                let _ = app.emit("capture-error", error);
                            }
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Failed storage initialization should not block capture or translation.
            let storage = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())
                .and_then(|path| history::HistoryStore::open(&path.join("history.sqlite")));
            app.manage(storage.unwrap_or_else(history::HistoryStore::unavailable));
            windows::create_capture_windows(app)?;
            commands::register_capture_shortcut(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                window.app_handle().exit(0);
                return;
            }
            // Auxiliary windows are hidden, not destroyed, so their listeners remain active.
            if window.label() != "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let app = window.app_handle();
                let _ =
                    commands::dismiss_capture(app.clone(), app.state::<commands::CaptureState>());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_capture,
            commands::get_capture,
            commands::show_capture,
            commands::submit_selection,
            commands::dismiss_capture,
            commands::get_selection,
            commands::get_translation,
            commands::get_models,
            commands::select_model,
            commands::translate_text,
            commands::cancel_translation,
            commands::query_history,
            commands::set_study_flag,
            anki_export::export_anki,
        ])
        .run(tauri::generate_context!())
        .expect("error while running RustLens");
}
