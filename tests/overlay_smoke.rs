/// Smoke test — requires a running COSMIC Wayland compositor.
/// Run with: COSMIC_SHOT_INTEGRATION=1 cargo test -- --ignored
#[test]
#[ignore]
fn overlay_run_with_synthetic_frame_does_not_panic() {
    if std::env::var("COSMIC_SHOT_INTEGRATION").is_err() {
        return;
    }
    use cosmic_shot::types::{FrameBuffer, PixelFormat};
    let frame = FrameBuffer {
        data: vec![0xFF, 0x00, 0x00, 0xFF], // single red pixel, Abgr8888
        width: 1,
        height: 1,
        stride: 4,
        format: PixelFormat::Abgr8888,
    };
    let _ = cosmic_shot::overlay::run(vec![frame]);
}
