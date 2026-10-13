mod anki_export;
mod capture;
mod geometry;
mod history;
mod ocr;
mod ollama;
mod pipeline;

use capture::{Frame, FramePreview, SelectionPreview};
use geometry::Rect;
use serde::Serialize;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use tauri::{
    Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(Default)]
struct CaptureState {
    busy: AtomicBool,
    next_id: AtomicU64,
    frame: Mutex<Option<Frame>>,
    selection: Mutex<Option<SelectionPreview>>,
    shortcut_error: Mutex<Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    shortcut: &'static str,
    shortcut_error: Option<String>,
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> String {
    "Не вдалося отримати стан захоплення. Перезапустіть RustLens.".into()
}

#[tauri::command]
fn get_status(state: State<CaptureState>) -> Result<Status, String> {
    Ok(Status {
        shortcut: "Ctrl + Shift + L",
        shortcut_error: state.shortcut_error.lock().map_err(lock_error)?.clone(),
    })
}

#[tauri::command]
fn get_capture(
    window: WebviewWindow,
    state: State<CaptureState>,
) -> Result<Option<FramePreview>, String> {
    if window.label() != "selection" {
        return Err("Кадр доступний лише у вікні виділення.".into());
    }
    Ok(state
        .frame
        .lock()
        .map_err(lock_error)?
        .as_ref()
        .map(|frame| frame.preview.clone()))
}

#[tauri::command]
fn get_selection(state: State<CaptureState>) -> Result<Option<SelectionPreview>, String> {
    Ok(state.selection.lock().map_err(lock_error)?.clone())
}

fn hide_capture_windows(app: &tauri::AppHandle) -> Result<(), String> {
    for label in ["selection", "result"] {
        if let Some(window) = app.get_webview_window(label) {
            window.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn dismiss_capture(app: tauri::AppHandle, state: State<CaptureState>) -> Result<(), String> {
    cancel_translation(app.clone())?;
    hide_capture_windows(&app)?;
    state.next_id.fetch_add(1, Ordering::SeqCst);
    *state.frame.lock().map_err(lock_error)? = None;
    state.busy.store(false, Ordering::SeqCst);
    Ok(())
}

async fn begin_capture(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<CaptureState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Err("Виділення вже відкрите. Завершіть його або натисніть Esc.".into());
    }
    cancel_translation(app.clone())?;
    let id = state.next_id.fetch_add(1, Ordering::SeqCst) + 1;
    let result = async {
        let cursor = app.cursor_position().map_err(|e| e.to_string())?;
        hide_capture_windows(app)?;
        let frame = tauri::async_runtime::spawn_blocking(move || {
            // Give the compositor time to remove the preceding result window.
            std::thread::sleep(std::time::Duration::from_millis(80));
            capture::capture(id, (cursor.x as i32, cursor.y as i32))
        })
        .await
        .map_err(|e| e.to_string())??;
        if state.next_id.load(Ordering::SeqCst) != id {
            return Ok(());
        }
        let position = PhysicalPosition::new(frame.origin.0, frame.origin.1);
        let size = PhysicalSize::new(frame.image.width(), frame.image.height());
        *state.frame.lock().map_err(lock_error)? = Some(frame);
        let window = app
            .get_webview_window("selection")
            .ok_or("Вікно виділення недоступне.")?;
        window.set_position(position).map_err(|e| e.to_string())?;
        window.set_size(size).map_err(|e| e.to_string())?;
        // The frontend shows the window after its snapshot image has loaded.
        window
            .emit("capture-started", ())
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    }
    .await;
    if result.is_err() && state.next_id.load(Ordering::SeqCst) == id {
        state.busy.store(false, Ordering::SeqCst);
        if let Ok(mut frame) = state.frame.lock() {
            *frame = None;
        }
        let _ = hide_capture_windows(app);
    }
    result
}

#[tauri::command]
async fn start_capture(app: tauri::AppHandle) -> Result<(), String> {
    begin_capture(&app).await
}

#[tauri::command]
fn show_capture(
    window: WebviewWindow,
    state: State<CaptureState>,
    frame_id: u64,
) -> Result<(), String> {
    if window.label() != "selection" {
        return Err("Недоступне вікно виділення.".into());
    }
    if state
        .frame
        .lock()
        .map_err(lock_error)?
        .as_ref()
        .is_none_or(|f| f.id != frame_id)
    {
        return Err("Цей кадр уже закрито.".into());
    }
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
async fn submit_selection(
    app: tauri::AppHandle,
    window: WebviewWindow,
    frame_id: u64,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if window.label() != "selection" {
        return Err("Виділення приймається лише з відповідного вікна.".into());
    }
    let rect = Rect {
        x,
        y,
        width,
        height,
    };
    let (preview, origin, size) = {
        let state = app.state::<CaptureState>();
        let frame = state.frame.lock().map_err(lock_error)?;
        let frame = frame
            .as_ref()
            .ok_or("Кадр уже закрито. Почніть виділення знову.")?;
        if frame.id != frame_id {
            return Err("Кадр застарів. Почніть виділення знову.".into());
        }
        rect.validate(frame.image.width(), frame.image.height())?;
        (
            frame.crop(rect)?,
            frame.origin,
            (frame.image.width(), frame.image.height()),
        )
    };
    *app.state::<CaptureState>()
        .selection
        .lock()
        .map_err(lock_error)? = Some(preview.clone());
    let result = app
        .get_webview_window("result")
        .ok_or("Вікно результату недоступне.")?;
    // Move to the captured monitor first, then use the resulting physical window size.
    result
        .set_position(PhysicalPosition::new(origin.0, origin.1))
        .map_err(|e| e.to_string())?;
    let popup = result.outer_size().map_err(|e| e.to_string())?;
    let (px, py) = geometry::popup_position(origin, size, rect, (popup.width, popup.height));
    result
        .set_position(PhysicalPosition::new(px, py))
        .map_err(|e| e.to_string())?;
    window.hide().map_err(|e| e.to_string())?;
    result
        .emit("selection-ready", &preview)
        .map_err(|e| e.to_string())?;
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit("selection-ready", &preview);
    }
    pipeline::launch(app.clone(), preview, None)?;
    result.show().map_err(|e| e.to_string())?;
    result.set_focus().map_err(|e| e.to_string())?;
    let state = app.state::<CaptureState>();
    *state.frame.lock().map_err(lock_error)? = None;
    state.busy.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_translation(state: State<pipeline::Pipeline>) -> Result<Option<pipeline::Progress>, String> {
    state.current()
}

#[tauri::command]
async fn get_models(state: State<'_, ollama::OllamaService>) -> Result<ollama::ModelList, String> {
    state.models().await
}

#[tauri::command]
async fn select_model(
    state: State<'_, ollama::OllamaService>,
    model: String,
) -> Result<(), String> {
    state.select_model(model).await
}

#[tauri::command]
fn cancel_translation(app: tauri::AppHandle) -> Result<(), String> {
    let pipeline = app.state::<pipeline::Pipeline>();
    pipeline.cancel();
    if let Some(progress) = pipeline.current()? {
        let _ = app.emit("translation-progress", progress);
    }
    Ok(())
}

#[tauri::command]
fn translate_text(app: tauri::AppHandle, selection_id: u64, text: String) -> Result<(), String> {
    let text = ollama::validate_text(&text)?.to_string();
    let state = app.state::<CaptureState>();
    if state.busy.load(Ordering::SeqCst) {
        return Err("Спочатку завершіть виділення області.".into());
    }
    let selection = state
        .selection
        .lock()
        .map_err(lock_error)?
        .clone()
        .ok_or("Спочатку виберіть фрагмент.")?;
    if selection.id != selection_id {
        return Err("Фрагмент уже змінено. Використайте поточний текст.".into());
    }
    pipeline::launch(app, selection, Some(text))
}

#[tauri::command]
async fn query_history(
    app: tauri::AppHandle,
    page: u32,
    search: String,
    only_study: bool,
) -> Result<history::HistoryPage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<history::HistoryStore>()
            .query(page, &search, only_study)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_study_flag(app: tauri::AppHandle, id: i64, selected: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<history::HistoryStore>()
            .set_study(id, selected)?;
        if let Some(progress) = app.state::<pipeline::Pipeline>().mark_study(id, selected)? {
            let _ = app.emit("translation-progress", progress);
        }
        let _ = app.emit("history-changed", ());
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CaptureState::default())
        .manage(ocr::OcrService::default())
        .manage(ollama::OllamaService::default())
        .manage(pipeline::Pipeline::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _, event| {
            if event.state() == ShortcutState::Pressed {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = begin_capture(&app).await { let _ = app.emit("capture-error", error); }
                });
            }
        }).build())
        .setup(|app| {
            let storage = app.path().app_data_dir().map_err(|e|e.to_string()).and_then(|path|history::HistoryStore::open(&path.join("history.sqlite")));
            app.manage(storage.unwrap_or_else(history::HistoryStore::unavailable));
            WebviewWindowBuilder::new(app, "selection", WebviewUrl::App("index.html?view=selection".into()))
                .title("Виділення — RustLens").visible(false).decorations(false).resizable(false)
                .always_on_top(true).skip_taskbar(true).shadow(false).build()?;
            WebviewWindowBuilder::new(app, "result", WebviewUrl::App("index.html?view=result".into()))
                .title("Переклад — RustLens").visible(false).inner_size(480.0, 620.0)
                .min_inner_size(280.0, 220.0).always_on_top(true).skip_taskbar(true).build()?;
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyL);
            if let Err(error) = app.global_shortcut().register(shortcut) {
                *app.state::<CaptureState>().shortcut_error.lock().map_err(lock_error)? = Some(format!("Не вдалося зареєструвати Ctrl + Shift + L: {error}. Користуйтеся кнопкою або закрийте програму з такою самою комбінацією."));
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                window.app_handle().exit(0);
                return;
            }
            if window.label() != "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let app = window.app_handle();
                let _ = dismiss_capture(app.clone(), app.state::<CaptureState>());
            }
        })
        .invoke_handler(tauri::generate_handler![get_status, start_capture, get_capture, show_capture, submit_selection, dismiss_capture, get_selection, get_translation, get_models, select_model, translate_text, cancel_translation, query_history, set_study_flag, anki_export::export_anki])
        .run(tauri::generate_context!())
        .expect("error while running RustLens");
}
