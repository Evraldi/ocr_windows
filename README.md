# OCR Scan Tool

Aplikasi desktop Windows untuk scan barcode/QR dan teks dari area layar, gambar file, atau clipboard. Shortcut global: **Ctrl+Shift+S**.

## Prasyarat

- Windows 10/11
- Node.js 24 (lihat `.nvmrc`)
- Rust toolchain stabil (via [rustup](https://rustup.rs/))
- WebView2 Runtime (sudah ada di Windows 11 / kebanyakan Windows 10)
- Visual Studio Build Tools dengan **Desktop development with C++** (dibutuhkan `ort-sys`)

## Development

```bash
npm ci
npm run tauri dev
```

`npm run tauri dev` otomatis menjalankan `next dev` dan membuka jendela Tauri. Jangan pakai `cargo tauri` — script npm adalah entry point yang didukung.

## Build installer

```bash
npm run tauri build
```

Hasil: installer NSIS (`.exe`) dan MSI di `src-tauri/target/release/bundle/`.

ONNX Runtime di-link **statically** oleh `ort-sys` (default feature `download-binaries`), jadi tidak ada `onnxruntime.dll` yang perlu disalin ke installer. Build membutuhkan akses network sekali untuk mengunduh binary yang diverifikasi SHA-256; setelah itu cache di `%LOCALAPPDATA%\ort.pyke.io` dipakai ulang.

Model PaddleOCR ada di `src-tauri/resources/models/` dan ikut ter-bundle ke folder `resources/models/`.

## Gate kualitas

```bash
npm run typecheck
npm run lint
npm test
npm run build
npm run audit:prod
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

`audit:prod` memakai `audit-allowlist.json` untuk advisory yang memang tidak reachable dari static export. Allowlist punya tanggal kedaluwarsa; perbarui setelah ditinjau.

## Arsitektur singkat

- `src/app/page.tsx` — UI utama, history, import/paste gambar, autostart.
- `src/app/overlay/page.tsx` — overlay drag-select, mengirim region CSS + DPR ke backend.
- `src-tauri/src/capture.rs` — konversi region ke koordinat physical absolut dan crop lintas monitor.
- `src-tauri/src/decode_code.rs` — QR/Code128 via `rxing`.
- `src-tauri/src/ocr_text.rs` — fallback teks via `paddle-ocr-rs` / ONNX.
- `src-tauri/src/history.rs` — SQLite di app data directory.
- `src-tauri/src/clipboard.rs` — copy/read clipboard gambar.

## Catatan keamanan

- CSP aktif baik di produksi maupun dev (`devCsp` mengizinkan HMR).
- Single-instance plugin aktif: instance kedua hanya memfokuskan jendela yang sudah ada.
- Tidak ada updater atau code signing yang diaktifkan tanpa sertifikat/kunci dan endpoint yang valid.
- Semua data (history, model, hasil scan) lokal; tidak ada telemetry atau jaringan dari aplikasi.
