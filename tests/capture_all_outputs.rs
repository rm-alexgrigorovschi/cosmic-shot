/// Integration test — requires a running COSMIC Wayland compositor.
/// Run with: COSMIC_SHOT_INTEGRATION=1 cargo test -- --ignored
#[test]
#[ignore]
fn capture_all_outputs_returns_at_least_one_frame() {
    if std::env::var("COSMIC_SHOT_INTEGRATION").is_err() {
        return;
    }
    let frames = cosmic_shot::capture::capture_all_outputs()
        .expect("capture_all_outputs failed");
    assert!(!frames.is_empty(), "expected at least one captured frame");
    let frame = &frames[0];
    assert!(frame.width > 0);
    assert!(frame.height > 0);
    assert_eq!(frame.data.len(), (frame.stride * frame.height) as usize);
}
