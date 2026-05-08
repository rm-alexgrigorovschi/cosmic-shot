/// Pixel format of a captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// ARGB8888 — bytes in memory: [B, G, R, A] (little-endian).
    Argb8888,
    /// XRGB8888 — bytes in memory: [B, G, R, X] (little-endian), alpha ignored.
    Xrgb8888,
}

/// Raw pixel data from a screen capture.
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
}

impl FrameBuffer {
    /// Convert the raw pixel data to RGBA8 byte order.
    ///
    /// Input is [B, G, R, A/X] per pixel (Wayland little-endian convention).
    /// Output is [R, G, B, A] per pixel (standard RGBA for image/iced).
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            let row_start = (y * self.stride) as usize;
            for x in 0..self.width {
                let offset = row_start + (x * 4) as usize;
                let b = self.data[offset];
                let g = self.data[offset + 1];
                let r = self.data[offset + 2];
                let a = match self.format {
                    PixelFormat::Argb8888 => self.data[offset + 3],
                    PixelFormat::Xrgb8888 => 255,
                };
                rgba.extend_from_slice(&[r, g, b, a]);
            }
        }
        rgba
    }
}
