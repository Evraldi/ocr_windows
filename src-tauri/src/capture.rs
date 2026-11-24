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

/// Rectangle dalam coordinate physical absolut (lintas monitor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Region CSS relatif overlay -> physical absolut. Overlay dibuka tepat di
/// origin monitor yang aktif, jadi koordinat CSS harus diskalakan dengan DPR
/// lalu ditambah origin overlay. Tanpa origin ini, region di monitor sekunder
/// tidak akan pernah berpotongan dengan monitor mana pun.
pub fn region_to_physical(region: &Region, dpr: f64, origin: (i32, i32)) -> PhysicalRect {
    let scale = if dpr.is_finite() && dpr > 0.0 {
        dpr
    } else {
        1.0
    };
    let x = origin.0.saturating_add((region.x * scale).round() as i32);
    let y = origin.1.saturating_add((region.y * scale).round() as i32);
    PhysicalRect {
        x,
        y,
        width: (region.width * scale).round().max(1.0) as u32,
        height: (region.height * scale).round().max(1.0) as u32,
    }
}

/// Potongan `rect` yang berada di dalam `bounds`, atau `None` kalau tidak
/// berpotongan. Dipakai untuk menyusun crop dari beberapa monitor.
pub fn intersection(rect: PhysicalRect, bounds: PhysicalRect) -> Option<PhysicalRect> {
    let x0 = rect.x.max(bounds.x);
    let y0 = rect.y.max(bounds.y);
    let x1 = (rect.x + rect.width as i32).min(bounds.x + bounds.width as i32);
    let y1 = (rect.y + rect.height as i32).min(bounds.y + bounds.height as i32);
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    Some(PhysicalRect {
        x: x0,
        y: y0,
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
    })
}

pub fn capture_region(region: &Region, dpr: f64, origin: (i32, i32)) -> Result<DynamicImage> {
    let rect = region_to_physical(region, dpr, origin);

    let monitors = Monitor::all()?;
    if monitors.is_empty() {
        return Err(CaptureError::NoMonitor);
    }

    let mut canvas = DynamicImage::new_rgba8(rect.width, rect.height);

    for monitor in &monitors {
        let bounds = PhysicalRect {
            x: monitor.x()?,
            y: monitor.y()?,
            width: monitor.width()?,
            height: monitor.height()?,
        };
        let Some(part) = intersection(rect, bounds) else {
            continue;
        };

        let full = monitor.capture_image()?;
        let cropped = image::imageops::crop_imm(
            &full,
            (part.x - bounds.x) as u32,
            (part.y - bounds.y) as u32,
            part.width,
            part.height,
        )
        .to_image();

        let ox = (part.x - rect.x) as u32;
        let oy = (part.y - rect.y) as u32;
        let mut base = canvas.to_rgba8();
        image::imageops::overlay(&mut base, &cropped, ox as i64, oy as i64);
        canvas = DynamicImage::ImageRgba8(base);
    }

    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(x: f64, y: f64, width: f64, height: f64) -> Region {
        Region {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn maps_primary_monitor_css_to_physical() {
        let got = region_to_physical(&region(10.0, 20.0, 100.0, 50.0), 1.0, (0, 0));
        assert_eq!(
            got,
            PhysicalRect {
                x: 10,
                y: 20,
                width: 100,
                height: 50
            }
        );
    }

    #[test]
    fn maps_secondary_monitor_with_dpi_scale_and_negative_origin() {
        let got = region_to_physical(&region(25.5, 40.0, 200.0, 80.0), 1.5, (-1920, 300));
        assert_eq!(got.x, -1920 + 38);
        assert_eq!(got.y, 300 + 60);
        assert_eq!(got.width, 300);
        assert_eq!(got.height, 120);
    }

    #[test]
    fn invalid_dpr_falls_back_to_one() {
        let got = region_to_physical(&region(12.4, 8.6, 10.0, 10.0), f64::NAN, (100, 200));
        assert_eq!((got.x, got.y), (112, 209));
    }

    #[test]
    fn intersection_returns_overlap_only() {
        let rect = PhysicalRect {
            x: 100,
            y: 100,
            width: 200,
            height: 200,
        };
        let bounds = PhysicalRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(intersection(rect, bounds), Some(rect));
    }

    #[test]
    fn intersection_handles_monitor_on_the_left() {
        let rect = PhysicalRect {
            x: -50,
            y: 100,
            width: 200,
            height: 200,
        };
        let bounds = PhysicalRect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(
            intersection(rect, bounds),
            Some(PhysicalRect {
                x: -50,
                y: 100,
                width: 50,
                height: 200
            })
        );
    }

    #[test]
    fn intersection_rejects_disjoint_monitors() {
        let rect = PhysicalRect {
            x: 10,
            y: 10,
            width: 20,
            height: 20,
        };
        let bounds = PhysicalRect {
            x: -1920,
            y: 0,
            width: 1920,
            height: 1080,
        };
        assert_eq!(intersection(rect, bounds), None);
    }
}
