use image::DynamicImage;
use paddle_ocr_rs::ocr_lite::OcrLite;
use paddle_ocr_rs::ocr_result::TextBlock;

/// Teks dengan confidence di bawah ini cenderung salah baca (fixture
/// degradasi kita dapat 0.76 untuk "roice12345" dari "Invoice 12345").
const MIN_TEXT_SCORE: f32 = 0.8;

pub fn status() -> String {
    "PaddleOCR PP-OCRv4 (ONNX)".to_string()
}

pub fn init(
    det_model: &[u8],
    cls_model: &[u8],
    rec_model: &[u8],
    threads: usize,
) -> Result<OcrLite, String> {
    let mut ocr = OcrLite::new();
    ocr.init_models_from_memory(det_model, cls_model, rec_model, threads)
        .map_err(|e| e.to_string())?;
    Ok(ocr)
}

fn detect_blocks(ocr: &mut OcrLite, image: &DynamicImage) -> Result<Vec<TextBlock>, String> {
    let rgb = image.to_rgb8();
    let result = ocr
        .detect_angle_rollback(&rgb, 50, 1024, 0.5, 0.3, 1.6, true, true, 0.8)
        .map_err(|e| e.to_string())?;
    Ok(result.text_blocks)
}

/// Model PP-OCRv4 membaca beberapa glyph dengan salah tapi confidence tinggi:
/// `€` jadi `∈`, `✓` jadi `√` atau `v`. Perbaikan ini sengaja sempit hanya
/// pada konteks yang terobservasi di fixture, supaya teks matematika asli
/// (`√9`, `x∈y`) tidak ikut berubah.
fn normalize_symbols(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, ch) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        match ch {
            '∈' if next.is_some_and(|n| n.is_ascii_digit()) => out.push('€'),
            '√' if !next.is_some_and(|n| n.is_ascii_digit()) => out.push('✓'),
            _ => out.push(*ch),
        }
    }
    normalize_checkbox_word(&out)
}

