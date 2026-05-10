# Project Conventions for cosmic-shot

## Rust General
- Edition 2021, MSRV 1.75
- Format with `rustfmt` defaults — no custom config
- `cargo clippy -- -D warnings` must pass before any commit
- Prefer `?` over `.unwrap()` / `.expect()` in non-test code
- `unwrap()` is acceptable only when invariants are provably upheld; add a `// SAFETY:` or `// INVARIANT:` comment explaining why
- No `panic!()` in library code paths — return `Result` instead
- Public APIs need doc comments with at least one example for non-trivial functions

## Error Handling
- Use `thiserror` for library-style typed errors (in modules)
- Use `anyhow` only at the binary entry point (main.rs)
- Never swallow errors silently — log via `tracing` or propagate
- Errors crossing the Wayland boundary should include protocol context

## Async / Concurrency
- Tokio with `rt` + `macros` features — `current_thread` runtime only; no `rt-multi-thread`
- Avoid `tokio::spawn` unless there's a real reason; prefer single-task event loops
- No `block_on` inside async contexts
- Channels: `tokio::sync::mpsc` for async, `crossbeam` if we need sync

## Wayland-specific
- Never block the Wayland event loop with synchronous I/O (file writes, encoding)
- All image encoding happens off the event loop, in a worker task
- Buffer management: prefer `wl_shm` for v1 simplicity; revisit dmabuf later if perf demands it
- Always check protocol support at startup; fail loudly if required protocols aren't available
- Using `ext-image-copy-capture-v1` (COSMIC dropped `wlr-screencopy-unstable-v1` entirely)
- Protocol code is isolated in `capture/screencopy.rs` — all Dispatch impls live there
- COSMIC's compositor offers `Abgr8888`/`Xbgr8888` pixel formats (not `Argb8888`/`Xrgb8888`); both families are supported
- M1 uses synchronous `blocking_dispatch` for single-shot capture; this is acceptable for the short-lived capture phase but must move to async when the event loop becomes persistent

## iced / iced_layershell
- `iced` pinned to `0.13.1`; `iced_layershell` pinned to `0.13.7` (0.18.x requires iced 0.14)
- Multi-output overlay requires `build_pattern::daemon` + `StartMode::AllScreens` — `Application::run` panics with AllScreens
- Use `#[to_layer_message(multi)]` (not plain `#[to_layer_message]`) for the daemon pattern; generates `TryInto<LayershellCustomActionsWithId>`
- View closure for daemon fails HRTB lifetime bound — use a named function `fn overlay_view(...)`
- `listen_with` closure takes 3 args: `(event, _status, id: window::Id)`
- Use `iced::exit()` to close the application (not `window::close`)
- `canvas::Text` font weight: `iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }`
- `canvas::Text` alignment: `horizontal_alignment: iced::alignment::Horizontal::Center`, `vertical_alignment: iced::alignment::Vertical::Center`
- `LineDash` segments must be `const &[f32]` to avoid lifetime issues in `draw()`
- Window creation order in `iced_layershell` does NOT match `wl_output` enumeration order — frames must be matched to surfaces by resolution (aspect ratio), not by index. Known limitation: identical-resolution multi-monitor setups may get swapped frames (cosmetic only, crop still works on the active screen)
- `window::Event::Opened { size }` fires at surface creation with logical size — use it to match frames to surfaces immediately
- `canvas::Program::update()` fires only on mouse/touch events, NOT on every render — don't rely on it for init-time work
- Clipboard on Wayland: `arboard` spawns a thread (not a process) to serve clipboard; thread dies with `iced::exit()`. Use `wl-copy` subprocess instead — it forks and persists after parent exit

## Performance Discipline
- No allocations in the hot path during selection (mouse-move handlers)
- Image encoding is lazy — only on user export action, never during capture or selection
- Profile with `cargo flamegraph` before optimizing; no premature optimization
- Benchmarks for capture latency live in `benches/` using `criterion`

## Output Formats
- JPEG export requires RGBA→RGB conversion (drop alpha channel) — JPEG has no alpha support
- WebP in `image` 0.25 is lossless only; quality parameter is ignored; lossy WebP needs a newer crate version
- Clipboard always uses PNG regardless of configured format — paste targets have spotty JPEG/WebP support
- `image::codecs::jpeg::JpegEncoder::new_with_quality` and `image::codecs::webp::WebPEncoder::new_with_quality` are the encoding APIs

## CLI / UX Lessons
- cosmic-shot is launched via keyboard shortcut — stdout is invisible to the user
- Any user-visible feedback (countdowns, notifications, confirmations) must use desktop notifications (`notify-send` subprocess) or an on-screen iced overlay, never stdout/stderr
- `notify-send` subprocess follows the same pattern as `wl-copy`: spawn, pipe, fork-persist

## Architecture Boundaries
- Three-phase pipeline (freeze / select / export) maps to three modules — they don't share mutable state
- `capture/` — Wayland protocol code, knows nothing about UI
- `overlay/` — iced UI, knows nothing about Wayland protocols (receives a frame buffer)
- `export/` — encoding, clipboard, disk I/O, knows nothing about capture or UI
- Communication between phases via typed message channels, not shared
