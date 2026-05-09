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
}

/// Save a captured frame as a PNG file.
pub fn save_png(frame: &FrameBuffer, path: &Path) -> Result<(), ExportError> {
    let rgba = frame.to_rgba()?;

    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(frame.width, frame.height, rgba)
            .ok_or_else(|| ExportError::Encode("pixel buffer size mismatch".to_string()))?;

    img.save(path).map_err(|e| ExportError::Encode(e.to_string()))?;

    tracing::info!(path = %path.display(), "saved PNG");
    Ok(())
}
