mod selection;
#[allow(unused_imports)]
pub(crate) use selection::{SelectionState, normalize_rect};

use iced::widget::{canvas, container, image, stack};
use iced::{keyboard, Color, Element, Length, Point, Rectangle, Size, Task as IcedTask, Theme};
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, StartMode};
use iced_layershell::to_layer_message;

use crate::types::FrameBuffer;

/// State shared across all layer-shell surfaces.
struct OverlayState {
    handle: image::Handle,
    selection: SelectionState,
    cursor_pos: iced::Point,
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
    MousePressed,
    /// Left mouse button released — captured via event subscription.
    MouseReleased,
    /// Cursor moved to a new position.
    CursorMoved(iced::Point),
}

/// Canvas program that renders the dim overlay, selection rectangle, and placeholder toolbar.
struct SelectionCanvas<'a> {
    state: &'a OverlayState,
}

impl<'a> canvas::Program<Message> for SelectionCanvas<'a> {
    type State = ();

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

        // 2. Draw selection rect if Drawing or Selected.
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

            // 2c. Size label above the selection.
            frame.fill_text(canvas::Text {
                content: format!("{} × {}", rect.width as u32, rect.height as u32),
                position: Point::new(rect.x, rect.y - 18.0),
                color: Color::WHITE,
                size: iced::Pixels(12.0),
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

                // Placeholder labels (greyed out — wired in M4).
                for (i, label) in ["Copy", "Save"].iter().enumerate() {
                    frame.fill_text(canvas::Text {
                        content: label.to_string(),
                        position: Point::new(
                            toolbar_x + 15.0 + i as f32 * 55.0,
                            toolbar_y + 9.0,
                        ),
                        color: Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 },
                        size: iced::Pixels(13.0),
                        ..canvas::Text::default()
                    });
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
        match self.state.selection {
            SelectionState::Selected { .. } => iced::mouse::Interaction::default(),
            _ => iced::mouse::Interaction::Crosshair,
        }
    }
}

fn overlay_view(
    state: &OverlayState,
    _window: iced::window::Id,
) -> Element<'_, Message, Theme, iced::Renderer> {
    let frozen = image::Image::new(state.handle.clone())
        .width(Length::Fill)
        .height(Length::Fill);

    let selection_canvas = canvas(SelectionCanvas { state })
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

    // M2: show the first frame on all outputs.
    // TODO(M3): map each window::Id to its per-output frame.
    let frame = frames.into_iter().next().unwrap();
    // INVARIANT: frames was non-empty, so next() is always Some.
    let rgba = frame.to_rgba();
    let handle = image::Handle::from_rgba(frame.width, frame.height, rgba);

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
                Message::CursorMoved(pos) => {
                    state.cursor_pos = pos;
                    IcedTask::none()
                }
                Message::MousePressed => {
                    if matches!(state.selection, SelectionState::Idle) {
                        state.selection = SelectionState::Drawing { start: state.cursor_pos };
                    }
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
                Message::EscapePressed => match state.selection {
                    SelectionState::Idle => iced::exit(),
                    _ => {
                        state.selection = SelectionState::Idle;
                        IcedTask::none()
                    }
                },
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
            listen_with(|event, _status, _id| match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    Some(Message::CursorMoved(position))
                }
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    Some(Message::MousePressed)
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
            handle,
            selection: SelectionState::Idle,
            cursor_pos: iced::Point::ORIGIN,
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
