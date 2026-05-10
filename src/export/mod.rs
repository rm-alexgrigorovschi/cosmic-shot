pub mod crop;
pub use crop::{CroppedImage, crop_selection};

use std::path::Path;

use image::{ImageBuffer, Rgba};

use crate::config::OutputFormat;
use crate::types::{ConversionError, FrameBuffer};

/// Errors that can occur during image export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("image encoding failed: {0}")]
    Encode(String),

    #[error("failed to write file: {0}")]
    Io(#[from] std::io::Error),

    #[error("pixel conversion failed: {0}")]
    Conversion(#[from] ConversionError),

    #[error("clipboard error: {0}")]
    Clipboard(String),
}

/// Save a captured frame as a PNG file.
#[allow(dead_code)]
pub fn save_png(frame: &FrameBuffer, path: &Path) -> Result<(), ExportError> {
    let rgba = frame.to_rgba()?;

    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(frame.width, frame.height, rgba)
            .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".to_string()))?;

    img.save(path).map_err(|e| ExportError::Encode(e.to_string()))?;

    tracing::info!(path = %path.display(), "saved PNG");
    Ok(())
}

/// Copy a cropped image to the system clipboard via `wl-copy`.
///
/// Encodes the image as PNG and pipes it to `wl-copy --type image/png`.
/// `wl-copy` daemonises itself so the clipboard content persists after
/// cosmic-shot exits. Falls back to an error if `wl-copy` is not found.
pub fn copy_to_clipboard(image: &CroppedImage) -> Result<(), ExportError> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Encode as PNG in memory.
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
            .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".to_string()))?;

    let mut png_bytes: Vec<u8> = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| ExportError::Encode(e.to_string()))?;

    // Pipe PNG bytes to wl-copy. wl-copy forks itself and serves the
    // clipboard independently, so the content survives process exit.
    let mut child = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| ExportError::Clipboard(format!("failed to spawn wl-copy: {e}")))?;

    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(&png_bytes)
        .map_err(|e| ExportError::Clipboard(format!("failed to write to wl-copy: {e}")))?;

    let status = child
        .wait()
        .map_err(|e| ExportError::Clipboard(format!("wl-copy wait failed: {e}")))?;

    if !status.success() {
        return Err(ExportError::Clipboard(format!(
            "wl-copy exited with status {status}"
        )));
    }

    tracing::info!("copied {}×{} image to clipboard via wl-copy", image.width, image.height);
    Ok(())
}

/// Save a cropped image to a file in the specified format.
pub fn save_cropped(
    image: &CroppedImage,
    path: &Path,
    format: OutputFormat,
    quality: u8,
) -> Result<(), ExportError> {
    let quality = quality.clamp(1, 100);

    match format {
        OutputFormat::Png => {
            let img: ImageBuffer<Rgba<u8>, _> =
                ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
                    .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".into()))?;
            img.save(path)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
        OutputFormat::Jpeg => {
            // JPEG doesn't support alpha — convert RGBA to RGB.
            let rgb_data: Vec<u8> = image
                .rgba
                .chunks_exact(4)
                .flat_map(|px| [px[0], px[1], px[2]])
                .collect();
            let img: ImageBuffer<image::Rgb<u8>, _> =
                ImageBuffer::from_raw(image.width, image.height, rgb_data)
                    .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".into()))?;
            let mut file = std::fs::File::create(path)?;
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);
            img.write_with_encoder(encoder)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
        OutputFormat::WebP => {
            let img: ImageBuffer<Rgba<u8>, _> =
                ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
                    .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".into()))?;
            let mut file = std::fs::File::create(path)?;
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut file);
            img.write_with_encoder(encoder)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
    }

    tracing::info!(path = %path.display(), format = ?format, "saved cropped image");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OutputFormat;

    /// Create a tiny 2x2 RGBA test image.
    fn make_2x2_cropped() -> CroppedImage {
        CroppedImage {
            rgba: vec![
                255, 0, 0, 255,   0, 255, 0, 255,    // row 0: red, green
                0, 0, 255, 255,   255, 255, 0, 128,   // row 1: blue, yellow (semi-transparent)
            ],
            width: 2,
            height: 2,
        }
    }

    #[test]
    fn save_cropped_png() {
        let img = make_2x2_cropped();
        let dir = std::env::temp_dir().join("cosmic-shot-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.png");
        save_cropped(&img, &path, OutputFormat::Png, 85).unwrap();
        assert!(path.exists());
        let decoded = image::open(&path).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn save_cropped_jpeg() {
        let img = make_2x2_cropped();
        let dir = std::env::temp_dir().join("cosmic-shot-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.jpg");
        save_cropped(&img, &path, OutputFormat::Jpeg, 85).unwrap();
        assert!(path.exists());
        let decoded = image::open(&path).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn save_cropped_webp() {
        let img = make_2x2_cropped();
        let dir = std::env::temp_dir().join("cosmic-shot-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.webp");
        save_cropped(&img, &path, OutputFormat::WebP, 90).unwrap();
        assert!(path.exists());
        let decoded = image::open(&path).unwrap();
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        std::fs::remove_file(&path).ok();
    }
}
