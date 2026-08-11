use image::{DynamicImage, GrayImage};

const MIN_DIM_FOR_DECODE: u32 = 100;
const MAX_DECODE_DIM: u32 = 4096;

/// A single candidate luma buffer ready for `rxing`: `(bytes, width, height)`.
pub type LumaCandidate = (Vec<u8>, u32, u32);

pub fn enhanced_luma(img: &DynamicImage) -> LumaCandidate {
    let gray = img.to_luma8();
    let stretched = stretch_contrast(&gray);

    let w = stretched.width();
    let h = stretched.height();
    let scale = upscale_factor(w.min(h));
    if scale > 1 && w < MAX_DECODE_DIM && h < MAX_DECODE_DIM {
        let resized =
            image::imageops::resize(&stretched, w * scale, h * scale, image::imageops::FilterType::Lanczos3);
        let (rw, rh) = (resized.width(), resized.height());
        (resized.into_raw(), rw, rh)
    } else {
        (stretched.into_raw(), w, h)
    }
}

/// A few plausible preprocessing variants. Barcode decoding is tried against each
/// one in order until something decodes, which raises the hit-rate on low-contrast
/// or small codes without paying for a big search on every scan.
pub fn variants(img: &DynamicImage) -> Vec<LumaCandidate> {
    let mut out = Vec::with_capacity(3);

    let gray = img.to_luma8();
    let (w, h) = (gray.width(), gray.height());

    let scale = upscale_factor(w.min(h));
    let scaled = if scale > 1 && w < MAX_DECODE_DIM && h < MAX_DECODE_DIM {
        image::imageops::resize(&gray, w * scale, h * scale, image::imageops::FilterType::Lanczos3)
    } else {
        gray.clone()
    };
    let (sw, sh) = (scaled.width(), scaled.height());
    out.push((scaled.into_raw(), sw, sh));

    let stretched = stretch_contrast(&gray);
    out.push((stretched.into_raw(), w, h));

    out.push(enhanced_luma(img));
    out
}

fn stretch_contrast(gray: &GrayImage) -> GrayImage {
    let mut min = u8::MAX;
    let mut max = u8::MIN;
    for px in gray.pixels() {
        let v = px[0];
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    if max == min {
        return gray.clone();
    }
    let range = max as u32 - min as u32;
    let mut out = gray.clone();
    for px in out.pixels_mut() {
        let v = px[0] as u32;
        let scaled = ((v - min as u32) * 255 + range / 2) / range;
        px[0] = scaled as u8;
    }
    out
}

fn upscale_factor(min_dim: u32) -> u32 {
    if min_dim == 0 {
        return 1;
    }
    if min_dim >= MIN_DIM_FOR_DECODE {
        return 1;
    }
    let factor = (MIN_DIM_FOR_DECODE + min_dim - 1) / min_dim;
    factor.min(8)
}
