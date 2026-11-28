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

pub fn recognize_text(ocr: &mut OcrLite, image: &DynamicImage) -> Result<Option<String>, String> {
    let text = detect_blocks(ocr, image)?
        .iter()
        .filter(|b| b.text_score >= MIN_TEXT_SCORE)
        .map(|b| b.text.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let text = text.trim().to_string();
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
}
