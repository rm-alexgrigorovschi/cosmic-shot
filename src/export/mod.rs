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

/// Copy a cropped image to the system clipboard.
pub fn copy_to_clipboard(image: &CroppedImage) -> Result<(), ExportError> {
    use arboard::{Clipboard, ImageData};
    use std::borrow::Cow;

    let mut clipboard =
        Clipboard::new().map_err(|e| ExportError::Clipboard(e.to_string()))?;

    let img_data = ImageData {
        width: image.width as usize,
        height: image.height as usize,
        bytes: Cow::Borrowed(&image.rgba),
    };

    clipboard
        .set_image(img_data)
        .map_err(|e| ExportError::Clipboard(e.to_string()))?;

    tracing::info!("copied {}×{} image to clipboard", image.width, image.height);
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
