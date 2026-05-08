mod screencopy;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_output, wl_registry, wl_shm};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1;
use wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::{
    ExtImageCopyCaptureManagerV1, Options,
};

use smithay_client_toolkit::delegate_shm;
use smithay_client_toolkit::shm::raw::RawPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};

use crate::types::{FrameBuffer, PixelFormat};

use self::screencopy::FrameState;

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("failed to connect to Wayland display: {0}")]
    Connection(String),

    #[error("Wayland global error: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),

    #[error("compositor does not support ext-image-copy-capture-v1")]
    NoScreencopy,

    #[error("compositor does not support ext-output-image-capture-source-v1")]
    NoImageCaptureSource,

    #[error("no active output found")]
    NoOutput,

    #[error("compositor offered no supported pixel format (need Argb8888 or Xrgb8888)")]
    UnsupportedFormat,

    #[error("capture frame failed")]
    FrameFailed,

    #[error("capture timed out after too many dispatch iterations")]
    Timeout,

    #[error("Wayland dispatch error: {0}")]
    Dispatch(#[from] wayland_client::DispatchError),

    #[error("shm pool creation failed: {0}")]
    ShmPool(String),

    #[error("Wayland backend error: {0}")]
    Backend(#[from] wayland_client::backend::WaylandError),

    #[error("SCTK global error: {0}")]
    SctGlobal(#[from] smithay_client_toolkit::error::GlobalError),

    #[error("SCTK bind error: {0}")]
    SctBind(String),
}

/// Internal state for the Wayland capture session.
pub(crate) struct CaptureState {
    pub shm: Shm,
    pub frame: FrameState,
}

impl ShmHandler for CaptureState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_shm!(CaptureState);

// Handle wl_registry events for globals we bind manually.
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for CaptureState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // SCTK's delegate_shm handles shm globals. We handle other globals
        // via GlobalList::bind() below, so no event processing needed here.
    }
}

/// Capture the first available output and return its pixel data.
pub fn capture_output() -> Result<FrameBuffer, CaptureError> {
    // 1. Connect to Wayland display.
    let conn =
        Connection::connect_to_env().map_err(|e| CaptureError::Connection(e.to_string()))?;
    let (globals, mut event_queue) = registry_queue_init::<CaptureState>(&conn)?;
    let qh = event_queue.handle();

    // 2. Bind required globals.
    let shm = Shm::bind(&globals, &qh).map_err(|e| CaptureError::SctBind(e.to_string()))?;

    let capture_manager: ExtImageCopyCaptureManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .map_err(|_| CaptureError::NoScreencopy)?;

    let source_manager: ExtOutputImageCaptureSourceManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .map_err(|_| CaptureError::NoImageCaptureSource)?;

    // Bind the first wl_output.
    let output: wl_output::WlOutput = globals
        .bind(&qh, 3..=4, ())
        .map_err(|_| CaptureError::NoOutput)?;

    let mut state = CaptureState {
        shm,
        frame: FrameState::default(),
    };

    // 3. Create capture source from the output.
    let source = source_manager.create_source(&output, &qh, ());

    // 4. Create a capture session from the source.
    let session = capture_manager.create_session(&source, Options::empty(), &qh, ());

    // 5. Dispatch until session constraints are done.
    let mut iterations = 0u32;
    const MAX_ITERATIONS: u32 = 1000;
    while !state.frame.constraints_done && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(&mut state)?;
        iterations += 1;
    }

    // 6. Pick a supported pixel format.
    let format = state
        .frame
        .shm_formats
        .iter()
        .find(|f| **f == wl_shm::Format::Argb8888 || **f == wl_shm::Format::Xrgb8888)
        .copied()
        .ok_or(CaptureError::UnsupportedFormat)?;

    let width = state.frame.buffer_width;
    let height = state.frame.buffer_height;
    // INVARIANT: stride is width * 4 for 32-bit ARGB/XRGB formats; the protocol
    // does not provide stride, so we compute it from the known pixel width.
    let stride = width * 4;
    let pool_size = (stride * height) as usize;

    // 7. Create an shm buffer.
    let mut pool =
        RawPool::new(pool_size, &state.shm).map_err(|e| CaptureError::ShmPool(e.to_string()))?;
    let wl_buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        format,
        (),
        &qh,
    );

    // 8. Create a frame, attach buffer, declare damage, and capture.
    let frame = session.create_frame(&qh, ());
    frame.attach_buffer(&wl_buffer);
    frame.damage_buffer(0, 0, width as i32, height as i32);
    frame.capture();

    // 9. Wait for the frame to be ready.
    let mut iterations = 0u32;
    while !state.frame.ready && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(&mut state)?;
        iterations += 1;
    }

    if state.frame.failed {
        return Err(CaptureError::FrameFailed);
    }

    // 10. Read pixel data from the shared memory pool.
    let data = pool.mmap()[..pool_size].to_vec();

    let pixel_format = match format {
        wl_shm::Format::Argb8888 => PixelFormat::Argb8888,
        wl_shm::Format::Xrgb8888 => PixelFormat::Xrgb8888,
        // INVARIANT: we filtered for exactly these two formats above.
        _ => return Err(CaptureError::UnsupportedFormat),
    };

    // 11. Clean up Wayland objects.
    wl_buffer.destroy();
    frame.destroy();
    session.destroy();
    source.destroy();
    source_manager.destroy();
    capture_manager.destroy();
    output.release();

    tracing::info!(width, height, ?pixel_format, "frame captured");

    Ok(FrameBuffer {
        data,
        width,
        height,
        stride,
        format: pixel_format,
    })
}
