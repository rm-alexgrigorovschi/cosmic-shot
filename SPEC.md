# cosmic-shot

A fast, native screenshot tool for Pop!_OS COSMIC DE, inspired by Shottr (macOS).

## Goals
- Sub-50ms freeze-to-overlay latency
- No xdg-desktop-portal round-trips
- Native COSMIC look and feel

## Architecture
Three-phase pipeline that must never block:
1. **Freeze** — wlr-screencopy grab → fullscreen overlay with frozen frame
2. **Select** — pure UI, no I/O
3. **Export** — encode + clipboard/disk only on user action

## Tech Stack
- Rust 2021, MSRV 1.75
- wayland-client + smithay-client-toolkit
- wayland-protocols-wlr (wlr-screencopy-unstable-v1)
- iced for overlay UI (matches COSMIC)
- image crate for encoding (lazy, on export only)
- arboard for clipboard
- serde + toml for config at ~/.config/cosmic-shot/config.toml

## Out of Scope (v1)
- Flatpak (kills the low-latency story)
- X11 support
- OCR (feature flag for v1.1)

## Packaging
- GitHub Actions: build binary + tarball + .deb via cargo-deb on tag
- install.sh for curl-pipe install
- APT repo via GitHub Pages once there's demand

## Milestones
- M1: wlr-screencopy capture working, dump frame to PNG
- M2: Fullscreen overlay showing frozen frame
- M3: Selection rectangle + crop on export
- M4: Clipboard + file save
- M5: Global shortcut integration
- M6: cargo-deb packaging
