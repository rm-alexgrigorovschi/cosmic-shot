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

    #[error("no outputs available on the compositor")]
    NoOutputs,

    #[error("compositor offered no supported pixel format (need Argb8888, Xrgb8888, Abgr8888, or Xbgr8888)")]
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

    // Run the fallible capture logic; always clean up protocol objects afterward.
    let result = capture_output_inner(&session, &qh, &mut event_queue, &mut state);

    session.destroy();
    source.destroy();
    source_manager.destroy();
    capture_manager.destroy();
    output.release();

    result
}

/// Inner capture logic for single-output, separated for guaranteed cleanup.
fn capture_output_inner(
    session: &wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    qh: &QueueHandle<CaptureState>,
    event_queue: &mut wayland_client::EventQueue<CaptureState>,
    state: &mut CaptureState,
) -> Result<FrameBuffer, CaptureError> {
    const MAX_ITERATIONS: u32 = 1000;

    // 5. Dispatch until session constraints are done.
    let mut iterations = 0u32;
    while !state.frame.constraints_done && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(state)?;
        iterations += 1;
    }

    // 6. Pick a supported pixel format.
    let format = state
        .frame
        .shm_formats
        .iter()
        .find(|f| {
            matches!(
                f,
                wl_shm::Format::Argb8888
                    | wl_shm::Format::Xrgb8888
                    | wl_shm::Format::Abgr8888
                    | wl_shm::Format::Xbgr8888
            )
        })
        .copied()
        .ok_or(CaptureError::UnsupportedFormat)?;

    let width = state.frame.buffer_width;
    let height = state.frame.buffer_height;
    // INVARIANT: stride is width * 4 for all supported 32-bit formats; the
    // protocol does not provide stride, so we compute it from the known pixel width.
    let stride = width * 4;
    let pool_size = (stride as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| CaptureError::ShmPool("pool size overflow".into()))?;

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
        qh,
    );

    // 8. Create a frame, attach buffer, declare damage, and capture.
    let capture_frame = session.create_frame(qh, ());
    capture_frame.attach_buffer(&wl_buffer);
    capture_frame.damage_buffer(0, 0, width as i32, height as i32);
    capture_frame.capture();

    // 9. Wait for the frame to be ready.
    let mut iterations = 0u32;
    while !state.frame.ready && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(state)?;
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
        wl_shm::Format::Abgr8888 => PixelFormat::Abgr8888,
        wl_shm::Format::Xbgr8888 => PixelFormat::Xbgr8888,
        // INVARIANT: we filtered for exactly these four formats above.
        _ => return Err(CaptureError::UnsupportedFormat),
    };

    wl_buffer.destroy();
    capture_frame.destroy();

    tracing::info!(width, height, ?pixel_format, "frame captured");

    Ok(FrameBuffer {
        data,
        width,
        height,
        stride,
        format: pixel_format,
    })
}

/// Capture all available outputs and return their pixel data.
///
/// Opens a Wayland connection, enumerates all `wl_output` globals, and
/// captures each one in sequence using [`capture_one_output`].
///
/// # Errors
///
/// Returns [`CaptureError::NoOutputs`] if no `wl_output` globals are present.
/// Other variants are returned if any individual capture fails.
///
/// # Example
///
/// ```no_run
/// let frames = cosmic_shot::capture::capture_all_outputs().expect("capture failed");
/// println!("captured {} output(s)", frames.len());
/// ```
pub fn capture_all_outputs() -> Result<Vec<FrameBuffer>, CaptureError> {
    let conn =
        Connection::connect_to_env().map_err(|e| CaptureError::Connection(e.to_string()))?;
    let (globals, mut event_queue) = registry_queue_init::<CaptureState>(&conn)?;
    let qh = event_queue.handle();

    // Collect (name, advertised_version) of all wl_output globals.
    let output_entries: Vec<(u32, u32)> = globals.contents().with_list(|list| {
        list.iter()
            .filter(|g| g.interface == "wl_output")
            .map(|g| (g.name, g.version))
            .collect()
    });

    if output_entries.is_empty() {
        return Err(CaptureError::NoOutputs);
    }

    let shm = Shm::bind(&globals, &qh).map_err(|e| CaptureError::SctBind(e.to_string()))?;

    let capture_manager: ExtImageCopyCaptureManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .map_err(|_| CaptureError::NoScreencopy)?;

    let source_manager: ExtOutputImageCaptureSourceManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .map_err(|_| CaptureError::NoImageCaptureSource)?;

    let mut state = CaptureState {
        shm,
        frame: FrameState::default(),
    };

    let mut frames = Vec::with_capacity(output_entries.len());

    for (name, adv_version) in output_entries {
        // Bind this specific output by its global name via the underlying registry.
        // We cap at version 4 (the max our Dispatch impl handles) and floor at 3
        // (so .release() is available).
        let bind_version = adv_version.clamp(3, 4);
        let output: wl_output::WlOutput =
            globals.registry().bind(name, bind_version, &qh, ());

        // Reset per-capture state before each output.
        state.frame = FrameState::default();

        let frame = capture_one_output(
            &capture_manager,
            &source_manager,
            &output,
            &qh,
            &mut event_queue,
            &mut state,
        )?;

        output.release();
        frames.push(frame);
    }

    source_manager.destroy();
    capture_manager.destroy();

    Ok(frames)
}

