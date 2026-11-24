import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const root = fileURLToPath(new URL("..", import.meta.url));
const read = (path: string) => readFileSync(join(root, path), "utf8");

test("ort does not enable copy-dylibs in CI", () => {
  const cargo = read("src-tauri/Cargo.toml");
  const active = cargo
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("#"))
    .join("\n");
  assert.equal(active.includes("copy-dylibs"), false);
  assert.match(
    cargo,
    /paddle-ocr-rs = \{ version = "0\.6", default-features = false/
  );
  assert.match(
    cargo,
    /ort = \{ version = "=2\.0\.0-rc\.10", default-features = false/
  );
});

test("root layout does not rely on generated LayoutProps type", () => {
  const layout = read("src/app/layout.tsx");
  assert.equal(layout.includes("LayoutProps"), false);
  assert.match(layout, /\{ children \}: \{ children: ReactNode \}/);
});
