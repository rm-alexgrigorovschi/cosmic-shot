mod selection;
#[allow(unused_imports)]
pub(crate) use selection::{SelectionState, normalize_rect};

use iced::widget::{container, image};
use iced::{keyboard, Element, Length, Task as IcedTask, Theme};
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

fn overlay_view(
    state: &OverlayState,
    _window: iced::window::Id,
) -> Element<'_, Message, Theme, iced::Renderer> {
    container(
        image::Image::new(state.handle.clone())
            .width(Length::Fill)
            .height(Length::Fill),
    )
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
