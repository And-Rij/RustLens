import { useEffect, useRef, useState, type PointerEvent } from "react";
import { Button } from "../components/ui/button";
import {
  dismissCapture,
  errorMessage,
  getFrame,
  onCapture,
  showCapture,
  submitSelection,
  type CaptureRect,
  type Frame,
} from "../capture-api";
import { captureRect, selectionBox, type Drag } from "../lib/selection-geometry";

/** Frozen screenshot overlay; all submitted coordinates are physical pixels. */
export function SelectionView() {
  const [frame, setFrame] = useState<Frame | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const submitting = useRef(false);
  // Pointer events may arrive before React renders the latest drag state.
  const dragRef = useRef<Drag | null>(null);

  useEffect(() => {
    let disposed = false;
    let cleanup: (() => void) | undefined;
    async function refresh() {
      const next = await getFrame();
      if (!disposed) {
        setFrame(next);
        setDrag(null);
        dragRef.current = null;
        setError("");
        setBusy(false);
        submitting.current = false;
      }
    }
    void (async () => {
      const stop = await onCapture(() => {
        void refresh().catch((value) => {
          if (!disposed) setError(errorMessage(value));
        });
      });
      if (disposed) {
        stop();
        return;
      }
      cleanup = stop;
      await refresh();
    })().catch((value) => {
      if (!disposed) setError(errorMessage(value));
    });
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, []);

  async function cancelSelection() {
    try {
      await dismissCapture();
      setFrame(null);
      setDrag(null);
      dragRef.current = null;
    } catch (value) {
      setError(errorMessage(value));
    }
  }

  async function submitRegion(rect: CaptureRect) {
    if (!frame || submitting.current) return;
    submitting.current = true;
    setBusy(true);
    setError("");
    try {
      await submitSelection(frame.id, rect);
      setFrame(null);
      setDrag(null);
      dragRef.current = null;
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      submitting.current = false;
      setBusy(false);
    }
  }

  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void cancelSelection();
      }
      if (event.key === "Enter" && frame && !submitting.current && event.target === document.body) {
        event.preventDefault();
        void submitRegion({ x: 0, y: 0, width: frame.width, height: frame.height });
      }
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });

  function pointerPosition(event: PointerEvent<HTMLDivElement>) {
    const bounds = event.currentTarget.getBoundingClientRect();
    return {
      x: Math.max(0, Math.min(bounds.width, event.clientX - bounds.left)),
      y: Math.max(0, Math.min(bounds.height, event.clientY - bounds.top)),
    };
  }

  function startDrag(event: PointerEvent<HTMLDivElement>) {
    if (!frame || busy || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const p = pointerPosition(event);
    const next = { start: p, end: p };
    dragRef.current = next;
    setDrag(next);
    setError("");
  }

  function updateDrag(event: PointerEvent<HTMLDivElement>) {
    if (!dragRef.current || busy) return;
    const next = { start: dragRef.current.start, end: pointerPosition(event) };
    dragRef.current = next;
    setDrag(next);
  }

  function finishDrag(event: PointerEvent<HTMLDivElement>) {
    const current = dragRef.current;
    if (!current || !frame || busy) return;
    const end = pointerPosition(event);
    const bounds = event.currentTarget.getBoundingClientRect();
    const rect = captureRect({ start: current.start, end }, frame, bounds);
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    void submitRegion(rect);
  }

  const box = drag ? selectionBox(drag) : null;

  return (
    <div
      className="selection-surface"
      onPointerDown={startDrag}
      onPointerMove={updateDrag}
      onPointerUp={finishDrag}
      onPointerCancel={() => {
        dragRef.current = null;
        setDrag(null);
      }}
    >
      {frame && (
        <img
          className="snapshot"
          src={frame.imageUrl}
          alt="Захоплений екран для вибору області"
          draggable={false}
          onLoad={() => {
            void showCapture(frame.id).catch((value) => setError(errorMessage(value)));
          }}
        />
      )}
      <div className="selection-shade" />
      {box && <div className="selection-box" style={box} />}
      <div className="selection-help" onPointerDown={(event) => event.stopPropagation()}>
        <p>{busy ? "Готуємо фрагмент…" : "Обведи текст мишею"}</p>
        <p>Esc — скасувати · Enter — весь екран</p>
        <Button variant="outline" size="sm" onClick={() => void cancelSelection()}>
          Скасувати
        </Button>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
      </div>
    </div>
  );
}
