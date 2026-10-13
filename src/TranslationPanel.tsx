import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./components/ui/button";
import { errorMessage, nativeAvailable, type Selection } from "./capture-api";

interface Progress { selectionId: number; requestId: number; phase: "recognizing" | "translating" | "done" | "error"; sourceText: string; translation: string; error: string | null; model: string; ocrMs: number | null; translationMs: number | null; historyId:number|null; selectedForStudy:boolean; storageError:string|null }

export function TranslationPanel({ selection }: { selection: Selection }) {
  const [progress, setProgress] = useState<Progress | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [requesting, setRequesting] = useState(false);
  const [marking,setMarking]=useState(false);
  const revision = useRef(0);
  const edited = useRef(false);
  const selectionId = selection.id;
  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    setProgress(null); setText(""); setError(""); setRequesting(false); edited.current = false; revision.current = 0;
    function update(value: Progress | null) {
      if (!disposed && value?.selectionId === selectionId && value.requestId >= revision.current) {
        revision.current = value.requestId; setProgress(value);
        if (!edited.current && value.sourceText) setText(value.sourceText);
      }
    }
    void (async () => {
      const cleanup = await listen<Progress>("translation-progress", event => update(event.payload));
      if (disposed) { cleanup(); return; } stop = cleanup;
      update(await invoke<Progress | null>("get_translation"));
    })().catch(value => { if (!disposed) setError(errorMessage(value)); });
    return () => { disposed = true; stop?.(); };
  }, [selectionId]);
  async function translate() {
    setError(""); setRequesting(true);
    try { await invoke("translate_text", { selectionId, text }); }
    catch (value) { setError(errorMessage(value)); }
    finally { setRequesting(false); }
  }
  async function mark() {
    if(!progress?.historyId)return;
    setMarking(true);setError("");
    try{await invoke("set_study_flag",{id:progress.historyId,selected:!progress.selectedForStudy});}
    catch(value){setError(errorMessage(value));}finally{setMarking(false);}
  }
  const recognizing = progress?.phase === "recognizing";
  const translating = progress?.phase === "translating";
  const outdated = !!progress?.sourceText && text.trim() !== progress.sourceText.trim();
  return <section className="translation-panel" aria-label="Переклад фрагмента">
    <label htmlFor="source-text">Англійський текст</label>
    <textarea id="source-text" value={text} onChange={event => { edited.current = true; setText(event.target.value); }} placeholder={recognizing ? "Розпізнаємо текст…" : "Введи або виправ текст"} rows={3} maxLength={4000} disabled={recognizing} />
    <div className="translation-actions"><Button size="sm" onClick={() => void translate()} disabled={!text.trim() || recognizing || requesting || translating}>{requesting || translating ? "Перекладаємо…" : "Перекласти"}</Button>{(recognizing || translating) && <Button variant="ghost" size="sm" onClick={() => void invoke("cancel_translation").catch(value => setError(errorMessage(value)))}>Скасувати запит</Button>}</div>
    {recognizing && <p role="status" className="muted">Розпізнаємо англійський текст…</p>}
    {translating && <p role="status" className="muted">Перекладаємо локально. Перший запит може потребувати завантаження моделі.</p>}
    <h2>Український переклад</h2>
    {progress?.translation ? <><p className="translated-text" aria-live="polite">{progress.translation}</p>{outdated && <p className="muted">Текст змінено. Натисни «Перекласти», щоб оновити результат.</p>}</> : <p className="muted">{translating ? "Очікуємо відповідь моделі…" : "Тут з’явиться переклад."}</p>}
    {(error || progress?.error) && <p role="alert" className="error">{error || progress?.error}</p>}
    {progress?.phase === "done" && progress.historyId && <Button variant="outline" size="sm" aria-pressed={progress.selectedForStudy} disabled={marking||outdated} onClick={()=>void mark()}>{marking?"Зберігаємо…":progress.selectedForStudy?"Прибрати з навчання":"Для навчання"}</Button>}
    {progress?.storageError && <p role="alert" className="error">Переклад отримано, але історію не збережено: {progress.storageError}</p>}
    {progress?.phase === "done" && <p className="muted timing">{progress.model} · {progress.ocrMs !== null && `OCR ${(progress.ocrMs / 1000).toFixed(1)} с · `}переклад {((progress.translationMs ?? 0) / 1000).toFixed(1)} с</p>}
  </section>;
}

export function ModelSettings() {
  const [models, setModels] = useState<string[]>([]);
  const [selected, setSelected] = useState("translategemma:12b");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  async function refresh() {
    setError(""); setLoading(true);
    try { const response = await invoke<{ models: string[]; selected: string }>("get_models"); setModels(response.models); setSelected(response.selected); }
    catch (value) { setError(errorMessage(value)); }
    finally { setLoading(false); }
  }
  useEffect(() => { if (nativeAvailable) void refresh(); }, []);
  async function change(model: string) {
    setError(""); setLoading(true);
    try { await invoke("select_model", { model }); setSelected(model); }
    catch (value) { setError(errorMessage(value)); }
    finally { setLoading(false); }
  }
  return <section className="model-settings" aria-label="Модель перекладу"><label htmlFor="translation-model">Локальна модель</label><div className="model-controls"><select id="translation-model" value={selected} onChange={event => void change(event.target.value)} disabled={loading || !models.length}>{!models.includes(selected) && <option value={selected}>{selected}{models.length ? " — не встановлена" : ""}</option>}{models.map(model => <option key={model} value={model}>{model}</option>)}</select><Button variant="outline" size="sm" disabled={loading || !nativeAvailable} onClick={() => void refresh()}>{loading ? "Перевіряємо…" : "Оновити список"}</Button></div>{error && <p role="alert" className="error">{error}</p>}<p className="muted help">Ollama має бути запущена. Вибір моделі діє до перезапуску RustLens.</p></section>;
}
