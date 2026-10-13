# RustLens

Локальний перекладач English → Ukrainian для вивчення англійської під час гри. Виділи фрагмент екрана, отримай переклад поруч і збережи потрібні фрази для повторення в Anki.

**MVP працює на Windows:** захоплення → OCR → Ollama → SQLite-історія → файловий експорт в Anki. Користувач підтвердив переклад у повноекранній Limbus Company та появу матеріалу у власній Anki.

## Можливості

- `Ctrl + Shift + L`: захоплення монітора під курсором і виділення прямокутником.
- Невелике вікно з англійським текстом та українським перекладом біля вибраної області.
- Редагування помилок OCR, повторний переклад, скасування запиту й вибір встановленої моделі.
- Постійна історія з пошуком, сторінками по 20 записів і позначенням «Для навчання».
- Експорт позначених фраз у UTF-8 TSV-файл для Anki: English / Ukrainian / тег `rustlens`.
- Світла й темна тема за налаштуванням ОС.

Захоплений кадр зберігається лише в пам'яті сеансу. Успішні переклади записуються у локальну SQLite. Текст надсилається лише локальній Ollama за `http://127.0.0.1:11434`; скриншот залишається в застосунку. Гра під час виділення продовжує працювати.

## Запуск з коду на Windows

Потрібні:

- Rust MSVC toolchain з Cargo, rustfmt і Clippy. Перевірено Rust `1.96.0`.
- Microsoft C++ Build Tools з **Desktop development with C++** і Windows SDK.
- WebView2 Runtime. Докладні [передумови Tauri для Windows](https://v2.tauri.app/start/prerequisites/).
- Node.js `24.x` і pnpm `12.11.2` (версію зафіксовано в `package.json`).
- Запущена [Ollama](https://ollama.com/) з локальною [TranslateGemma](https://ollama.com/library/translategemma). Типова модель — `translategemma:12b`.
- Anki для імпорту карток; AnkiConnect для цього способу не потрібний.

Отримай код, потім виконай команди в його папці:

```powershell
git clone https://github.com/And-Rij/RustLens.git
cd RustLens
pnpm install --frozen-lockfile
pnpm models:download
ollama list
# Якщо TranslateGemma ще немає: завантаження моделі потребує інтернету й місця на диску.
ollama pull translategemma:12b
pnpm tauri dev
```

`models:download` завантажує дві OCR-моделі (~12 MB), перевіряє SHA-256 за `ocr-models.json` і повторно використовує вже перевірені файли. Ваги Ollama завантажуються окремо. Після початкового встановлення й завантаження моделей переклад працює локально.

Для першої перевірки відкрий гру англійською, наведи курсор на її монітор, натисни `Ctrl + Shift + L` та обведи текст. OCR і переклад запустяться автоматично. `Esc` скасовує виділення; `Enter` вибирає весь кадр, якщо фокус на поверхні виділення. Закриття головного вікна завершує RustLens.

## Збереження в Anki

1. Познач корисну фразу кнопкою «Для навчання» або прапорцем в історії.
2. В історії натисни «Експорт в Anki» й збережи файл.
3. В Anki імпортуй його, обери тип нотатки **Basic** або локалізований відповідник і свою колоду.
4. Перевір поля: **Front** — англійський текст, **Back** — український переклад, **Tags** — теги. Файл задає Tab і HTML у заголовках.

Для повторного імпорту використовуй той самий тип нотатки, колоду та зіставлення дублікатів; обирай оновлення або пропуск існуючих нотаток. [Подробиці та перевірки імпорту](docs/BLOCK_3.md).

Історія розміщена у `%APPDATA%\com.andrij.rust-lens\history.sqlite`; поточний шлях показано в інтерфейсі. Для резервної копії закрий RustLens і скопіюй усю цю папку. Скриншоти в базі не зберігаються.

## Перевірки та збірка

```powershell
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
# Додатково: локальні OCR-моделі; Ollama не потрібна.
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib recognizes_english_fixture -- --ignored
# Додатково: запущена Ollama з типовою моделлю.
cargo test --manifest-path src-tauri/Cargo.toml --locked --lib translates_using_local_ollama -- --ignored
```

У звичайному наборі 13 тестів; 3 додаткові тести потребують локальних ресурсів або створюють fixture. Перевірено збереження SQLite після повторного відкриття, пошук, позначки, TSV/HTML та реальний повторний імпорт у тимчасову колекцію Anki без дублікатів. [CI](.github/workflows/checks.yml) запускає Windows checks і OCR-тест без Ollama; його перший запуск на GitHub ще очікується.

Локальна release-збірка NSIS-інсталятора перевірена на цьому Windows-комп'ютері. Запуск після встановлення ще не перевірено. Повторна збірка:

```powershell
pnpm models:download
node scripts/collect-notices.mjs
pnpm tauri build --bundles nsis
```

Файли збірки: `src-tauri/target/release/`; інсталятор: `src-tauri/target/release/bundle/nsis/`. OCR-ресурси включає Tauri; Ollama та її модель встановлюються окремо. Локально зібраний інсталятор не підписаний.

## Перевірене середовище й обмеження

- Windows, Limbus Company у повноекранному режимі: користувач підтвердив захоплення й переклад. Точний exclusive/borderless режим не встановлено.
- На Ryzen 7500F, RTX 5060 Ti 16 GB, 32 GB RAM повідомлений час перекладу — **0,9–3 с**. Це спостереження користувача; холодний старт, VRAM і вплив на FPS окремо не виміряні.
- Інші ігри, HDR, різний DPI і кілька моніторів ще не перевірені. Вікно результату може змінювати фокус гри.
- OCR і переклад можуть помилятися; текст можна виправити перед повторним перекладом. Вибір моделі діє в поточному сеансі.
- Linux, пряме додавання через AnkiConnect, Obsidian, пояснення, вимова й мініігри — [подальші етапи](docs/ROADMAP.md).

## Стек і документація

Tauri 2 / Rust; React / TypeScript / Vite; Tailwind CSS 4 і Button на основі shadcn/ui Radix. Захоплення — XCap; OCR — ocrs/RTen на CPU; переклад — Ollama; історія — SQLite/rusqlite.

- [Архітектура](docs/ARCHITECTURE.md) і [вимоги](PRODUCT.md).
- [Етапи реалізації](docs/ROADMAP.md), [перевірки гри](docs/GAME_TESTS.md), [локальне середовище](docs/ENVIRONMENT.md).
- Розбір блоків: [захоплення](docs/BLOCK_1.md), [OCR/переклад](docs/BLOCK_2.md), [історія/Anki](docs/BLOCK_3.md).
- [Підготовка репозиторію GitHub](docs/GITHUB.md).

Код RustLens — [MIT](LICENSE). Сторонні бібліотеки й моделі мають власні ліцензії; [джерела та відомості](THIRD_PARTY.md).
