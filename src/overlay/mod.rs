mod selection;
pub(crate) use selection::{SelectionState, normalize_rect};

use iced::widget::{canvas, container, image, stack};
use iced::{keyboard, Color, Element, Length, Point, Rectangle, Size, Task as IcedTask, Theme};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;

use crate::config::{self, Config};
use crate::export::{self, crop_selection};
use crate::types::FrameBuffer;

/// State shared across all layer-shell surfaces.
struct OverlayState {
    /// Raw captured frames in output order — used for cropping on export.
    raw_frames: Vec<FrameBuffer>,
    /// Frozen frames as image handles in output order — used for display.
    handles: Vec<image::Handle>,
    /// Windows assigned so far: window::Id → index into raw_frames/handles.
    window_frame_idx: std::collections::HashMap<iced::window::Id, usize>,
    /// Logical canvas size per window — used to compute HiDPI scale factor at crop time.
    surface_logical_size: std::collections::HashMap<iced::window::Id, (f32, f32)>,
    /// Global selection state — shared across all surfaces.
    selection: SelectionState,
    /// Current cursor position on the active window.
    cursor_pos: iced::Point,
    /// Which window owns the current selection (set on MousePressed).
    active_window: Option<iced::window::Id>,
}

/// Messages for the overlay daemon.
///
/// `#[to_layer_message(multi)]` generates `TryInto<LayershellCustomActionsWithId>`
/// (required by the `daemon` build pattern) plus multi-window layer-shell variants.
/// The catch-all arm in `update` handles those generated variants.
#[to_layer_message(multi)]
#[derive(Debug, Clone)]
enum Message {
    /// Escape key — exits if Idle, resets to Idle if Drawing or Selected.
    EscapePressed,
    /// Left mouse button pressed — captured via event subscription.
    MousePressed(iced::window::Id),
    /// Left mouse button released — captured via event subscription.
    MouseReleased,
    /// Cursor moved to a new position.
    CursorMoved(iced::window::Id, iced::Point),
    /// User clicked Copy in the toolbar.
    CopyRequested,
    /// User clicked Save in the toolbar.
    SaveRequested,
    /// Canvas reported its logical bounds for HiDPI scale computation.
    SurfaceBoundsKnown(iced::window::Id, f32, f32),
}

/// Canvas program that renders the dim overlay, selection rectangle, and placeholder toolbar.
struct SelectionCanvas<'a> {
    state: &'a OverlayState,
    window: iced::window::Id,
}

