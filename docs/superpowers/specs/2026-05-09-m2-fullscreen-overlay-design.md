# M2: Fullscreen Overlay — Design Spec

Date: 2026-05-09

## Goal

Replace the M1 borderless window with a proper fullscreen layer-shell overlay that:
- Covers every connected monitor with its frozen frame
- Sits above all other windows (panels, docks, apps)
- Closes on Escape
- Introduces Tokio (single-threaded) as the async runtime for the pipeline

## Decisions

| Question | Decision |
|---|---|
| Overlay mechanism | `zwlr_layer_shell_v1` via `iced-layershell` crate |
| Monitor coverage | All connected outputs |
| Async runtime | `tokio` with `flavor = "current_thread"` (rt feature only, no rt-multi-thread) |
| PNG dump from M1 | Removed (was scaffolding) |

## Architecture

### Pipeline

```
#[tokio::main(flavor = "current_thread")]
main()
  └─ tracing::init
  └─ capture::capture_all_outputs() -> Vec<FrameBuffer>
       (sequential blocking_dispatch per output, acceptable for short-lived capture phase)
  └─ overlay::run(frames: Vec<FrameBuffer>)
       (one iced-layershell surface per output, each showing its frozen frame)
```

### Module responsibilities

**`src/capture/mod.rs`**
- New function: `capture_all_outputs() -> Result<Vec<FrameBuffer>>`
- Enumerates all `wl_output` globals from the Wayland registry
- Runs existing `capture_output()` for each, collecting frames
- Returns error if any output fails to capture

**`src/overlay/mod.rs`**
- Rewritten to use `iced-layershell` instead of vanilla iced
- `run(frames: Vec<FrameBuffer>) -> anyhow::Result<()>`
- One layer-shell surface per frame/output
- Layer: `Overlay` (above everything)
- Anchors: all four edges (fills the output)
- Exclusive zone: `0`
- Keyboard interactivity: `OnDemand` (grabs keyboard for Escape)
- Displays frame as full-surface image widget
- Closes all surfaces on Escape

**`src/main.rs`**
- Becomes `#[tokio::main(flavor = "current_thread")]`
- Calls `capture_all_outputs()` then `overlay::run()`
- PNG export call removed

**`src/export/mod.rs`**
- Unchanged in M2

## Dependencies to add

- `tokio` with features `["rt"]` (no `rt-multi-thread`)
- `iced-layershell` — version must be pinned to match `iced 0.13`; verify compatibility at build time before writing logic

## Layer Shell Surface Settings

```
layer:             Overlay
anchor:            Top | Bottom | Left | Right
exclusive_zone:    0
keyboard_interactivity: OnDemand
```

## Error Handling

- If zero outputs are captured: return `Err` with a clear message ("no outputs found")
- If `iced-layershell` version is incompatible with `iced 0.13`: surfaces as a build error; resolve by pinning versions before writing logic
- Capture errors per-output: propagate as `CaptureError`, abort entire run (M2 is all-or-nothing)

## Testing Strategy (TDD)

### Unit tests (no compositor required)

| Test | Location | What it checks |
|---|---|---|
| `pixel_format_abgr8888` | `src/types.rs` | `to_rgba()` correct for `Abgr8888` |
| `pixel_format_xbgr8888` | `src/types.rs` | `to_rgba()` correct for `Xbgr8888`, alpha forced to 255 |
| `overlay_run_empty_frames` | `src/overlay/mod.rs` | `run(vec![])` returns `Ok(())` without panicking |

### Integration tests (require compositor, `#[ignore]` by default)

Run with `COSMIC_SHOT_INTEGRATION=1 cargo test -- --ignored`

| Test | Location | What it checks |
|---|---|---|
| `capture_all_outputs` | `tests/capture_all_outputs.rs` | Full pipeline: capture → non-empty `Vec<FrameBuffer>` |
| `overlay_smoke` | `tests/overlay_smoke.rs` | Overlay starts with a synthetic 1×1 frame without error |

### What is NOT tested

- Layer shell surface placement (compositor behavior)
- Visual correctness of the frozen frame (deferred to M3)

## TDD Flow

For each unit:
1. Write the failing test
2. Implement the minimum code to make it pass
3. `cargo clippy -- -D warnings` must pass
4. Wire up to integration point

## Out of Scope for M2

- Output picker / active-output-only mode
- Selection rectangle (M3)
- Clipboard / file save (M4)
- Parallel capture across outputs
