# M2: Fullscreen Overlay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the M1 borderless iced window with a proper `zwlr_layer_shell_v1` fullscreen overlay that covers all monitors, sits above everything, and closes on Escape.

**Architecture:** Capture all outputs sequentially → pass all `FrameBuffer`s to `overlay::run()` → `iced_layershell` daemon with `StartMode::AllScreens` creates one layer-shell surface per output → each shows its frozen frame → Escape closes all. Tokio single-threaded runtime introduced in `main.rs`.

**Tech Stack:** `iced_layershell = "0.13.7"` (targets `iced ^0.13`), `tokio` with `rt` feature only, existing `wayland-client`/`smithay-client-toolkit` stack.

---

## File Map

| File | Change |
|---|---|
| `Cargo.toml` | Add `iced_layershell = "0.13.7"`, `tokio = { version = "1", features = ["rt"] }` |
| `src/main.rs` | Add `#[tokio::main(flavor = "current_thread")]`, call `capture_all_outputs()`, remove PNG export |
| `src/capture/mod.rs` | Add `capture_all_outputs() -> Result<Vec<FrameBuffer>>` |
| `src/overlay/mod.rs` | Rewrite: `iced_layershell` daemon with `StartMode::AllScreens`, `#[to_layer_message]` |
| `src/types.rs` | Add explicit unit tests for `to_rgba()` pixel format conversions |
| `tests/capture_all_outputs.rs` | New: integration test (ignored by default) |
| `tests/overlay_smoke.rs` | New: integration test (ignored by default) |

---

## Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add tokio and iced_layershell to Cargo.toml**

Open `Cargo.toml` and add under `[dependencies]`:

```toml
tokio = { version = "1", features = ["rt"] }
iced_layershell = { version = "0.13.7", default-features = false }
```

Also remove `wayland-protocols-wlr` if it's still there but unused (check with `cargo build` warnings).

- [ ] **Step 2: Verify it resolves without conflicts**

```bash
cargo fetch 2>&1
```

Expected: no error. If there's a version conflict between `iced_layershell 0.13.7` and `iced 0.13`, fix by ensuring `iced` is pinned to `"0.13"` (not `"0.13.1"`) in `Cargo.toml` so cargo can pick the compatible patch.

- [ ] **Step 3: Verify build still compiles**

```bash
cargo build 2>&1
```

