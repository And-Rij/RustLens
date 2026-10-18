# Сторонні компоненти та моделі

MIT у `LICENSE` поширюється на код RustLens. Залежності, OCR-ваги та зовнішні моделі зберігають власні умови. Точні версії залежностей зафіксовані в `src-tauri/Cargo.lock` та `pnpm-lock.yaml`.

## Основні бібліотеки

| Компонент | Джерело / ліцензія |
| --- | --- |
| Tauri та plugins | [tauri-apps](https://github.com/tauri-apps/tauri), MIT / Apache-2.0 |
| React | [facebook/react](https://github.com/facebook/react), MIT |
| Tailwind CSS | [tailwindlabs/tailwindcss](https://github.com/tailwindlabs/tailwindcss), MIT |
| shadcn/ui, Radix | [shadcn/ui](https://github.com/shadcn-ui/ui), [radix-ui/primitives](https://github.com/radix-ui/primitives), MIT |
| XCap | [nashaofu/xcap](https://github.com/nashaofu/xcap), Apache-2.0 |
| ocrs і RTen | [robertknight/ocrs](https://github.com/robertknight/ocrs), [robertknight/rten](https://github.com/robertknight/rten), MIT / Apache-2.0 |
| rusqlite | [rusqlite/rusqlite](https://github.com/rusqlite/rusqlite), MIT; SQLite — [public domain](https://www.sqlite.org/copyright.html) |
| image, reqwest, csv, serde, tokio, tempfile та інші crates | Ліцензії у відповідних Cargo package manifests і вихідних пакетах |

Це огляд, а не повний перелік транзитивних залежностей. Перед поширенням готових бінарних пакетів додавай тексти ліцензій і notices фактично включених залежностей. Згенерований комплект міститься у `src-tauri/licenses/DEPENDENCIES.txt`; оновлення — `node scripts/collect-notices.mjs` після встановлення залежностей.

## OCR-ресурси

Автор — **Robert Knight / Ocrs project contributors**. Джерела: [ocrs-models](https://github.com/robertknight/ocrs-models) та [картка моделей Ocrs](https://huggingface.co/robertknight/ocrs). Картка моделей зазначає [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Це окрема ліцензія ваг, відмінна від MIT / Apache-2.0 бібліотеки ocrs.

RustLens завантажує незмінені ONNX-файли з офіційного S3 джерела Ocrs. Імена, адреси та SHA-256 — у `ocr-models.json`. Повідомлення авторства та посилання на ліцензію включено у ресурси інсталятора (`src-tauri/licenses/OCR-MODELS.txt`). Ваги не зберігаються в Git. Навчальні дані upstream: HierText та синтетичні дані, див. картку моделі.

## Переклад

[Ollama](https://github.com/ollama/ollama) працює як окрема програма. Ваги TranslateGemma / Gemma не входять до репозиторію або інсталятора RustLens і завантажуються користувачем окремо. Для них застосовуються [Gemma Terms of Use](https://ai.google.dev/gemma/terms) і відомості відповідної [моделі Ollama](https://ollama.com/library/translategemma).

## Тестові матеріали

`src-tauri/tests/fixtures/english-dialogue.png` — синтетичний англійський текст для OCR-тесту; `anki-import.txt` — тестові фрази для імпорту. У цих fixtures немає скриншотів Limbus Company або даних користувацької колекції Anki.
