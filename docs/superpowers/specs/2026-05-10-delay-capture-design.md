# Delay Capture Design

## Overview

Add a configurable pre-capture delay to cosmic-shot, enabling capture of
tooltips, menus, and other transient UI states that disappear on click.
Delay is set via `config.toml` and/or a `--delay N` CLI flag. CLI takes
precedence. Stdout countdown provides feedback during the wait.

## Config

New field in `~/.config/cosmic-shot/config.toml`:

```toml
delay_secs = 3   # wait 3 seconds before capturing (default: 0)
```

- Type: `u64`, default `0` (no delay)
- Clamped to max 60 at load time; values above 60 log a warning and use 60

## CLI Flag

```
cosmic-shot --delay 5
```

- Parsed manually in `main.rs` (same pattern as `--print-shortcut`)
- Accepts a positive integer in seconds
- If `--delay` is given without a value or with a non-integer, print an error
  and exit 1
- `--delay 0` is valid and means no delay (same as the default)
- CLI value takes precedence over `config.toml`

## Countdown Feedback

When delay > 0, print to stdout once per second:

```
Capturing in 3...
Capturing in 2...
Capturing in 1...
```

Then proceed silently to `capture_all_outputs()`.

Implemented with `tokio::time::sleep(Duration::from_secs(1))` in a loop.

## Architecture

All delay logic lives in `main.rs` between tracing init and
`capture_all_outputs()`. The `capture/` module is not touched — it knows
nothing about delays.

```
main()
  ├── --print-shortcut? → exit (no delay, no Wayland)
  ├── parse --delay N → cli_delay: Option<u64>
  ├── Config::load() → cfg
  ├── resolve delay: cli_delay.unwrap_or(cfg.delay_secs).min(60)
  ├── if delay > 0: countdown loop
  ├── capture_all_outputs()
  └── overlay::run(frames)
```

## Tokio Feature

Add `time` to Tokio's features in `Cargo.toml`:

```toml
tokio = { version = "1", features = ["rt", "macros", "time"] }
```

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | Add `time` feature to `tokio` |
| `src/config.rs` | Add `delay_secs: u64` field, default 0, clamped to 60 |
| `src/main.rs` | Parse `--delay N` flag, resolve delay, countdown loop |

## Out of Scope

- Visual on-screen countdown overlay
- Desktop notification countdown
- Fractional seconds
- `--delay` without a value meaning "use config value" (redundant)
- Delays longer than 60 seconds

## Testing

- Unit tests in `config.rs`: `delay_secs` default is 0, TOML parse, clamping at 60
- Integration test in `tests/cli.rs`: `--delay` with non-integer exits 1; `--delay 0` behaves same as no flag
- Manual: `cosmic-shot --delay 3` shows countdown and captures after 3 seconds