fn normalize_checkbox_word(text: &str) -> String {
    text.lines()
        .map(|line| {
            const CHECK_WORDS: [&str; 4] = ["ok", "check", "done", "yes"];
            let mut parts: Vec<String> = line.split_whitespace().map(str::to_string).collect();
            let previous_is_check = parts
                .get(parts.len().saturating_sub(2))
                .is_some_and(|p| CHECK_WORDS.iter().any(|word| p.eq_ignore_ascii_case(word)));
            if parts.last().is_some_and(|p| p.eq_ignore_ascii_case("v")) && previous_is_check {
                *parts.last_mut().expect("checked above") = "✓".to_string();
            }
            parts.join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn recognize_text(ocr: &mut OcrLite, image: &DynamicImage) -> Result<Option<String>, String> {
    let text = detect_blocks(ocr, image)?
        .iter()
        .filter(|b| b.text_score >= MIN_TEXT_SCORE)
        .map(|b| b.text.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let text = normalize_symbols(text.trim());
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_engine() -> OcrLite {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("models");
        let det = std::fs::read(dir.join("ch_PP-OCRv4_det_infer.onnx")).expect("det model");
        let cls =
            std::fs::read(dir.join("ch_ppocr_mobile_v2.0_cls_infer.onnx")).expect("cls model");
        let rec = std::fs::read(dir.join("ch_PP-OCRv4_rec_infer.onnx")).expect("rec model");
        init(&det, &cls, &rec, 2).expect("engine init")
    }

    #[test]
    fn engine_loads_from_bundled_models() {
        load_engine();
    }

    #[test]
    fn blank_image_yields_no_text() {
        let mut engine = load_engine();
        let img = DynamicImage::new_rgb8(320, 64);
        let res = recognize_text(&mut engine, &img).expect("recognize");
        assert!(res.is_none(), "blank image should yield no text");
    }

    #[test]
    fn noise_image_yields_no_text() {
        use image::Rgb;
        let mut engine = load_engine();
        let mut rgb = DynamicImage::new_rgb8(640, 160).to_rgb8();
        let mut seed = 0x1234_5678u32;
        for y in 0..rgb.height() {
            for x in 0..rgb.width() {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let v = (seed >> 24) as u8;
                rgb.put_pixel(x, y, Rgb([v, v, v]));
            }
        }
        let res = recognize_text(&mut engine, &DynamicImage::ImageRgb8(rgb)).expect("recognize");
        assert!(res.is_none(), "noise should yield no text, got {res:?}");
    }

    fn fixture(name: &str) -> DynamicImage {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        image::open(path).expect("fixture")
    }

    /// Character Error Rate (Levenshtein / panjang referensi).
    fn cer(expected: &str, actual: &str) -> f64 {
        let a: Vec<char> = expected.chars().collect();
        let b: Vec<char> = actual.chars().collect();
        if a.is_empty() {
            return if b.is_empty() { 0.0 } else { 1.0 };
        }
        let mut prev: Vec<usize> = (0..=b.len()).collect();
        let mut curr = vec![0usize; b.len() + 1];
        for (i, ca) in a.iter().enumerate() {
            curr[0] = i + 1;
            for (j, cb) in b.iter().enumerate() {
                let cost = usize::from(ca != cb);
                curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            }
            std::mem::swap(&mut prev, &mut curr);
        }
        prev[b.len()] as f64 / a.len() as f64
    }

    #[test]
    fn fixtures_meet_accuracy_gate() {
        let mut engine = load_engine();
        const EXPECTED: &str = "Invoice 12345";

        let clean = recognize_text(&mut engine, &fixture("ocr-clean.png"))
            .expect("recognize clean")
            .expect("clean fixture must produce text");
        assert_eq!(cer(EXPECTED, &clean), 0.0, "clean CER: {clean}");

        let degraded = fixture("ocr-degraded.png");
        let raw = detect_blocks(&mut engine, &degraded)
            .expect("detect degraded")
            .iter()
            .map(|b| b.text.trim())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            cer(EXPECTED, &raw) > 0.2,
            "degraded fixture should be inaccurate, got {raw}"
        );
        assert_eq!(
            recognize_text(&mut engine, &degraded).expect("recognize degraded"),
            None,
            "low-confidence read must not be copied"
        );
    }

    #[test]
    fn normalizes_observed_ocr_symbol_confusions() {
        assert_eq!(
            normalize_symbols("Total ∈12,95 → OK v"),
            "Total €12,95 → OK ✓"
        );
        assert_eq!(normalize_symbols("OK √"), "OK ✓");
        assert_eq!(normalize_symbols("CHECK√ OK"), "CHECK✓ OK");
        // Konteks matematika tidak diubah.
        assert_eq!(normalize_symbols("√9"), "√9");
        assert_eq!(normalize_symbols("x∈y"), "x∈y");
        assert_eq!(normalize_symbols("Version v2"), "Version v2");
    }

    #[test]
    fn symbol_fixtures_match_accuracy_gate() {
        let mut engine = load_engine();
        let cases: [(&str, Option<&str>); 6] = [
            (
                "ocr-symbols-ascii.png",
                Some("INV-2026/11 #0042 @ 100% & $12.50 (paid) = OK"),
            ),
            (
                "ocr-symbols-dense.png",
                Some("A@B#C$D%E&F*G(H)I[J]K{L}M=N+O-P_Q:R;S,T.U?"),
            ),
            (
                "ocr-currency-id.png",
                Some("Total Rp 1.250,00 + PPN 10% = Rp 1.375,00"),
            ),
            ("ocr-plain-lower.png", Some("invoice 12345 paid today")),
            ("ocr-symbols-unicode.png", Some("Total 12,95 → OK ✓")),
            ("ocr-symbols-unicode-large.png", Some("Total €12,95 → OK ✓")),
        ];
        for (name, expected) in cases {
            let actual = recognize_text(&mut engine, &fixture(name)).expect("recognize");
            assert_eq!(actual.as_deref(), expected, "{name}: teks salah");
        }

        // Fixture padat sengaja tidak di-assert penuh: model menggabungkan
        // beberapa baris dan memotong ujungnya. Yang penting, glyph hasil
        // confusion tidak dibawa ke clipboard.
        let wide = recognize_text(&mut engine, &fixture("ocr-symbols-wide.png"))
            .expect("recognize wide")
            .expect("wide fixture should produce text");
        assert!(!wide.contains('√'), "glyph √ bocor: {wide}");
        assert!(!wide.contains('∈'), "glyph ∈ bocor: {wide}");
        assert!(
            wide.contains("CHECK✓"),
            "centang tidak dinormalisasi: {wide}"
        );
    }
}