/// Capture a single output using an already-initialized Wayland session.
fn capture_one_output(
    capture_manager: &ExtImageCopyCaptureManagerV1,
    source_manager: &ExtOutputImageCaptureSourceManagerV1,
    output: &wl_output::WlOutput,
    qh: &QueueHandle<CaptureState>,
    event_queue: &mut wayland_client::EventQueue<CaptureState>,
    state: &mut CaptureState,
) -> Result<FrameBuffer, CaptureError> {
    let source = source_manager.create_source(output, qh, ());
    let session = capture_manager.create_session(&source, Options::empty(), qh, ());

    let result = capture_one_output_inner(&session, qh, event_queue, state);

    // Always destroy protocol objects regardless of success/failure to avoid
    // compositor-side resource leaks.
    session.destroy();
    source.destroy();

    result
}

/// Inner capture logic separated so the caller can guarantee Wayland object cleanup.
fn capture_one_output_inner(
    session: &wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
    qh: &QueueHandle<CaptureState>,
    event_queue: &mut wayland_client::EventQueue<CaptureState>,
    state: &mut CaptureState,
) -> Result<FrameBuffer, CaptureError> {
    const MAX_ITERATIONS: u32 = 1000;
    let mut iterations = 0u32;
    while !state.frame.constraints_done && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(state)?;
        iterations += 1;
    }

    let format = state
        .frame
        .shm_formats
        .iter()
        .find(|f| {
            matches!(
                f,
                wl_shm::Format::Argb8888
                    | wl_shm::Format::Xrgb8888
                    | wl_shm::Format::Abgr8888
                    | wl_shm::Format::Xbgr8888
            )
        })
        .copied()
        .ok_or(CaptureError::UnsupportedFormat)?;

    let width = state.frame.buffer_width;
    let height = state.frame.buffer_height;
    // INVARIANT: stride is width * 4 for all supported 32-bit formats.
    let stride = width * 4;
    let pool_size = (stride as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| CaptureError::ShmPool("pool size overflow".into()))?;

    let mut pool =
        RawPool::new(pool_size, &state.shm).map_err(|e| CaptureError::ShmPool(e.to_string()))?;
    let wl_buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        format,
        (),
        qh,
    );

    let capture_frame = session.create_frame(qh, ());
    capture_frame.attach_buffer(&wl_buffer);
    capture_frame.damage_buffer(0, 0, width as i32, height as i32);
    capture_frame.capture();

    let mut iterations = 0u32;
    while !state.frame.ready && !state.frame.failed {
        if iterations >= MAX_ITERATIONS {
            return Err(CaptureError::Timeout);
        }
        event_queue.blocking_dispatch(state)?;
        iterations += 1;
    }

    if state.frame.failed {
        return Err(CaptureError::FrameFailed);
    }

    let data = pool.mmap()[..pool_size].to_vec();

    let pixel_format = match format {
        wl_shm::Format::Argb8888 => PixelFormat::Argb8888,
        wl_shm::Format::Xrgb8888 => PixelFormat::Xrgb8888,
        wl_shm::Format::Abgr8888 => PixelFormat::Abgr8888,
        wl_shm::Format::Xbgr8888 => PixelFormat::Xbgr8888,
        // INVARIANT: filtered for exactly these four formats above.
        _ => return Err(CaptureError::UnsupportedFormat),
    };

    wl_buffer.destroy();
    capture_frame.destroy();

    tracing::info!(width, height, ?pixel_format, "frame captured (multi-output)");

    Ok(FrameBuffer {
        data,
        width,
        height,
        stride,
        format: pixel_format,
    })
}