Expected: builds successfully (existing code unchanged).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add tokio and iced_layershell dependencies"
```

---

## Task 2: Unit tests for pixel format conversions

**Files:**
- Modify: `src/types.rs`

These tests must be written first (TDD) — they should already pass since M1 implemented `to_rgba()`, but making them explicit guards against regressions.

- [ ] **Step 1: Write the tests**

Add to the bottom of `src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(format: PixelFormat, pixels: Vec<u8>) -> FrameBuffer {
        let width = (pixels.len() / 4) as u32;
        FrameBuffer {
            data: pixels,
            width,
            height: 1,
            stride: width * 4,
            format,
        }
    }

    #[test]
    fn pixel_format_abgr8888_converts_correctly() {
        // Memory layout for Abgr8888: [R, G, B, A] per pixel
        // to_rgba() should produce [R, G, B, A]
        let frame = make_frame(PixelFormat::Abgr8888, vec![0x11, 0x22, 0x33, 0xFF]);
        let rgba = frame.to_rgba();
        assert_eq!(rgba, vec![0x11, 0x22, 0x33, 0xFF]);
    }

    #[test]
    fn pixel_format_xbgr8888_forces_alpha_to_255() {
        // Memory layout for Xbgr8888: [R, G, B, X] per pixel
        // to_rgba() should produce [R, G, B, 255] (X replaced by 255)
        let frame = make_frame(PixelFormat::Xbgr8888, vec![0x11, 0x22, 0x33, 0x00]);
        let rgba = frame.to_rgba();
        assert_eq!(rgba, vec![0x11, 0x22, 0x33, 0xFF]);
    }

    #[test]
    fn pixel_format_argb8888_converts_correctly() {
        // Memory layout for Argb8888: [B, G, R, A] per pixel
        // to_rgba() should produce [R, G, B, A]
        let frame = make_frame(PixelFormat::Argb8888, vec![0x33, 0x22, 0x11, 0xFF]);
        let rgba = frame.to_rgba();
        assert_eq!(rgba, vec![0x11, 0x22, 0x33, 0xFF]);
    }

    #[test]
    fn pixel_format_xrgb8888_forces_alpha_to_255() {
        // Memory layout for Xrgb8888: [B, G, R, X] per pixel
        // to_rgba() should produce [R, G, B, 255]
        let frame = make_frame(PixelFormat::Xrgb8888, vec![0x33, 0x22, 0x11, 0x00]);
        let rgba = frame.to_rgba();
        assert_eq!(rgba, vec![0x11, 0x22, 0x33, 0xFF]);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p cosmic-shot types:: 2>&1
```

Expected: all 4 pass. If any fail, fix `to_rgba()` in `src/types.rs` until they do.

- [ ] **Step 3: Commit**

```bash
git add src/types.rs
git commit -m "test: add explicit unit tests for FrameBuffer pixel format conversions"
```

---

## Task 3: `capture_all_outputs()`

**Files:**
- Modify: `src/capture/mod.rs`

- [ ] **Step 1: Read the current capture module**

Read `src/capture/mod.rs` and `src/capture/screencopy.rs` to understand the current `capture_output(conn, globals, qh, output)` signature before changing anything.

- [ ] **Step 2: Write the failing integration test**

Create `tests/capture_all_outputs.rs`:

```rust
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
```

Also add to `src/lib.rs` (create it if it doesn't exist) so integration tests can access the crate:

```rust
pub mod capture;
pub mod export;
pub mod overlay;
pub mod types;
```

- [ ] **Step 3: Verify test file compiles (may fail at link if pub mod missing)**

```bash
cargo test --test capture_all_outputs 2>&1 | head -30
```

Expected: compile error about `cosmic_shot::capture` not being public — that's expected, we fix it next.

- [ ] **Step 4: Make `capture` module public and add `capture_all_outputs`**

In `src/capture/mod.rs`, add after the existing `capture_output` function:

```rust
/// Capture a frame from every connected Wayland output.
///
/// # Errors
/// Returns [`CaptureError`] if the Wayland connection fails or any output
/// cannot be captured.
///
/// # Example
/// ```no_run
/// let frames = cosmic_shot::capture::capture_all_outputs().unwrap();
/// assert!(!frames.is_empty());
/// ```
pub fn capture_all_outputs() -> Result<Vec<crate::types::FrameBuffer>, CaptureError> {
    use wayland_client::{globals::registry_queue_init, Connection};

    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<CaptureState>(&conn)?;
    let qh = event_queue.handle();

    // Collect all wl_output globals
    let outputs: Vec<_> = globals
        .contents()
        .clone_list()
        .into_iter()
        .filter(|g| g.interface == "wl_output")
        .collect();

    if outputs.is_empty() {
        return Err(CaptureError::NoOutputs);
    }

    let mut frames = Vec::with_capacity(outputs.len());
    for global in outputs {
        let output: wayland_client::protocol::wl_output::WlOutput =
            globals.bind(&qh, global.name, 3..=4)?;
        let frame = capture_output(&conn, &globals, &qh, &mut event_queue, &output)?;
        output.release();
        frames.push(frame);
    }

    Ok(frames)
}
```

Also add `NoOutputs` variant to `CaptureError`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    // ... existing variants ...
    #[error("no Wayland outputs found")]
    NoOutputs,
}
```

And add a `src/lib.rs`:

```rust
pub mod capture;
pub mod export;
pub mod overlay;
pub mod types;
```

- [ ] **Step 5: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: builds without errors. Fix any type mismatches (e.g., the exact `globals.contents().clone_list()` API may differ — check `wayland-client 0.31` docs; use `globals.contents().with_list(|list| list.to_vec())` or similar if needed).

- [ ] **Step 6: Run unit tests to make sure nothing broke**

```bash
cargo test --lib 2>&1
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/capture/mod.rs tests/capture_all_outputs.rs
git commit -m "feat: add capture_all_outputs() and pub lib.rs"
```

---

## Task 4: Rewrite `overlay/mod.rs` with `iced_layershell`

**Files:**
- Modify: `src/overlay/mod.rs`

This is the core of M2. We use `iced_layershell`'s `build_pattern::daemon` with `StartMode::AllScreens` so one surface is created per output automatically.

- [ ] **Step 1: Write the failing smoke test**

Create `tests/overlay_smoke.rs`:

```rust
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
    // This will open and immediately... we can't auto-close it in a test.
    // So this test is manual-verification only; just check it doesn't panic on startup.
    // In CI, skip via ignore.
    let _ = cosmic_shot::overlay::run(vec![frame]);
}
```

- [ ] **Step 2: Write the unit smoke test (no compositor)**

Add to `src/overlay/mod.rs` at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameBuffer, PixelFormat};

    #[test]
    fn run_with_empty_frames_returns_ok() {
        // With no frames, run() should return Ok(()) immediately.
        // This tests the early-return path without needing a compositor.
        let result = run(vec![]);
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 3: Run unit test to verify it fails**

```bash
cargo test overlay::tests 2>&1
```

Expected: compile error (overlay module not yet updated) or test failure. That's fine.

- [ ] **Step 4: Rewrite `src/overlay/mod.rs`**

Replace the entire contents with:

```rust
use iced::widget::{container, image};
use iced::{keyboard, Element, Length, Task as Command, Theme};
use iced_layershell::build_pattern::{daemon, MainSettings};
use iced_layershell::reexport::{
    Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, StartMode,
};
use iced_layershell::settings::{LayerShellSettings, Settings};
use iced_layershell::to_layer_message;
use iced_layershell::Application;

use crate::types::FrameBuffer;

/// Display frozen frames as fullscreen layer-shell overlays.
/// One surface is created per Wayland output via `StartMode::AllScreens`.
/// Closes all surfaces on Escape.
///
/// # Errors
/// Returns an error if `iced_layershell` fails to initialize.
///
/// # Example
/// ```no_run
/// use cosmic_shot::overlay;
/// use cosmic_shot::types::{FrameBuffer, PixelFormat};
/// let frame = FrameBuffer { data: vec![], width: 0, height: 0, stride: 0, format: PixelFormat::Abgr8888 };
/// overlay::run(vec![frame]).unwrap();
/// ```
pub fn run(frames: Vec<FrameBuffer>) -> anyhow::Result<()> {
    if frames.is_empty() {
        return Ok(());
    }

    // Use the first frame for all surfaces in M2.
    // In M3+, we will map each surface id to its per-output frame.
    let frame = frames.into_iter().next().unwrap();
    // INVARIANT: we checked frames.is_empty() above, so next() is always Some.
    let rgba = frame.to_rgba();
    let handle = image::Handle::from_rgba(frame.width, frame.height, rgba);

    Overlay::run(Settings {
        layer_settings: LayerShellSettings {
            size: Some((0, 0)), // 0,0 = fill the output when anchored to all edges
            exclusive_zone: -1, // don't reserve space; overlay covers everything
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            layer: Layer::Overlay,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            start_mode: StartMode::AllScreens,
            ..Default::default()
        },
        ..Default::default()
    })
    .run_with(move || (Overlay { handle: handle.clone() }, Command::none()))
    .map_err(|e| anyhow::anyhow!("iced_layershell error: {e}"))?;

    Ok(())
}

struct Overlay {
    handle: image::Handle,
}

#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Close,
}

