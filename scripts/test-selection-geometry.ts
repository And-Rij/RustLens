import assert from "node:assert/strict";
import test from "node:test";
import { captureRect, selectionBox } from "../src/lib/selection-geometry.ts";

test("reversed dragging produces the same crop at 200% display scaling", () => {
  const frame = { width: 1920, height: 1080 };
  const surface = { width: 960, height: 540 };
  const drag = { start: { x: 450, y: 100 }, end: { x: 50, y: 50 } };
  assert.deepEqual(captureRect(drag, frame, surface), {
    x: 100,
    y: 100,
    width: 800,
    height: 100,
  });
  assert.deepEqual(
    captureRect({ start: drag.end, end: drag.start }, frame, surface),
    captureRect(drag, frame, surface),
  );
  assert.deepEqual(selectionBox(drag), { left: 50, top: 50, width: 400, height: 50 });
});

test("fractional coordinates round outward without exceeding the frame", () => {
  assert.deepEqual(
    captureRect(
      { start: { x: 0.2, y: 0.2 }, end: { x: 1.1, y: 1.1 } },
      { width: 100, height: 100 },
      { width: 50, height: 50 },
    ),
    { x: 0, y: 0, width: 3, height: 3 },
  );
  assert.deepEqual(
    captureRect(
      { start: { x: 49, y: 49 }, end: { x: 50, y: 50 } },
      { width: 100, height: 100 },
      { width: 50, height: 50 },
    ),
    { x: 98, y: 98, width: 2, height: 2 },
  );
});
