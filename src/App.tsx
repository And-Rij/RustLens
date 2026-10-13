import { useEffect, useRef, useState, type PointerEvent } from "react";
import { Button } from "./components/ui/button";
import { dismissCapture, errorMessage, getFrame, getSelection, getStatus, nativeAvailable, onCapture, onCaptureError, onSelection, showCapture, startCapture, submitSelection, type Frame, type Selection, type Status } from "./capture-api";
import { ModelSettings, TranslationPanel } from "./TranslationPanel";
import { HistoryPanel } from "./HistoryPanel";
import "./App.css";

function useSelection() {
  const [selection, setSelection] = useState<Selection | null>(null);
  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    let receivedEvent = false;
    void (async () => {
      const stop = await onSelection(value => { receivedEvent = true; if (!disposed) setSelection(value); });
      if (disposed) { stop(); return; }
      cleanup = stop;
      const value = await getSelection();
      if (!disposed && !receivedEvent) setSelection(value);
    })().catch(console.error);
    return () => { disposed = true; cleanup?.(); };
  }, []);
  return selection;
}

function Fragment({ selection }: { selection: Selection }) {
  return <figure className="fragment"><img src={selection.imageUrl} alt="Вибрана область екрана" /><figcaption>{selection.width} × {selection.height} px</figcaption></figure>;
}

function MainView() {
  const [view,setView]=useState<"translate"|"history">("translate");
  const selection = useSelection();
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    void getStatus().then(value => { if (!disposed) setStatus(value); }).catch(value => { if (!disposed) setError(errorMessage(value)); });
    void onCaptureError(value => { if (!disposed) setError(value); }).then(stop => { if (disposed) stop(); else cleanup = stop; });
    return () => { disposed = true; cleanup?.(); };
  }, []);
  async function start() {
    setError(""); setBusy(true);
    try { await startCapture(); } catch (value) { setError(errorMessage(value)); }
    finally { setBusy(false); }
  }
  return <main className="app-shell">
    <header><h1>RustLens</h1><p className="muted">Англійська у грі, практика після неї.</p></header>
    <nav className="main-navigation" aria-label="Розділи RustLens"><Button variant={view==="translate"?"default":"outline"} aria-pressed={view==="translate"} onClick={()=>setView("translate")}>Переклад</Button><Button variant={view==="history"?"default":"outline"} aria-pressed={view==="history"} onClick={()=>setView("history")}>Історія</Button></nav>
    <div hidden={view!=="translate"}>
    <section aria-labelledby="capture-heading" className="capture-section">
      <h2 id="capture-heading">Вибери текст на екрані</h2>
      <p>Відкрий Limbus Company, наведи курсор на монітор із грою та натисни <kbd>{status?.shortcut ?? "Ctrl + Shift + L"}</kbd>. Обведи потрібний фрагмент мишею.</p>
      <Button onClick={() => void start()} disabled={busy || !nativeAvailable}>{busy ? "Захоплюємо екран…" : "Виділити фрагмент"}</Button>
      <p className="muted help">Esc — скасувати. Enter — вибрати весь захоплений екран.</p>
      {!nativeAvailable && <p className="notice">Захоплення доступне у desktop-вікні RustLens. Запусти програму через pnpm tauri dev.</p>}
      {status?.shortcutError && <p role="alert" className="error">{status.shortcutError}</p>}
      {error && <p role="alert" className="error">{error}</p>}
    </section>
    <section aria-labelledby="preview-heading" className="preview-section">
      <h2 id="preview-heading">Останній фрагмент</h2>
      {selection ? <><details><summary>Фрагмент екрана</summary><Fragment selection={selection} /></details><TranslationPanel key={selection.id} selection={selection} /></> : <p className="muted empty">Після виділення тут з’явиться фрагмент. Перевір, чи в нього потрапив увесь потрібний текст.</p>}
    </section>
    <ModelSettings />
    </div>
    {view==="history"&&<HistoryPanel />}
    <footer className="muted">Переклади й історія зберігаються локально. Експорт в Anki — у розділі «Історія».</footer>
  </main>;
}