impl Application for Overlay {
    type Message = Message;
    type Flags = ();
    type Theme = Theme;
    type Executor = iced::executor::Default;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        // INVARIANT: new() is never called directly; run_with provides the initial state.
        unreachable!("run_with is used instead of new()")
    }

    fn namespace(&self) -> String {
        "cosmic-shot".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Close => iced::exit(),
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        container(
            image::Image::new(self.handle.clone())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::on_key_press(|key, _mods| match key {
            keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Close),
            _ => None,
        })
    }

    fn style(&self, theme: &Self::Theme) -> iced_layershell::Appearance {
        iced_layershell::Appearance {
            background_color: iced::Color::BLACK,
            text_color: theme.palette().text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameBuffer, PixelFormat};

    #[test]
    fn run_with_empty_frames_returns_ok() {
        let result = run(vec![]);
        assert!(result.is_ok());
    }
}
```

**Note:** If `iced_layershell 0.13.7`'s `Application` trait or `Settings` struct have different field names, resolve them by checking `cargo doc --open` or reading the crate source. The most likely difference is `layer_settings` vs a flat struct — adjust accordingly.

- [ ] **Step 5: Build and fix compilation errors**

```bash
cargo build 2>&1
```

Work through any compile errors. Common issues:
- `Settings` field names differ → check `iced_layershell::settings` module
- `#[to_layer_message]` needs `Message::_` catch-arm → already included above as `_ => Command::none()`
- `run_with` signature differs → may need `Overlay::run(settings).run_with(|| ...)` shape

