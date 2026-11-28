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

    fn padded_image(contents: &str, format: BarcodeFormat, scale: u32, quiet: u32) -> DynamicImage {
        let matrix = MultiFormatWriter
            .encode(contents, &format, 0, 0)
            .expect("encode failed");
        let w = matrix.getWidth();
        let h = matrix.getHeight();
        let mut gray = GrayImage::from_pixel(
            (w + quiet * 2) * scale,
            (h + quiet * 2) * scale,
            Luma([255]),
        );
        for y in 0..h {
            for x in 0..w {
                if matrix.get(x, y) {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            gray.put_pixel(
                                (x + quiet) * scale + dx,
                                (y + quiet) * scale + dy,
                                Luma([0]),
                            );
                        }
                    }
                }
            }
        }
        DynamicImage::ImageLuma8(gray)
    }

    #[test]
    fn decodes_all_supported_symbol_formats() {
        // CODABAR sengaja pakai payload `A1234B`: `A`/`B` adalah penanda
        // start/stop, jadi hasil decode yang benar adalah `1234`.
        let cases: [(&str, BarcodeFormat, &str, &str, &str); 13] = [
            (
                "QR_CODE",
                BarcodeFormat::QR_CODE,
                "https://example.com/scan",
                "https://example.com/scan",
                "qrcode",
            ),
            (
                "DATA_MATRIX",
                BarcodeFormat::DATA_MATRIX,
                "DM-12345",
                "DM-12345",
                "datamatrix",
            ),
            (
                "AZTEC",
                BarcodeFormat::AZTEC,
                "AZTEC-42",
                "AZTEC-42",
                "aztec",
            ),
            (
                "PDF_417",
                BarcodeFormat::PDF_417,
                "PDF-2026",
                "PDF-2026",
                "pdf 417",
            ),
            (
                "CODE_128",
                BarcodeFormat::CODE_128,
                "PRODUCT-90210",
                "PRODUCT-90210",
                "code 128",
            ),
            (
                "CODE_39",
                BarcodeFormat::CODE_39,
                "CODE39-77",
                "CODE39-77",
                "code 39",
            ),
            (
                "CODE_93",
                BarcodeFormat::CODE_93,
                "CODE93-88",
                "CODE93-88",
                "code 93",
            ),
            (
                "EAN_13",
                BarcodeFormat::EAN_13,
                "5901234123457",
                "5901234123457",
                "ean 13",
            ),
            (
                "EAN_8",
                BarcodeFormat::EAN_8,
                "96385074",
                "96385074",
                "ean 8",
            ),
            (
                "UPC_A",
                BarcodeFormat::UPC_A,
                "036000291452",
                "036000291452",
                "upc a",
            ),
            (
                "UPC_E",
                BarcodeFormat::UPC_E,
                "04252614",
                "04252614",
                "upc e",
            ),
            ("ITF", BarcodeFormat::ITF, "1234567890", "1234567890", "itf"),
            (
                "CODABAR",
                BarcodeFormat::CODABAR,
                "A1234B",
                "1234",
                "codabar",
            ),
        ];
        for (name, format, payload, expected, expected_format) in cases {
            let results = decode(&padded_image(payload, format, 4, 6));
            assert!(!results.is_empty(), "{name} tidak terdeteksi");
            assert_eq!(results[0].0, expected, "{name}: payload salah");
            assert_eq!(results[0].1, expected_format, "{name}: format salah");
        }
    }

    fn colored_image(
        contents: &str,
        format: BarcodeFormat,
        scale: u32,
        quiet: u32,
        dark: u8,
        light: u8,
    ) -> DynamicImage {
        let matrix = MultiFormatWriter
            .encode(contents, &format, 0, 0)
            .expect("encode failed");
        let w = matrix.getWidth();
        let h = matrix.getHeight();
        let mut gray = GrayImage::from_pixel(
            (w + quiet * 2) * scale,
            (h + quiet * 2) * scale,
            Luma([light]),
        );
        for y in 0..h {
            for x in 0..w {
                if matrix.get(x, y) {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            gray.put_pixel(
                                (x + quiet) * scale + dx,
                                (y + quiet) * scale + dy,
                                Luma([dark]),
                            );
                        }
                    }
                }
            }
        }
        DynamicImage::ImageLuma8(gray)
    }

    fn rotate_gray(gray: &GrayImage, degrees: f32) -> GrayImage {
        let (w, h) = (gray.width(), gray.height());
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let angle = degrees * std::f32::consts::PI / 180.0;
        let (cos, sin) = (angle.cos(), angle.sin());
        let mut out = GrayImage::from_pixel(w, h, Luma([255]));
        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let sx = cx + dx * cos + dy * sin;
                let sy = cy - dx * sin + dy * cos;
                if sx >= 0.0 && sy >= 0.0 && sx < w as f32 && sy < h as f32 {
                    out.put_pixel(x, y, *gray.get_pixel(sx as u32, sy as u32));
                }
            }
        }
        out
    }

    #[test]
    fn decodes_degraded_and_multiple_symbols() {
        let low_contrast =
            colored_image("LOW-CONTRAST-77", BarcodeFormat::CODE_128, 4, 8, 120, 200);
        assert_eq!(
            decode(&low_contrast),
            vec![("LOW-CONTRAST-77".to_string(), "code 128".to_string())]
        );

        let qr = padded_image("ROTATED-QR", BarcodeFormat::QR_CODE, 4, 10);
        for angle in [5.0f32, 15.0, 30.0] {
            let rotated = DynamicImage::ImageLuma8(rotate_gray(&qr.to_luma8(), angle));
            assert_eq!(
                decode(&rotated),
                vec![("ROTATED-QR".to_string(), "qrcode".to_string())],
                "QR gagal pada rotasi {angle}°"
            );
        }

        let tiny = padded_image("TINY-QR-9", BarcodeFormat::QR_CODE, 1, 2);
        assert_eq!(
            decode(&tiny),
            vec![("TINY-QR-9".to_string(), "qrcode".to_string())]
        );

        let left = padded_image("https://a.example", BarcodeFormat::QR_CODE, 4, 6).to_luma8();
        let right = padded_image("MULTI-128", BarcodeFormat::CODE_128, 4, 6).to_luma8();
        let mut combined = GrayImage::from_pixel(
            left.width() + right.width() + 40,
            left.height(),
            Luma([255]),
        );
        image::imageops::overlay(&mut combined, &left, 0, 0);
        image::imageops::overlay(&mut combined, &right, (left.width() + 40) as i64, 0);
        assert_eq!(
            decode(&DynamicImage::ImageLuma8(combined)),
            vec![
                ("https://a.example".to_string(), "qrcode".to_string()),
                ("MULTI-128".to_string(), "code 128".to_string()),
            ]
        );
    }
}
