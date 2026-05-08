mod screencopy;

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_output, wl_registry, wl_shm};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;

use smithay_client_toolkit::delegate_shm;
use smithay_client_toolkit::shm::raw::RawPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};

use crate::types::{FrameBuffer, PixelFormat};

use self::screencopy::FrameState;

/// Errors that can occur during screen capture.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum CaptureError {
    #[error("failed to connect to Wayland display: {0}")]
    Connection(String),

    #[error("Wayland global error: {0}")]
    Global(#[from] wayland_client::globals::GlobalError),

    #[error("compositor does not support wlr-screencopy-unstable-v1")]
    NoScreencopy,

    #[error("no active output found")]
    NoOutput,

    #[error("compositor offered no supported pixel format (need Argb8888 or Xrgb8888)")]
    UnsupportedFormat,

    #[error("screencopy frame capture failed")]
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
#[allow(dead_code)]
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

// Handle wl_registry events for globals we bind manually (wl_output, screencopy).
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
#[allow(dead_code)]
pub fn capture_output() -> Result<FrameBuffer, CaptureError> {
    // 1. Connect to Wayland display.
    let conn =
        Connection::connect_to_env().map_err(|e| CaptureError::Connection(e.to_string()))?;
    let (globals, mut event_queue) = registry_queue_init::<CaptureState>(&conn)?;
    let qh = event_queue.handle();

    // 2. Bind required globals.
    let shm = Shm::bind(&globals, &qh).map_err(|e| CaptureError::SctBind(e.to_string()))?;

    let screencopy_manager: ZwlrScreencopyManagerV1 = globals
        .bind(&qh, 1..=3, ())
        .map_err(|_| CaptureError::NoScreencopy)?;

    // Bind the first wl_output.
    let output: wl_output::WlOutput = globals
        .bind(&qh, 3..=4, ())
        .map_err(|_| CaptureError::NoOutput)?;

    let mut state = CaptureState {
        shm,
        frame: FrameState::default(),
    };

    // 3. Request a frame capture.
    let frame_proxy = screencopy_manager.capture_output(0, &output, &qh, ());

    // 4. Dispatch until the compositor tells us what buffer format it wants.
    // Blocking dispatch is acceptable here: this is a single-shot capture function,
    // not a persistent event loop. The Wayland connection is short-lived and dedicated
    // to this capture.
    let mut iterations = 0u32;
    const MAX_ITERATIONS: u32 = 1000;
    while !state.frame.buffer_done && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(&mut state)?;
        iterations += 1;
    }

    if state.frame.failed {
        return Err(CaptureError::FrameFailed);
    }

    let format = state.frame.format.ok_or(CaptureError::UnsupportedFormat)?;
    let width = state.frame.width;
    let height = state.frame.height;
    let stride = state.frame.stride;
    let pool_size = (stride * height) as usize;

    // 5. Create an shm buffer and tell the compositor to copy into it.
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

    frame_proxy.copy(&wl_buffer);

    // 6. Wait for the frame to be ready.
    // Blocking dispatch is acceptable here: this is a single-shot capture function,
    // not a persistent event loop. The Wayland connection is short-lived and dedicated
    // to this capture.
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

    // 7. Read pixel data from the shared memory pool.
    let data = pool.mmap()[..pool_size].to_vec();

    let pixel_format = match format {
        wl_shm::Format::Argb8888 => PixelFormat::Argb8888,
        wl_shm::Format::Xrgb8888 => PixelFormat::Xrgb8888,
        _ => return Err(CaptureError::UnsupportedFormat),
    };

    // 8. Clean up Wayland objects.
    wl_buffer.destroy();
    frame_proxy.destroy();
    screencopy_manager.destroy();
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