- [ ] **Step 6: Run unit tests**

```bash
cargo test --lib 2>&1
```

Expected: `overlay::tests::run_with_empty_frames_returns_ok` passes, all others pass.

- [ ] **Step 7: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Fix all warnings.

- [ ] **Step 8: Commit**

```bash
git add src/overlay/mod.rs tests/overlay_smoke.rs
git commit -m "feat: rewrite overlay with iced_layershell fullscreen layer-shell"
```

---

## Task 5: Update `main.rs` with Tokio and `capture_all_outputs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Read the current main.rs**

Read `src/main.rs` to see the current entry point before changing it.

- [ ] **Step 2: Rewrite `src/main.rs`**

Replace the entire contents with:

```rust
use anyhow::Context;
use tracing_subscriber::EnvFilter;

mod capture;
mod export;
mod overlay;
mod types;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("starting cosmic-shot");

    let frames = capture::capture_all_outputs()
        .context("failed to capture outputs")?;

    tracing::info!("captured {} output(s)", frames.len());

    overlay::run(frames).context("overlay error")?;

    Ok(())
}
```

**Note:** Remove the PNG export call and the old single-output capture call. The `mod` declarations here shadow the ones in `lib.rs` for the binary; that's fine since `lib.rs` exposes them for integration tests.

- [ ] **Step 3: Build**

```bash
cargo build --release 2>&1
```

Expected: clean build. Fix any errors.

- [ ] **Step 4: Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```

Fix all warnings.

- [ ] **Step 5: Run all unit tests**

```bash
cargo test --lib 2>&1
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire tokio runtime and capture_all_outputs into main"
```

---

## Task 6: Manual end-to-end verification

- [ ] **Step 1: Run the binary on COSMIC**

```bash
cargo run --release 2>&1
```

Expected:
- No panics or error messages in the terminal
- A fullscreen black overlay with the frozen desktop appears on all connected monitors
- The overlay sits above all other windows (panels, app windows, dock)
- Pressing Escape dismisses all overlays and the process exits cleanly

- [ ] **Step 2: Final clippy + test pass**

```bash
cargo clippy -- -D warnings && cargo test --lib 2>&1
```

Expected: zero warnings, all tests pass.

- [ ] **Step 3: Commit if any fixups were needed**

```bash
git add -A
git commit -m "fix: address manual verification issues in M2 overlay"
```

(Skip if nothing changed.)

---

## Task 7: Update design doc with any deviations

- [ ] **Step 1: Note any API differences discovered during implementation**

If `iced_layershell 0.13.7`'s API differed from the design (e.g., settings field names, multi-output handling), add a "Implementation Notes" section to `docs/superpowers/specs/2026-05-09-m2-fullscreen-overlay-design.md` documenting what changed and why.

- [ ] **Step 2: Commit the updated spec**

```bash
git add docs/superpowers/specs/2026-05-09-m2-fullscreen-overlay-design.md
git commit -m "docs: update M2 spec with implementation notes"
```

---

## Known Risks & Mitigations

| Risk | Mitigation |
|---|---|
| `iced_layershell 0.13.7` `Application` trait differs from vanilla iced | Read `cargo doc` for the crate before writing overlay code; the example in the docs is authoritative |
| `StartMode::AllScreens` not available in 0.13.x | Check enum variants in source; fallback: use `StartMode::Active` (single screen) for M2 and document as known limitation |
| `#[to_layer_message]` macro conflicts with `iced::exit()` | Use `Command::done(Message::Close)` pattern from the macro-generated variants if `iced::exit()` causes issues |
| Multi-output: same frame shown on all screens | Accepted for M2; per-output frame mapping deferred to M3 |
| `globals.contents().clone_list()` API missing | Use `globals.contents().with_list(|g| g.to_vec())` or iterate via `GlobalList` — check wayland-client 0.31 API |
