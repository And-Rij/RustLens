import { useEffect, useState } from "react";
import { Button } from "../components/ui/button";
import { dismissCapture, errorMessage } from "../capture-api";
import { useSelection } from "../hooks/useSelection";
import { CaptureFragment } from "../components/CaptureFragment";
import { TranslationPanel } from "../TranslationPanel";

export function ResultView() {
  const selection = useSelection();
  const [error, setError] = useState("");
  const close = async () => {
    try {
      await dismissCapture();
    } catch (value) {
      setError(errorMessage(value));
    }
  };

  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void close();
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });

  return (
    <main className="result-shell">
      <h1>Переклад</h1>
      {selection ? (
        <>
          <TranslationPanel key={selection.id} selection={selection} />
          <details>
            <summary>Фрагмент екрана</summary>
            <CaptureFragment selection={selection} />
          </details>
        </>
      ) : (
        <p className="muted">Очікуємо фрагмент…</p>
      )}
      <Button className="close-result" variant="outline" size="sm" onClick={() => void close()}>
        Закрити · Esc
      </Button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </main>
  );
}
