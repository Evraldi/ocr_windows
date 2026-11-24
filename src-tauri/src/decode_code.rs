use image::DynamicImage;

use crate::preprocess;

pub fn decode(img: &DynamicImage) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for (luma, w, h) in preprocess::variants(img) {
        if let Ok(results) = rxing::helpers::detect_multiple_in_luma(luma, w, h) {
            for r in results {
                let pair = (r.getText().to_string(), r.getBarcodeFormat().to_string());
                if !found.contains(&pair) {
                    found.push(pair);
                }
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GrayImage, Luma};
    use rxing::common::BitMatrix;
    use rxing::{BarcodeFormat, MultiFormatWriter, Writer};

    fn qr_image(contents: &str, scale: u32) -> DynamicImage {
        let writer = MultiFormatWriter;
        let matrix = writer
            .encode(contents, &BarcodeFormat::QR_CODE, 0, 0)
            .expect("encode failed");
        bit_matrix_to_image(&matrix, scale)
    }

    fn code128_image(contents: &str, scale: u32) -> DynamicImage {
        let writer = MultiFormatWriter;
        let matrix = writer
            .encode(contents, &BarcodeFormat::CODE_128, 0, 100)
            .expect("encode failed");
        bit_matrix_to_image(&matrix, scale)
    }

    fn bit_matrix_to_image(matrix: &BitMatrix, scale: u32) -> DynamicImage {
        let w = matrix.getWidth();
        let h = matrix.getHeight();
        let mut gray = GrayImage::new(w * scale, h * scale);
        for y in 0..h {
            for x in 0..w {
                let v = if matrix.get(x, y) { 0u8 } else { 255u8 };
                for dy in 0..scale {
                    for dx in 0..scale {
                        gray.put_pixel(x * scale + dx, y * scale + dy, Luma([v]));
                    }
                }
            }
        }
        DynamicImage::ImageLuma8(gray)
    }

    #[test]
    fn decodes_qr_code() {
        let img = qr_image("https://example.com/scan", 4);
        let results = decode(&img);
        assert!(!results.is_empty(), "QR not detected");
        assert_eq!(results[0].0, "https://example.com/scan");
    }

    #[test]
    fn decodes_small_qr_with_upscale() {
        let img = qr_image("SMALL-QR-42", 1);
        let results = decode(&img);
        assert!(!results.is_empty(), "small QR not detected");
        assert_eq!(results[0].0, "SMALL-QR-42");
    }

    #[test]
    fn decodes_code128() {
        let img = code128_image("PRODUCT-90210", 3);
        let results = decode(&img);
        assert!(!results.is_empty(), "Code128 not detected");
        assert_eq!(results[0].0, "PRODUCT-90210");
    }

    #[test]
    fn returns_empty_for_plain_text() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(200, 60, Luma([255])));
        assert!(decode(&img).is_empty());
    }
}
