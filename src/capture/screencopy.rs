//! Wayland Dispatch implementations for wlr-screencopy-unstable-v1.
//! Isolated in this module for future swap to ext-image-copy-capture-v1.

use wayland_client::protocol::{wl_buffer, wl_output};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use super::CaptureState;

/// Tracks the state of a single screencopy frame capture.
#[derive(Debug, Default)]
pub(crate) struct FrameState {
    /// Preferred shm format from the compositor.
    pub format: Option<wayland_client::protocol::wl_shm::Format>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    /// All buffer formats have been advertised.
    pub buffer_done: bool,
    /// Frame pixel data is ready to read.
    pub ready: bool,
    /// Capture failed.
    pub failed: bool,
}

impl Dispatch<ZwlrScreencopyManagerV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrScreencopyManagerV1,
        _event: <ZwlrScreencopyManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Manager has no events.
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use wayland_client::protocol::wl_shm::Format;

        match event {
            zwlr_screencopy_frame_v1::Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                // Accept only formats we can handle. Prefer the first supported one.
                if let (None, WEnum::Value(fmt)) = (&state.frame.format, format) {
                    if fmt == Format::Argb8888 || fmt == Format::Xrgb8888 {
                        state.frame.format = Some(fmt);
                        state.frame.width = width;
                        state.frame.height = height;
                        state.frame.stride = stride;
                    }
                }
            }
            zwlr_screencopy_frame_v1::Event::BufferDone => {
                state.frame.buffer_done = true;
            }
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                state.frame.ready = true;
            }
            zwlr_screencopy_frame_v1::Event::Failed => {
                state.frame.failed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_buffer::WlBuffer,
        _event: wl_buffer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // wl_buffer::Release — safe to ignore for single-shot capture.
    }
}

impl Dispatch<wl_output::WlOutput, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: wl_output::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // We only need the output proxy, not its events.
    }
}
