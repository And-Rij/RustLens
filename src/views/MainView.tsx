import { useEffect, useState } from "react";
import { Button } from "../components/ui/button";
import {
  errorMessage,
  getStatus,
  nativeAvailable,
  onCaptureError,
  startCapture,
  type Status,
} from "../capture-api";
import { useSelection } from "../hooks/useSelection";
import { CaptureFragment } from "../components/CaptureFragment";
import { ModelSettings } from "../ModelSettings";
import { TranslationPanel } from "../TranslationPanel";
import { HistoryPanel } from "../HistoryPanel";

export function MainView() {
  const [view, setView] = useState<"translate" | "history">("translate");
  const selection = useSelection();
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    void getStatus()
      .then((value) => {
        if (!disposed) setStatus(value);
      })
      .catch((value) => {
        if (!disposed) setError(errorMessage(value));
      });
    void onCaptureError((value) => {
      if (!disposed) setError(value);
    }).then((stop) => {
      if (disposed) stop();
      else cleanup = stop;
    });
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, []);

  async function handleCapture() {
    setError("");
    setBusy(true);
    try {
      await startCapture();
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="app-shell">
      <header>
        <h1>RustLens</h1>
      </header>
      <nav className="main-navigation" aria-label="Розділи RustLens">
        <Button
          variant={view === "translate" ? "default" : "outline"}
          aria-pressed={view === "translate"}
          onClick={() => setView("translate")}
        >
          Переклад
        </Button>
        <Button
          variant={view === "history" ? "default" : "outline"}
          aria-pressed={view === "history"}
          onClick={() => setView("history")}
        >
          Історія
        </Button>
      </nav>
      <div hidden={view !== "translate"}>
        <section aria-labelledby="capture-heading" className="capture-section">
          <h2 id="capture-heading">Вибери текст на екрані</h2>
          <p>
            Наведи курсор на потрібний монітор і натисни{" "}
            <kbd>{status?.shortcut ?? "Ctrl + Shift + L"}</kbd>. Обведи текст мишею.
          </p>
          <Button onClick={() => void handleCapture()} disabled={busy || !nativeAvailable}>
            {busy ? "Захоплюємо екран…" : "Виділити фрагмент"}
          </Button>
          <p className="muted help">Esc — скасувати. Enter — вибрати весь захоплений екран.</p>
          {!nativeAvailable && (
            <p className="notice">Відкрий настільну програму RustLens, щоб захоплювати екран.</p>
          )}
          {status?.shortcutError && (
            <p role="alert" className="error">
              {status.shortcutError}
            </p>
          )}
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}
        </section>
        <section aria-labelledby="preview-heading" className="preview-section">
          <h2 id="preview-heading">Останній переклад</h2>
          {selection ? (
            <>
              <details>
                <summary>Фрагмент екрана</summary>
                <CaptureFragment selection={selection} />
              </details>
              <TranslationPanel key={selection.id} selection={selection} />
            </>
          ) : (
            <p className="muted empty">Виділи текст на екрані, щоб отримати переклад.</p>
          )}
        </section>
        <ModelSettings />
      </div>
      {view === "history" && <HistoryPanel />}
    </main>
  );
}