type Point = { x: number; y: number };
type Drag = { start: Point; end: Point };
function SelectionView() {
  const [frame, setFrame] = useState<Frame | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const surface = useRef<HTMLDivElement>(null);
  const submitting = useRef(false);
  const dragRef = useRef<Drag | null>(null);
  useEffect(() => {
    let disposed = false;
    let cleanup: (() => void) | undefined;
    async function refresh() {
      const next = await getFrame();
      if (!disposed) { setFrame(next); setDrag(null); dragRef.current = null; setError(""); setBusy(false); submitting.current = false; }
    }
    void (async () => {
      const stop = await onCapture(() => { void refresh().catch(value => { if (!disposed) setError(errorMessage(value)); }); });
      if (disposed) { stop(); return; }
      cleanup = stop;
      await refresh();
    })().catch(value => { if (!disposed) setError(errorMessage(value)); });
    return () => { disposed = true; cleanup?.(); };
  }, []);
  async function cancel() {
    try { await dismissCapture(); setFrame(null); setDrag(null); dragRef.current = null; }
    catch (value) { setError(errorMessage(value)); }
  }
  async function submit(rect: { x: number; y: number; width: number; height: number }) {
    if (!frame || submitting.current) return;
    submitting.current = true; setBusy(true); setError("");
    try { await submitSelection(frame.id, rect); setFrame(null); setDrag(null); dragRef.current = null; }
    catch (value) { setError(errorMessage(value)); }
    finally { submitting.current = false; setBusy(false); }
  }
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); void cancel(); }
      if (event.key === "Enter" && frame && !submitting.current && event.target === document.body) {
        event.preventDefault(); void submit({ x: 0, y: 0, width: frame.width, height: frame.height });
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });
  function point(event: PointerEvent<HTMLDivElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    return { x: Math.max(0, Math.min(bounds.width, event.clientX - bounds.left)), y: Math.max(0, Math.min(bounds.height, event.clientY - bounds.top)) };
  }
  function down(event: PointerEvent<HTMLDivElement>) {
    if (!frame || busy || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const p = point(event); const next = { start: p, end: p };
    dragRef.current = next; setDrag(next); setError("");
  }
  function move(event: PointerEvent<HTMLDivElement>) {
    if (!dragRef.current || busy) return;
    const next = { start: dragRef.current.start, end: point(event) };
    dragRef.current = next; setDrag(next);
  }
  function up(event: PointerEvent<HTMLDivElement>) {
    const current = dragRef.current;
    if (!current || !frame || busy) return;
    const end = point(event);
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = Math.floor(Math.min(current.start.x, end.x) * frame.width / bounds.width);
    const y = Math.floor(Math.min(current.start.y, end.y) * frame.height / bounds.height);
    const right = Math.min(frame.width, Math.ceil(Math.max(current.start.x, end.x) * frame.width / bounds.width));
    const bottom = Math.min(frame.height, Math.ceil(Math.max(current.start.y, end.y) * frame.height / bounds.height));
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    void submit({ x, y, width: right - x, height: bottom - y });
  }
  const box = drag ? { left: Math.min(drag.start.x, drag.end.x), top: Math.min(drag.start.y, drag.end.y), width: Math.abs(drag.start.x - drag.end.x), height: Math.abs(drag.start.y - drag.end.y) } : null;
  return <div ref={surface} className="selection-surface" onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={() => { dragRef.current = null; setDrag(null); }}>
    {frame && <img className="snapshot" src={frame.imageUrl} alt="Захоплений екран для вибору області" draggable={false} onLoad={() => { void showCapture(frame.id).catch(value => setError(errorMessage(value))); }} />}
    <div className="selection-shade" />
    {box && <div className="selection-box" style={box} />}
    <div className="selection-help" onPointerDown={event => event.stopPropagation()}>
      <p>{busy ? "Готуємо фрагмент…" : "Обведи текст мишею"}</p>
      <p>Esc — скасувати · Enter — весь екран</p>
      <Button variant="outline" size="sm" onClick={() => void cancel()}>Скасувати</Button>
      {error && <p role="alert" className="error">{error}</p>}
    </div>
  </div>;
}

function ResultView() {
  const selection = useSelection();
  const [error, setError] = useState("");
  const close = async () => { try { await dismissCapture(); } catch (value) { setError(errorMessage(value)); } };
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => { if (event.key === "Escape") { event.preventDefault(); void close(); } };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });
  return <main className="result-shell"><h1>Переклад</h1>{selection ? <><TranslationPanel key={selection.id} selection={selection} /><details><summary>Фрагмент екрана</summary><Fragment selection={selection} /></details></> : <p className="muted">Очікуємо фрагмент…</p>}<Button className="close-result" variant="outline" size="sm" onClick={() => void close()}>Закрити · Esc</Button>{error && <p role="alert" className="error">{error}</p>}</main>;
}

export default function App() {
  const view = new URLSearchParams(window.location.search).get("view");
  return view === "selection" ? <SelectionView /> : view === "result" ? <ResultView /> : <MainView />;
}
