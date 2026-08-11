# OCR Scan Tool — Project Planning

## 1. Overview

Desktop app (Windows-first) yang capture screenshot dari region layar, terus auto-detect
isinya: kalau ada barcode/QR langsung di-decode, kalau isinya teks bebas (struk, dokumen,
KTP, dll) langsung di-OCR. Hasil auto-copy ke clipboard dan disimpan ke history lokal.

Mirip fungsinya kayak PowerToys Text Extractor / ShareX OCR, tapi built dari nol dan
self-hosted penuh (no cloud API, no external service dependency).

**Requirement utama:** akurasi harus tetap solid di berbagai kondisi input — teks di layar
(UI, dokumen digital) maupun hasil foto fisik (struk, kertas) yang mungkin miring/kurang
kontras.

## 2. Tech Stack

| Layer | Choice | Alasan |
|---|---|---|
| Core/backend | Rust + Tauri | Compile jadi single binary, no runtime dependency, cocok buat desktop tool lokal |
| Frontend/UI | Next.js | Konsisten sama project desktop tool lain (dev-launcher-tool) |
| State/history | SQLite via `rusqlite` | Local-first, no external DB server |

## 3. Pipeline / Architecture

```
Trigger capture (global hotkey / tray icon)
        |
Select region (overlay window)
        |
Capture & enhance (screenshot + preprocessing: grayscale, contrast, upscale)
        |
        +--> Decode code (barcode/QR via rxing)  --- kalau ketemu -> lanjut ke Output
        |
        +--> Read text (OCR fallback engine)     --- kalau code gak ketemu -> lanjut ke Output
        |
Output result (auto-copy clipboard + simpan ke history SQLite)
```

Barcode/QR decode dicoba duluan (murah secara compute, hasil pasti/structured). Kalau gak
ketemu kode apapun di region yang di-capture, baru fallback ke OCR teks bebas.

## 4. Component Breakdown & Crates

| Komponen | Crate / Tool | Catatan |
|---|---|---|
| Global hotkey + tray icon | `tauri-plugin-global-shortcut` | Trigger capture dari mana aja |
| Screenshot capture | `xcap` | Cross-platform screen capture |
| Region select overlay | Tauri window custom + Next.js overlay UI | Transparent window buat drag-select area |
| Image preprocessing | `image` crate | Grayscale, contrast boost, upscale — krusial buat naikin akurasi di kasus non-ideal |
| Barcode/QR decode | `rxing` | Port ZXing ke Rust, support QR/Code128/EAN/PDF417/DataMatrix, pure Rust |
| Text OCR | `ort` (ONNX Runtime binding) + model PaddleOCR PP-OCRv4 (exported ke ONNX) | Dipilih di atas Windows OCR API karena lebih stabil di kasus campuran (screen text + foto fisik) |
| Clipboard | `arboard` | Auto-copy hasil |
| History storage | `rusqlite` | Log hasil scan lokal |

**Catatan penting soal OCR engine:** model PP-OCRv4 perlu di-convert ke format ONNX
sekali di awal (proses one-time pakai Python + `paddle2onnx`), baru di-bundle ke aplikasi
Rust sebagai file model statis (~10-20MB nambah ke size app). Runtime-nya sendiri pure
Rust via `ort`, gak butuh Python terinstall di mesin end-user.

## 5. Proposed Project Structure

```
ocr-scan-tool/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── capture.rs        # screenshot + region select logic
│   │   ├── preprocess.rs     # image enhancement pipeline
│   │   ├── decode_code.rs    # rxing wrapper (barcode/QR)
│   │   ├── ocr_text.rs       # ort + PaddleOCR inference wrapper
│   │   ├── history.rs        # rusqlite CRUD
│   │   └── clipboard.rs      # arboard wrapper
│   ├── models/
│   │   └── pp-ocr-v4.onnx    # bundled OCR model
│   └── Cargo.toml
├── src/                       # Next.js frontend
│   ├── app/
│   │   ├── page.tsx           # main window: history list, settings
│   │   └── overlay/           # region-select overlay window
│   └── components/
├── planning.md
└── README.md
```

## 6. Build Phases

### Phase 1 — MVP: Barcode/QR only
- Global hotkey trigger + region select overlay
- Screenshot capture
- Decode via `rxing`
- Output ke clipboard (belum perlu history/UI dulu)
- **Acceptance:** bisa select region di layar, kalau ada QR/barcode di situ, hasil decode ke-copy ke clipboard dengan benar.

### Phase 2 — Preprocessing pipeline
- Tambah `image`-based enhancement (grayscale, contrast, upscale) sebelum masuk ke decode
- **Acceptance:** hit-rate decode naik terukur di kondisi capture kurang ideal (kode kecil, kontras rendah).

### Phase 3 — Text OCR fallback
- Convert PP-OCRv4 ke ONNX, bundle model
- Integrasi `ort` buat inference
- Branch logic: kalau `rxing` gak nemu kode, jalanin OCR teks
- **Acceptance:** teks dari screenshot UI maupun foto struk fisik ke-extract dengan akurasi wajar, hasil ke-copy ke clipboard.

### Phase 4 — History & UI polish
- Setup `rusqlite` buat nyimpen tiap hasil scan (timestamp, tipe hasil, isi)
- Next.js UI: list history, search, copy ulang, settings (hotkey binding, dll)
- **Acceptance:** semua hasil scan kesimpan, bisa di-browse dan di-copy ulang dari UI.

## 7. Open Notes / Decisions

- OCR engine final: **ONNX + PaddleOCR**, bukan Windows OCR API — trade-off setup lebih
  ribet di awal demi akurasi yang lebih konsisten di berbagai jenis input.
- Belum diputusin: scope multi-monitor support, dan apakah butuh setting buat toggle
  bahasa OCR (ID/EN) atau auto-detect.
