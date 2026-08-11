use arboard::Clipboard;

pub fn copy_text(text: &str) -> Result<(), String> {
    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text.to_string()).map_err(|e| e.to_string())
}

/// Reads the current clipboard image (if any) and encodes it as a PNG
/// buffer. Returns `Ok(None)` when no image is available.
pub fn read_image_png() -> Result<Option<Vec<u8>>, String> {
    let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
    let img = match cb.get_image() {
        Ok(i) => i,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let rgba = image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
        .ok_or_else(|| "invalid clipboard image".to_string())?;
    let mut buf = Vec::new();
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(Some(buf))
}
