import { useEffect, useState } from "react";
import { getSelection, nativeAvailable, onSelection, type Selection } from "../capture-api";

/** Subscribe before loading the snapshot so a newer native event always wins. */
export function useSelection() {
  const [selection, setSelection] = useState<Selection | null>(null);
  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    let receivedEvent = false;
    void (async () => {
      const stop = await onSelection((value) => {
        receivedEvent = true;
        if (!disposed) setSelection(value);
      });
      if (disposed) {
        stop();
        return;
      }
      cleanup = stop;
      const value = await getSelection();
      if (!disposed && !receivedEvent) setSelection(value);
    })().catch(console.error);
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, []);
  return selection;
}
