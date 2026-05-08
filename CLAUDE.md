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
- Tokio with `rt` feature only — we don't need multi-thread runtime for a screenshot app
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

## Performance Discipline
- The freeze phase has a budget: capture-to-overlay-visible must be <50ms on reference hardware
- No allocations in the hot path during selection (mouse-move handlers)
- Image encoding is lazy — only on user export action, never during capture or selection
- Profile with `cargo flamegraph` before optimizing; no premature optimization
- Benchmarks for capture latency live in `benches/` using `criterion`

## Architecture Boundaries
- Three-phase pipeline (freeze / select / export) maps to three modules — they don't share mutable state
- `capture/` — Wayland protocol code, knows nothing about UI
- `overlay/` — iced UI, knows nothing about Wayland protocols (receives a frame buffer)
- `export/` — encoding, clipboard, disk I/O, knows nothing about capture or UI
- Communication between phases via typed message channels, not shared
