import { useEffect, useRef, useState } from "react";
import { Button } from "./components/ui/button";
import { errorMessage, type Selection } from "./capture-api";
import { setStudyFlag } from "./history-api";
import {
  cancelTranslation,
  getTranslation,
  onTranslationProgress,
  translateText,
  MAX_SOURCE_LENGTH,
  type TranslationProgress,
} from "./translation-api";

export function TranslationPanel({ selection }: { selection: Selection }) {
  const [progress, setProgress] = useState<TranslationProgress | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [requesting, setRequesting] = useState(false);
  const [marking, setMarking] = useState(false);
  // Ignore progress from older requests, even if their events arrive late.
  const revision = useRef(0);
  // Native progress must not overwrite a manually edited source phrase.
  const edited = useRef(false);
  const selectionId = selection.id;

  useEffect(() => {
    let disposed = false;
    let stop: (() => void) | undefined;
    setProgress(null);
    setText("");
    setError("");
    setRequesting(false);
    edited.current = false;
    revision.current = 0;
    function update(value: TranslationProgress | null) {
      if (!disposed && value?.selectionId === selectionId && value.requestId >= revision.current) {
        revision.current = value.requestId;
        setProgress(value);
        if (!edited.current && value.sourceText) setText(value.sourceText);
      }
    }
    void (async () => {
      const cleanup = await onTranslationProgress(update);
      if (disposed) {
        cleanup();
        return;
      }
      stop = cleanup;
      update(await getTranslation());
    })().catch((value) => {
      if (!disposed) setError(errorMessage(value));
    });
    return () => {
      disposed = true;
      stop?.();
    };
  }, [selectionId]);

  async function requestTranslation() {
    setError("");
    setRequesting(true);
    try {
      await translateText(selectionId, text);
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setRequesting(false);
    }
  }

  async function toggleStudyFlag() {
    if (!progress?.historyId) return;
    setMarking(true);
    setError("");
    try {
      await setStudyFlag(progress.historyId, !progress.selectedForStudy);
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setMarking(false);
    }
  }

  const recognizing = progress?.phase === "recognizing";
  const translating = progress?.phase === "translating";
  const outdated = !!progress?.sourceText && text.trim() !== progress.sourceText.trim();

  return (
    <section className="translation-panel" aria-label="Переклад фрагмента">
      <label htmlFor="source-text">Англійський текст</label>
      <textarea
        id="source-text"
        value={text}
        onChange={(event) => {
          edited.current = true;
          setText(event.target.value);
        }}
        placeholder={recognizing ? "Розпізнаємо текст…" : "Введи або виправ текст"}
        rows={3}
        maxLength={MAX_SOURCE_LENGTH}
        disabled={recognizing}
      />
      <div className="translation-actions">
        <Button
          size="sm"
          onClick={() => void requestTranslation()}
          disabled={!text.trim() || recognizing || requesting || translating}
        >
          {requesting || translating ? "Перекладаємо…" : "Перекласти"}
        </Button>
        {(recognizing || translating) && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => void cancelTranslation().catch((value) => setError(errorMessage(value)))}
          >
            Скасувати запит
          </Button>
        )}
      </div>
      {recognizing && (
        <p role="status" className="muted">
          Розпізнаємо англійський текст…
        </p>
      )}
      <h2>Український переклад</h2>
      {progress?.translation ? (
        <>
          <p className="translated-text" aria-live="polite">
            {progress.translation}
          </p>
          {outdated && (
            <p className="muted">Текст змінено. Натисни «Перекласти», щоб оновити результат.</p>
          )}
        </>
      ) : (
        <p className="muted">
          {translating ? "Очікуємо відповідь моделі…" : "Тут з’явиться переклад."}
        </p>
      )}
      {(error || progress?.error) && (
        <p role="alert" className="error">
          {error || progress?.error}
        </p>
      )}
      {progress?.phase === "done" && progress.historyId && (
        <Button
          variant="outline"
          size="sm"
          aria-pressed={progress.selectedForStudy}
          disabled={marking || outdated}
          onClick={() => void toggleStudyFlag()}
        >
          {marking
            ? "Зберігаємо…"
            : progress.selectedForStudy
              ? "Прибрати з навчання"
              : "Для навчання"}
        </Button>
      )}
      {progress?.storageError && (
        <p role="alert" className="error">
          Переклад отримано, але історію не збережено: {progress.storageError}
        </p>
      )}
      {progress?.phase === "done" && (
        <p className="muted timing">
          {progress.model} ·{" "}
          {progress.ocrMs !== null && `OCR ${(progress.ocrMs / 1000).toFixed(1)} с · `}переклад{" "}
          {((progress.translationMs ?? 0) / 1000).toFixed(1)} с
        </p>
      )}
    </section>
  );
}
