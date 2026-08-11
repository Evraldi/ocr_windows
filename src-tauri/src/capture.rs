use image::DynamicImage;
use thiserror::Error;
use xcap::{Monitor, XCapError};

use crate::models::Region;

pub type Result<T> = std::result::Result<T, CaptureError>;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("no display detected")]
    NoMonitor,
    #[error("screen capture failed: {0}")]
    XCap(#[from] XCapError),
    #[error("image processing failed: {0}")]
    Image(#[from] image::ImageError),
}

pub fn capture_region(region: &Region, dpr: f64) -> Result<DynamicImage> {
    let rx = (region.x * dpr).round() as i32;
    let ry = (region.y * dpr).round() as i32;
    let rw = ((region.width * dpr).round().max(1.0)) as u32;
    let rh = ((region.height * dpr).round().max(1.0)) as u32;

    let monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitor);
    }

    let mut canvas = DynamicImage::new_rgba8(rw, rh);

    for monitor in &monitors {
        let mx = monitor.x()?;
        let my = monitor.y()?;
        let mw = monitor.width()?;
        let mh = monitor.height()?;

        let ix0 = rx.max(mx);
        let iy0 = ry.max(my);
        let ix1 = (rx + rw as i32).min(mx + mw as i32);
        let iy1 = (ry + rh as i32).min(my + mh as i32);
        if ix0 >= ix1 || iy0 >= iy1 {
            continue;
        }

        let iw = (ix1 - ix0) as u32;
        let ih = (iy1 - iy0) as u32;

        let full = monitor.capture_image()?;
        let local_x = (ix0 - mx) as u32;
        let local_y = (iy0 - my) as u32;
        let cropped = image::imageops::crop_imm(&full, local_x, local_y, iw, ih).to_image();

        let ox = (ix0 - rx) as u32;
        let oy = (iy0 - ry) as u32;
        let mut base = canvas.to_rgba8();
        image::imageops::overlay(&mut base, &cropped, ox as i64, oy as i64);
        canvas = DynamicImage::ImageRgba8(base);
    }

    Ok(canvas)
}