pub mod crop;
pub use crop::{CroppedImage, crop_selection};

use std::path::Path;

use image::{ImageBuffer, Rgba};

use crate::types::{ConversionError, FrameBuffer};

/// Errors that can occur during image export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("PNG encoding failed: {0}")]
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

/// Save a cropped image as a PNG file.
pub fn save_cropped_png(image: &CroppedImage, path: &Path) -> Result<(), ExportError> {
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(image.width, image.height, image.rgba.clone())
            .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".to_string()))?;

    img.save(path)
        .map_err(|e| ExportError::Encode(e.to_string()))?;

    tracing::info!(path = %path.display(), "saved cropped PNG");
    Ok(())
}
