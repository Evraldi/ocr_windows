import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (path: string) => readFileSync(join(root, path), "utf8");

test("scan results are rendered only from the scan-result event", () => {
  const page = read("src/app/page.tsx");

  assert.match(page, /api\.onScanResult\(handleResult\)/);
  assert.equal(page.includes("lastTimestamp"), false);
  assert.equal(/\.then\(handleResult\)/.test(page), false);
  assert.equal(page.includes("handleResult(await api.scanImage"), false);
  assert.match(page, /if \(result === null\) handleResult\(null\)/);
});

test("app info exposes OCR readiness to the main UI", () => {
  const api = read("src/lib/api.ts");
  const page = read("src/app/page.tsx");

  assert.match(api, /ocr_ready: boolean/);
  assert.match(api, /ocr_error: string \| null/);
  assert.match(page, /OCR loading…/);
  assert.match(page, /OCR unavailable/);
});

test("normal launch opens the main window while autostart stays in tray", () => {
  const lib = read("src-tauri/src/lib.rs");
  const autostart = read("src-tauri/src/autostart.rs");

  assert.match(lib, /has_background_flag\(std::env::args\(\)\.skip\(1\)\)/);
  assert.match(lib, /show_main\(app\.handle\(\)\)/);
  assert.match(autostart, /--background/);
});
