import assert from "node:assert/strict";
import { test } from "node:test";

import { clampPoint, rectBetween } from "../src/lib/selection.ts";

test("clampPoint keeps coordinates inside the overlay viewport", () => {
  assert.deepEqual(clampPoint(-10, -5, 1920, 1080), { x: 0, y: 0 });
  assert.deepEqual(clampPoint(2000, 1200, 1920, 1080), { x: 1920, y: 1080 });
  assert.deepEqual(clampPoint(100, 200, 1920, 1080), { x: 100, y: 200 });
});

test("rectBetween handles drags in both directions", () => {
  assert.deepEqual(rectBetween({ x: 10, y: 20 }, { x: 110, y: 80 }), {
    x: 10,
    y: 20,
    width: 100,
    height: 60,
  });
  assert.deepEqual(rectBetween({ x: 110, y: 80 }, { x: 10, y: 20 }), {
    x: 10,
    y: 20,
    width: 100,
    height: 60,
  });
});

test("rectBetween returns null when a point is missing", () => {
  assert.equal(rectBetween(null, { x: 1, y: 2 }), null);
  assert.equal(rectBetween({ x: 1, y: 2 }, null), null);
});
