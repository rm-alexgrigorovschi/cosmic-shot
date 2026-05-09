use crate::types::FrameBuffer;

/// Cropped image in RGBA byte order.
#[derive(Debug)]
pub struct CroppedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Crop a rectangular region from a `FrameBuffer`.
///
/// The region is specified in pixel coordinates and clamped to the frame's
/// bounds. The output is in RGBA8 byte order (same as `FrameBuffer::to_rgba`).
///
/// # Errors
///
/// Returns [`super::ExportError::Conversion`] if the frame data is malformed.
pub fn crop_selection(
    frame: &FrameBuffer,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<CroppedImage, super::ExportError> {
    let rgba = frame.to_rgba()?;

    // Clamp the crop region to the frame bounds.
    let x = x.min(frame.width);
    let y = y.min(frame.height);
    let w = width.min(frame.width.saturating_sub(x));
    let h = height.min(frame.height.saturating_sub(y));

    if w == 0 || h == 0 {
        return Ok(CroppedImage {
            rgba: Vec::new(),
            width: 0,
            height: 0,
        });
    }

    let src_stride = frame.width as usize * 4;
    let mut cropped = Vec::with_capacity(w as usize * h as usize * 4);

    for row in 0..h {
        let src_row_start = (y + row) as usize * src_stride + x as usize * 4;
        let src_row_end = src_row_start + w as usize * 4;
        cropped.extend_from_slice(&rgba[src_row_start..src_row_end]);
    }

    Ok(CroppedImage {
        rgba: cropped,
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PixelFormat;

    /// Create a 4×4 Abgr8888 frame where pixel (x,y) has value [x, y, 0, 255].
    fn make_4x4_frame() -> FrameBuffer {
        let mut data = Vec::with_capacity(4 * 4 * 4);
        for y in 0..4u8 {
            for x in 0..4u8 {
                data.extend_from_slice(&[x, y, 0, 255]);
            }
        }
        FrameBuffer {
            data,
            width: 4,
            height: 4,
            stride: 16, // 4 pixels * 4 bytes
            format: PixelFormat::Abgr8888,
        }
    }

    #[test]
    fn crop_selection_extracts_correct_pixels() {
        let frame = make_4x4_frame();
        // Crop a 2×2 region starting at (1, 1).
        let cropped = crop_selection(&frame, 1, 1, 2, 2).unwrap();
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        // Abgr8888: memory is already [R,G,B,A], so to_rgba() is identity.
        // Pixel (1,1) → [1, 1, 0, 255], pixel (2,1) → [2, 1, 0, 255]
        // Pixel (1,2) → [1, 2, 0, 255], pixel (2,2) → [2, 2, 0, 255]
        assert_eq!(
            cropped.rgba,
            vec![
                1, 1, 0, 255, 2, 1, 0, 255, // row y=1
                1, 2, 0, 255, 2, 2, 0, 255, // row y=2
            ]
        );
    }

    #[test]
    fn crop_selection_clamps_to_frame_bounds() {
        let frame = make_4x4_frame();
        // Request extends beyond frame: x=3, width=5 → clamped to width=1
        let cropped = crop_selection(&frame, 3, 0, 5, 2).unwrap();
        assert_eq!(cropped.width, 1);
        assert_eq!(cropped.height, 2);
        // Pixel (3,0) → [3, 0, 0, 255], pixel (3,1) → [3, 1, 0, 255]
        assert_eq!(cropped.rgba, vec![3, 0, 0, 255, 3, 1, 0, 255]);
    }

    #[test]
    fn crop_selection_fully_outside_returns_empty() {
        let frame = make_4x4_frame();
        // Start beyond frame bounds.
        let cropped = crop_selection(&frame, 10, 10, 5, 5).unwrap();
        assert_eq!(cropped.width, 0);
        assert_eq!(cropped.height, 0);
        assert!(cropped.rgba.is_empty());
    }
}
