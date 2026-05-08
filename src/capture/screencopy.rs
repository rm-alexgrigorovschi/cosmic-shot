//! Wayland Dispatch implementations for ext-image-copy-capture-v1.
//! Isolated in this module so the protocol can be swapped without touching the rest of the codebase.

use wayland_client::protocol::{wl_buffer, wl_output, wl_shm};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use super::CaptureState;

/// Tracks the state of a capture session and its current frame.
#[derive(Debug, Default)]
pub(crate) struct FrameState {
    /// Supported shm formats advertised by the compositor.
    pub shm_formats: Vec<wl_shm::Format>,
    /// Buffer width from session constraints.
    pub buffer_width: u32,
    /// Buffer height from session constraints.
    pub buffer_height: u32,
    /// All session constraints have been advertised.
    pub constraints_done: bool,
    /// Frame pixel data is ready to read.
    pub ready: bool,
    /// Capture failed.
    pub failed: bool,
}

impl Dispatch<ExtImageCopyCaptureManagerV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCopyCaptureManagerV1,
        _event: <ExtImageCopyCaptureManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Manager has no events.
    }
}

impl Dispatch<ExtOutputImageCaptureSourceManagerV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtOutputImageCaptureSourceManagerV1,
        _event: <ExtOutputImageCaptureSourceManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Source manager has no events.
    }
}

impl Dispatch<ExtImageCaptureSourceV1, ()> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtImageCaptureSourceV1,
        _event: <ExtImageCaptureSourceV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Source has no client-relevant events.
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_image_copy_capture_session_v1::Event::ShmFormat {
                format: WEnum::Value(fmt),
            } => {
                state.frame.shm_formats.push(fmt);
            }
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                state.frame.buffer_width = width;
                state.frame.buffer_height = height;
            }
            ext_image_copy_capture_session_v1::Event::Done => {
                state.frame.constraints_done = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for CaptureState {
    fn event(
        state: &mut Self,
        _proxy: &ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_image_copy_capture_frame_v1::Event::Ready => {
                state.frame.ready = true;
            }
            ext_image_copy_capture_frame_v1::Event::Failed { .. } => {
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
