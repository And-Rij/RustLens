//! Configuration for the two persistent auxiliary windows.
//!
//! They are created hidden and reused for each capture. Their query parameters
//! select the corresponding React view from the same bundled frontend.

use tauri::{WebviewUrl, WebviewWindowBuilder};

pub fn create_capture_windows(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(
        app,
        "selection",
        WebviewUrl::App("index.html?view=selection".into()),
    )
    .title("Виділення — RustLens")
    .visible(false)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .build()?;

    WebviewWindowBuilder::new(
        app,
        "result",
        WebviewUrl::App("index.html?view=result".into()),
    )
    .title("Переклад — RustLens")
    .visible(false)
    .inner_size(480.0, 620.0)
    .min_inner_size(280.0, 220.0)
    .always_on_top(true)
    .skip_taskbar(true)
    .build()?;

    Ok(())
}
