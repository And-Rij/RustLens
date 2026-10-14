import type { CaptureRect, Frame } from "../capture-api";

export interface Point {
  x: number;
  y: number;
}

export interface Drag {
  start: Point;
  end: Point;
}

interface SurfaceSize {
  width: number;
  height: number;
}

/** CSS coordinates for the visible selection box, regardless of drag direction. */
export function selectionBox(drag: Drag) {
  return {
    left: Math.min(drag.start.x, drag.end.x),
    top: Math.min(drag.start.y, drag.end.y),
    width: Math.abs(drag.start.x - drag.end.x),
    height: Math.abs(drag.start.y - drag.end.y),
  };
}

/**
 * Convert CSS pixels into physical screenshot pixels before invoking Rust.
 * The two dimensions are scaled independently to support display scaling.
 * Round the edges outward so partially covered pixels remain in the crop.
 */
export function captureRect(
  drag: Drag,
  frame: Pick<Frame, "width" | "height">,
  surface: SurfaceSize,
): CaptureRect {
  const x = Math.floor((Math.min(drag.start.x, drag.end.x) * frame.width) / surface.width);
  const y = Math.floor((Math.min(drag.start.y, drag.end.y) * frame.height) / surface.height);
  const right = Math.min(
    frame.width,
    Math.ceil((Math.max(drag.start.x, drag.end.x) * frame.width) / surface.width),
  );
  const bottom = Math.min(
    frame.height,
    Math.ceil((Math.max(drag.start.y, drag.end.y) * frame.height) / surface.height),
  );

  return { x, y, width: right - x, height: bottom - y };
}