impl<'a> canvas::Program<Message> for SelectionCanvas<'a> {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        _event: canvas::Event,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        // Report logical bounds once per window so the export handler can compute
        // the HiDPI scale factor (physical_pixels / logical_pixels).
        let known = self.state.surface_logical_size.get(&self.window);
        let msg = if known != Some(&(bounds.width, bounds.height)) {
            Some(Message::SurfaceBoundsKnown(
                self.window,
                bounds.width,
                bounds.height,
            ))
        } else {
            None
        };
        (canvas::event::Status::Ignored, msg)
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // 1. Dim the entire overlay (~35% black).
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color { r: 0.0, g: 0.0, b: 0.0, a: 0.35 },
        );

        // 2. Only draw the selection on the active window.
        if self.state.active_window == Some(self.window) {
            let maybe_rect = match &self.state.selection {
                SelectionState::Drawing { start } => {
                    Some(normalize_rect(*start, self.state.cursor_pos))
                }
                SelectionState::Selected { rect } => Some(*rect),
                SelectionState::Idle => None,
            };

            if let Some(rect) = maybe_rect {
                // 2a. Dashed white border.
                let border = canvas::Path::rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, rect.height),
                );
                const DASH: &[f32] = &[6.0, 4.0];
                frame.stroke(
                    &border,
                    canvas::Stroke {
                        style: canvas::stroke::Style::Solid(Color::WHITE),
                        width: 1.5,
                        line_dash: canvas::LineDash {
                            segments: DASH,
                            offset: 0,
                        },
                        ..canvas::Stroke::default()
                    },
                );

                // 2b. Corner handles (5×5 white squares).
                let handle = Size::new(5.0, 5.0);
                for corner in [
                    Point::new(rect.x - 2.5, rect.y - 2.5),
                    Point::new(rect.x + rect.width - 2.5, rect.y - 2.5),
                    Point::new(rect.x - 2.5, rect.y + rect.height - 2.5),
                    Point::new(rect.x + rect.width - 2.5, rect.y + rect.height - 2.5),
                ] {
                    frame.fill_rectangle(corner, handle, Color::WHITE);
                }

                // 2c. Size label above the selection — white pill, bold black text, centred.
                let label = format!("{} × {}", rect.width as u32, rect.height as u32);
                let font_size = 13.0_f32;
                let pill_w = 90.0_f32;
                let pill_h = 22.0_f32;
                let pill_x = rect.x;
                let pill_y = rect.y - pill_h - 4.0;
                frame.fill_rectangle(
                    Point::new(pill_x, pill_y),
                    Size::new(pill_w, pill_h),
                    Color::WHITE,
                );
                frame.fill_text(canvas::Text {
                    content: label,
                    position: Point::new(pill_x + pill_w / 2.0, pill_y + pill_h / 2.0),
                    color: Color::BLACK,
                    size: iced::Pixels(font_size),
                    font: iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::Font::default()
                    },
                    horizontal_alignment: iced::alignment::Horizontal::Center,
                    vertical_alignment: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });

                // 3. Placeholder toolbar when Selected.
                if matches!(self.state.selection, SelectionState::Selected { .. }) {
                    let toolbar_w = 120.0_f32;
                    let toolbar_h = 32.0_f32;
                    let toolbar_x = rect.x + (rect.width - toolbar_w) / 2.0;
                    let toolbar_y = if rect.y + rect.height + 8.0 + toolbar_h < bounds.height {
                        rect.y + rect.height + 8.0
                    } else {
                        rect.y - 8.0 - toolbar_h
                    };

                    // Toolbar background.
                    frame.fill_rectangle(
                        Point::new(toolbar_x, toolbar_y),
                        Size::new(toolbar_w, toolbar_h),
                        Color { r: 0.15, g: 0.15, b: 0.15, a: 0.95 },
                    );

                    // Active toolbar labels (white).
                    for (i, label) in ["Copy", "Save"].iter().enumerate() {
                        frame.fill_text(canvas::Text {
                            content: label.to_string(),
                            position: Point::new(
                                toolbar_x + 15.0 + i as f32 * 55.0,
                                toolbar_y + 9.0,
                            ),
                            color: Color::WHITE,
                            size: iced::Pixels(13.0),
                            ..canvas::Text::default()
                        });
                    }
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &(),
        _bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        if self.state.active_window == Some(self.window) {
            match self.state.selection {
                SelectionState::Selected { .. } => iced::mouse::Interaction::default(),
                _ => iced::mouse::Interaction::Crosshair,
            }
        } else {
            iced::mouse::Interaction::Crosshair
        }
    }
}

/// Compute the HiDPI scale factor for a window.
///
/// Returns `physical_width / logical_width`. Falls back to 1.0 if the logical
/// size hasn't been reported yet (selection before first canvas event — rare).
fn scale_factor(
    frame: &FrameBuffer,
    surface_logical_size: &std::collections::HashMap<iced::window::Id, (f32, f32)>,
    window: iced::window::Id,
) -> f32 {
    if let Some(&(logical_w, _)) = surface_logical_size.get(&window) {
        if logical_w > 0.0 {
            return frame.width as f32 / logical_w;
        }
    }
    1.0
}

fn overlay_view(
    state: &OverlayState,
    window: iced::window::Id,
) -> Element<'_, Message, Theme, iced::Renderer> {
    // Assign frame index on first CursorMoved; fall back to frame 0 before any cursor event.
    let frame_idx = state.window_frame_idx.get(&window).copied().unwrap_or(0);
    let handle = state
        .handles
        .get(frame_idx)
        .or_else(|| state.handles.first())
        .cloned()
        .unwrap_or_else(|| state.handles[0].clone());

    let frozen = image::Image::new(handle)
        .width(Length::Fill)
        .height(Length::Fill);

    let selection_canvas = canvas(SelectionCanvas { state, window })
        .width(Length::Fill)
        .height(Length::Fill);

    container(stack![frozen, selection_canvas])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Display frozen frames as fullscreen layer-shell overlays on all outputs.
