use image::DynamicImage;
use paddle_ocr_rs::ocr_lite::OcrLite;

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

pub fn recognize_text(ocr: &mut OcrLite, image: &DynamicImage) -> Result<Option<String>, String> {
    let rgb = image.to_rgb8();
    let result = ocr
        .detect_angle_rollback(&rgb, 50, 1024, 0.5, 0.3, 1.6, true, true, 0.8)
        .map_err(|e| e.to_string())?;

    let text = result
        .text_blocks
        .iter()
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
}
