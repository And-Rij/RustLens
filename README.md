# RustLens

A local English-to-Ukrainian screen translator with translation history and Anki export.

Select text on the screen, translate it in a nearby window, and save useful phrases for review. Text recognition runs on the CPU with Ocrs; translation runs through a local Ollama instance.

## Features

- Screen region selection with `Ctrl + Shift + L`.
- Editable recognized text and local translation.
- Selection of installed Ollama models.
- Searchable SQLite history and study bookmarks.
- UTF-8 text export for Anki.
- Light and dark appearance based on system settings.

## Requirements

Windows is the currently supported platform. To build from source, install:

- Rust `1.96.0` with the MSVC toolchain, Cargo, rustfmt, and Clippy.
- Microsoft C++ Build Tools with **Desktop development with C++** and Windows SDK.
- WebView2 Runtime. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
- Node.js `24.x` and pnpm `12.11.2`.
- [Ollama](https://ollama.com/) with a translation model. The default is `translategemma:12b`.
- [Anki](https://apps.ankiweb.net/) to import exported phrases.

## Development

```powershell
git clone https://github.com/And-Rij/RustLens.git
cd RustLens
pnpm install --frozen-lockfile
pnpm models:download
```

Install the default translation model if it is not already available:

```powershell
ollama pull translategemma:12b
```

Keep Ollama running, then start RustLens:

```powershell
pnpm tauri dev
```

The OCR download script checks SHA-256 hashes against `ocr-models.json` and reuses verified files. OCR weights and Ollama models are separate downloads. Once dependencies and models are installed, translation works offline.

## Usage

1. Move the pointer to the monitor containing the text.
2. Press `Ctrl + Shift + L` and drag a rectangle around the text.
3. Read the translation in the result window. Correct the English text and translate again when needed.
4. Mark phrases with **Для навчання** and open **Історія** to search or export them.

Press `Esc` to dismiss selection or the result window. In the selection overlay, `Enter` selects the full captured screen when the selection surface has focus. Closing the main window exits the application.

The interface is in Ukrainian. OCR and translation can make mistakes. Capturing a frame does not pause the application being captured, and the result window may take focus. Model selection lasts until RustLens restarts.

## Anki export

In **Історія**, choose **Експорт в Anki** to save all study bookmarks. Import the resulting text file into Anki with a **Basic** note type and the desired deck.

| Field | Content               |
| ----- | --------------------- |
| Front | English text          |
| Back  | Ukrainian translation |
| Tags  | `rustlens`            |

The file includes Tab and HTML import headers. For subsequent imports, keep the same note type, deck, and duplicate matching settings; choose to update or skip existing notes. AnkiConnect is not required for file export.

## Local data

Translation requests go to `http://127.0.0.1:11434`. Screenshots stay in memory and are not sent to Ollama or saved in history.

History is stored at `%APPDATA%\com.andrij.rust-lens\history.sqlite`. To back it up, close RustLens and copy the containing folder. Export files are saved to the chosen destination.

## Code structure

| Location                                     | Responsibility                                   |
| -------------------------------------------- | ------------------------------------------------ |
| `src/views/`                                 | Main window, screen selection, and result window |
| `src/hooks/`                                 | Native event subscriptions                       |
| `src/*-api.ts`                               | Typed Tauri commands and event payloads          |
| `src/TranslationPanel.tsx`                   | Translation input, progress, and study bookmark  |
| `src/HistoryPanel.tsx`                       | Search, pagination, bookmarks, and export        |
| `src-tauri/src/lib.rs`                       | Application setup and command registration       |
| `src-tauri/src/commands.rs`                  | Tauri command handlers                           |
| `src-tauri/src/windows.rs`                   | Capture and result window configuration          |
| `src-tauri/src/capture.rs`, `geometry.rs`    | Screenshot capture, cropping, and positioning    |
| `src-tauri/src/pipeline.rs`                  | OCR, translation, cancellation, and persistence  |
| `src-tauri/src/ocr.rs`, `ollama.rs`          | OCR engine and local translation client          |
| `src-tauri/src/history.rs`, `anki_export.rs` | SQLite storage and Anki text export              |

## Checks

```powershell
pnpm format:check
pnpm test:frontend
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

Use `pnpm format` and `cargo fmt --manifest-path src-tauri/Cargo.toml` to format source files. Windows CI runs the checks and the OCR integration test. Tests requiring local Ollama are ignored by default.

## Windows build

```powershell
pnpm models:download
node scripts/collect-notices.mjs
pnpm tauri build --bundles nsis
```

The installer is written to `src-tauri/target/release/bundle/nsis/`. It includes OCR models and license notices. Ollama and its translation model must be installed separately.

## License

RustLens is licensed under [MIT](LICENSE). Dependencies and model weights retain their own licenses; see [third-party notices](THIRD_PARTY.md).
