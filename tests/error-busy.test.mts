import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (path: string) => readFileSync(join(root, path), "utf8");

const lib = read("src-tauri/src/lib.rs");
const page = read("src/app/page.tsx");
const overlay = read("src/app/overlay/page.tsx");
const api = read("src/lib/api.ts");

test("backend serializes scans and publishes busy state", () => {
  assert.match(lib, /scanning: AtomicBool/);
  assert.match(lib, /compare_exchange\(false, true/);
  assert.match(lib, /emit\("scan-busy"/);
  assert.match(api, /onScanBusy/);
  assert.match(page, /onScanBusy\(/);
});

test("scan region is disabled while any scan is busy", () => {
  assert.match(page, /disabled=\{isBusy\}/);
  assert.match(page, /const isBusy = busy \|\| remoteBusy/);
});

test("error toasts stay visible until replaced", () => {
  assert.match(page, /if \(!toast \|\| !toast\.ok\) return;/);
  assert.match(page, /aria-label="Dismiss"/);
});

test("every command-level scan error is emitted to the UI", () => {
  assert.match(lib, /fn emit_result_error/);
  assert.match(lib, /emit_result_error\(&app, show_overlay\(&app\)\)/);
  assert.match(lib, /run_scan\(app,/);
  assert.doesNotMatch(lib, /log::error!\("failed to show overlay: \{e\}"\)/);
});

test("overlay clears a stale error when shown again", () => {
  assert.match(overlay, /addEventListener\("focus", onFocus\)/);
});

test("async autostart result is ignored after unmount", () => {
  assert.match(page, /if \(active\) setAutoStart\(value\)/);
});

test("clearing history also closes the lightbox", () => {
  assert.match(page, /setViewImage\(null\)/);
});
