# cosmic-shot

A fast, native screenshot tool for Pop!_OS COSMIC DE, inspired by Shottr (macOS).

## Goals
- Sub-50ms freeze-to-overlay latency
- No xdg-desktop-portal round-trips
- Native COSMIC look and feel
- All configuration via `~/.config/cosmic-shot/config.toml` — no settings UI

## Architecture
Three-phase pipeline that must never block:
1. **Freeze** — ext-image-copy-capture-v1 grab → fullscreen overlay with frozen frame
2. **Select** — pure UI, no I/O
3. **Export** — encode + clipboard/disk only on user action

## Tech Stack
- Rust 2021, MSRV 1.75
- wayland-client + smithay-client-toolkit
- wayland-protocols (`ext-image-copy-capture-v1` — COSMIC dropped wlr-screencopy entirely)
- iced 0.13.1 + iced_layershell 0.13.7 for overlay UI
- image crate for encoding (lazy, on export only; PNG/JPEG/WebP)
- wl-copy subprocess for clipboard (arboard abandoned — thread dies with iced::exit)
- serde + toml for config at ~/.config/cosmic-shot/config.toml

## Out of Scope (v1)
- Flatpak (kills the low-latency story)
- X11 support
- OCR (feature flag for v2)
- Settings UI (config.toml only)
- Delay capture with visible countdown (stdout-only; invisible from keyboard shortcut)

## Packaging
- GitHub Actions: build binary + tarball + .deb via cargo-deb on tag push
- install.sh with remote download from GitHub Releases
- APT repo via GitHub Pages once there's demand

## Milestones

### Shipped
- **M1:** ext-image-copy-capture-v1 capture; saves `./capture.png`
- **M2:** Fullscreen layer-shell overlay (iced_layershell daemon, AllScreens); Escape closes
- **M3:** Selection rectangle — click-drag dashed rect, corner handles, size label, Escape resets
- **M4:** Clipboard (wl-copy) + file save; HiDPI crop scaling; config module
- **M5:** Global shortcut integration; `--print-shortcut` flag; .desktop file; install.sh
- **M6:** cargo-deb packaging; GitHub Actions release workflow; remote install.sh

### In Progress
- **M7:** Output format options — JPEG/WebP alongside PNG; `format`/`quality` in config ✅

### Planned
- **M8:** Annotation tools — draw arrows, add text, highlight regions; contextual toolbar
  near selection rectangle; tool selection via keyboard shortcuts; defaults in config
- **M9:** Tray / persistent mode — keep running, re-capture on shortcut; no Wayland
  re-handshake per capture; config: `persistent = true`
