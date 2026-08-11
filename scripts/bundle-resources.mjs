// Copies the ONNX Runtime DLL next to the bundle so the packaged app can find it.
// Runs after `cargo build` (via Tauri's beforeBundleCommand), so the DLL produced
// by ort's `copy-dylibs` already exists under target/{debug,release}.
import { cpSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const destDir = join(root, "resources", "ort");
const dest = join(destDir, "onnxruntime.dll");

for (const profile of ["release", "debug"]) {
  const src = join(root, "src-tauri", "target", profile, "onnxruntime.dll");
  if (existsSync(src)) {
    mkdirSync(destDir, { recursive: true });
    cpSync(src, dest);
    console.log(`[bundle-resources] copied ${src} -> ${dest}`);
    process.exit(0);
  }
}

console.warn("[bundle-resources] onnxruntime.dll not found; skipping copy");