///
/// Uses `StartMode::AllScreens` via the `daemon` build pattern (required for
/// multi-output — `Application::run` asserts against `AllScreens`).
/// Closes all surfaces on Escape.
///
/// Returns `Ok(())` immediately if `frames` is empty.
///
/// # Errors
/// Returns an error if `iced_layershell` fails to initialize.
///
/// # Example
/// ```no_run
/// use cosmic_shot::overlay;
/// use cosmic_shot::types::{FrameBuffer, PixelFormat};
/// overlay::run(vec![]).unwrap();
/// ```
pub fn run(frames: Vec<FrameBuffer>) -> anyhow::Result<()> {
    if frames.is_empty() {
        return Ok(());
    }

    // Keep raw frames for export cropping; build image handles for display.
    let raw_frames = frames;
    let handles: Vec<image::Handle> = raw_frames
        .iter()
        .map(|f| {
            // INVARIANT: data was read as pool.mmap()[..stride*height] in
            // capture_one_output, so data.len() == stride * height always holds.
            let rgba = f.to_rgba().expect("captured frame data is well-formed");
            image::Handle::from_rgba(f.width, f.height, rgba)
        })
        .collect();

    let layer_settings = LayerShellSettings {
        layer: Layer::Overlay,
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        exclusive_zone: -1,
        keyboard_interactivity: KeyboardInteractivity::Exclusive,
        start_mode: StartMode::AllScreens,
        size: Some((0, 0)), // fill output when all edges are anchored
        ..Default::default()
    };

    daemon(
        |_state: &OverlayState| "cosmic-shot".to_string(),
        |state: &mut OverlayState, message: Message| -> IcedTask<Message> {
            match message {
                Message::CursorMoved(id, pos) => {
                    // Assign a frame index to this window on first seen, in order.
                    if !state.window_frame_idx.contains_key(&id) {
                        let next_idx = state.window_frame_idx.len();
                        let clamped = next_idx.min(state.handles.len().saturating_sub(1));
                        state.window_frame_idx.insert(id, clamped);
                    }
                    // Only track cursor on the active window (or any window if Idle).
                    if state.active_window.is_none() || state.active_window == Some(id) {
                        state.cursor_pos = pos;
                    }
                    IcedTask::none()
                }
                Message::MousePressed(id) => {
                    // If a selection is active on this window, check if the click
                    // lands on a toolbar button before starting a new selection.
                    if let SelectionState::Selected { rect } = &state.selection {
                        if state.active_window == Some(id) {
                            let toolbar_w = 120.0_f32;
                            let toolbar_h = 32.0_f32;
                            let toolbar_x = rect.x + (rect.width - toolbar_w) / 2.0;
                            // Mirror the draw() toolbar placement logic exactly.
                            // Use the raw frame height as the surface height (layer-shell fills the output).
                            let surface_h = state.window_frame_idx
                                .get(&id)
                                .and_then(|&idx| state.raw_frames.get(idx))
                                .map(|f| f.height as f32)
                                .unwrap_or(f32::MAX);
                            let toolbar_y = if rect.y + rect.height + 8.0 + toolbar_h < surface_h {
                                rect.y + rect.height + 8.0
                            } else {
                                rect.y - 8.0 - toolbar_h
                            };

                            let click = state.cursor_pos;
                            let copy_rect = Rectangle {
                                x: toolbar_x,
                                y: toolbar_y,
                                width: toolbar_w / 2.0,
                                height: toolbar_h,
                            };
                            let save_rect = Rectangle {
                                x: toolbar_x + toolbar_w / 2.0,
                                y: toolbar_y,
                                width: toolbar_w / 2.0,
                                height: toolbar_h,
                            };
                            if copy_rect.contains(click) {
                                return IcedTask::done(Message::CopyRequested);
                            }
                            if save_rect.contains(click) {
                                return IcedTask::done(Message::SaveRequested);
                            }
                        }
                    }
                    // Default: start a new selection — cancels any existing rect.
                    state.active_window = Some(id);
                    state.selection = SelectionState::Drawing { start: state.cursor_pos };
                    IcedTask::none()
                }
                Message::MouseReleased => {
                    if let SelectionState::Drawing { start } = state.selection {
                        state.selection = SelectionState::Selected {
                            rect: normalize_rect(start, state.cursor_pos),
                        };
                    }
                    IcedTask::none()
                }
                Message::EscapePressed => {
                    if matches!(state.selection, SelectionState::Idle) {
                        iced::exit()
                    } else {
                        state.selection = SelectionState::Idle;
                        state.active_window = None;
                        IcedTask::none()
                    }
                }
                Message::CopyRequested => {
                    if let SelectionState::Selected { rect } = &state.selection {
                        if let Some(window_id) = state.active_window {
                            let frame_idx = state.window_frame_idx
                                .get(&window_id)
                                .copied()
                                .unwrap_or(0);
                            if let Some(frame) = state.raw_frames.get(frame_idx) {
                                let scale = scale_factor(frame, &state.surface_logical_size, window_id);
                                match crop_selection(
                                    frame,
                                    (rect.x * scale) as u32,
                                    (rect.y * scale) as u32,
                                    (rect.width * scale) as u32,
                                    (rect.height * scale) as u32,
                                ) {
                                    Ok(cropped) => {
                                        if let Err(e) = export::copy_to_clipboard(&cropped) {
                                            tracing::error!(%e, "clipboard copy failed");
                                        }
                                    }
                                    Err(e) => tracing::error!(%e, "crop failed"),
                                }
                            }
                        }
                    }
                    iced::exit()
                }
                Message::SaveRequested => {
                    if let SelectionState::Selected { rect } = &state.selection {
                        if let Some(window_id) = state.active_window {
                            let frame_idx = state.window_frame_idx
                                .get(&window_id)
                                .copied()
                                .unwrap_or(0);
                            if let Some(frame) = state.raw_frames.get(frame_idx) {
                                let scale = scale_factor(frame, &state.surface_logical_size, window_id);
                                match crop_selection(
                                    frame,
                                    (rect.x * scale) as u32,
                                    (rect.y * scale) as u32,
                                    (rect.width * scale) as u32,
                                    (rect.height * scale) as u32,
                                ) {
                                    Ok(cropped) => {
                                        let cfg = Config::load();
                                        let dir = cfg.resolved_save_dir();
                                        if let Err(e) = std::fs::create_dir_all(&dir) {
                                            tracing::error!(%e, "failed to create save directory");
                                        } else {
                                            let path = dir.join(config::screenshot_filename());
                                            if let Err(e) = export::save_cropped_png(&cropped, &path) {
                                                tracing::error!(%e, "save failed");
                                            }
                                        }
                                    }
                                    Err(e) => tracing::error!(%e, "crop failed"),
                                }
                            }
                        }
                    }
                    iced::exit()
                }
                Message::SurfaceBoundsKnown(id, w, h) => {
                    state.surface_logical_size.insert(id, (w, h));
                    IcedTask::none()
                }
                _ => IcedTask::none(),
            }
        },
        overlay_view,
        |_state: &mut OverlayState, _id| {},
    )
    .subscription(|_state| {
        use iced::event::listen_with;
        use iced::{mouse, Event};

        iced::Subscription::batch([
            keyboard::on_key_press(|key, _mods| match key {
                keyboard::Key::Named(keyboard::key::Named::Escape) => {
                    Some(Message::EscapePressed)
                }
                _ => None,
            }),
            listen_with(|event, _status, id| match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Message::CursorMoved(id, position))
                }
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    Some(Message::MousePressed(id))
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::MouseReleased)
                }
                _ => None,
            }),
        ])
    })
    .layer_settings(layer_settings)
    .run_with(move || (
        OverlayState {
            raw_frames,
            handles,
            window_frame_idx: std::collections::HashMap::new(),
            surface_logical_size: std::collections::HashMap::new(),
            selection: SelectionState::Idle,
            cursor_pos: iced::Point::ORIGIN,
            active_window: None,
        },
        IcedTask::none(),
    ))
    .map_err(|e| anyhow::anyhow!("iced_layershell error: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_with_empty_frames_returns_ok() {
        // Tests the early-return path — no compositor needed.
        let result = run(vec![]);
        assert!(result.is_ok());
    }
}
