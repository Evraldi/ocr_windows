import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (path: string) => readFileSync(join(root, path), "utf8");

const lib = read("src-tauri/src/lib.rs");
const models = read("src-tauri/src/models.rs");
const page = read("src/app/page.tsx");
const overlay = read("src/app/overlay/page.tsx");
const api = read("src/lib/api.ts");
const history = read("src-tauri/src/history.rs");

test("clipboard failure keeps the scan result visible", () => {
  assert.match(models, /pub copied: bool/);
  assert.match(api, /copied: boolean/);
  assert.match(page, /result\.copied/);
  assert.match(page, /clipboard unavailable/);
  assert.doesNotMatch(lib, /clipboard::copy_text\(&result\.content\)\.map_err/);
});

test("history returns every stored entry with lightweight thumbnails", () => {
  assert.match(lib, /list\(history::MAX_ENTRIES as usize\)/);
  assert.match(lib, /HISTORY_PREVIEW_MAX_DIM/);
  assert.match(history, /preview_full/);
  assert.match(api, /getHistoryPreview/);
  assert.match(page, /getHistoryPreview\(/);
});

test("clearing history also removes the last-result card", () => {
  assert.match(page, /setLast\(null\)/);
});

test("overlay scan failure does not rethrow into the console", () => {
  assert.doesNotMatch(overlay, /throw err/);
});

test("autostart read failure is reported instead of swallowed", () => {
  assert.doesNotMatch(
    page,
    /getAutostart\(\)[\s\S]{0,120}\.catch\(\(\) => undefined\)/
  );
});

test("history row actions are visible when focused by keyboard", () => {
  assert.equal((page.match(/group-focus-within:opacity-100/g) ?? []).length, 3);
});
