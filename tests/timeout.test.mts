import assert from "node:assert/strict";
import { test } from "node:test";

import { withTimeout } from "../src/lib/timeout.ts";

test("returns the original value when the promise resolves in time", async () => {
  assert.equal(await withTimeout(Promise.resolve(42), 1000, "fast"), 42);
});

test("rejects a hanging invoke with the command name", async () => {
  await assert.rejects(
    withTimeout(new Promise<number>(() => undefined), 20, "finish_scan"),
    /finish_scan timed out after 20ms/
  );
});

test("propagates the original error immediately", async () => {
  await assert.rejects(
    withTimeout(Promise.reject(new Error("boom")), 1000, "scan_image"),
    /boom/
  );
});
