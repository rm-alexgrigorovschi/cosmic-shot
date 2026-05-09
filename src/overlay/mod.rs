use iced::widget::{container, image};
use iced::{keyboard, Element, Length, Task as IcedTask, Theme};
use iced_layershell::Application;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings, StartMode};
use iced_layershell::to_layer_message;

use crate::types::FrameBuffer;

struct App {
    image_handle: image::Handle,
}

/// Message enum for the overlay app.
///
/// The `#[to_layer_message]` macro generates the `TryInto<LayershellCustomActions>` impl
/// and adds layer-shell-specific variants automatically. Our update() must have a
/// catch-all `_ => IcedTask::none()` arm to handle those generated variants.
#[to_layer_message]
#[derive(Debug, Clone)]
enum Message {
    Close,
}

/// Display captured frames as a fullscreen layer-shell overlay on all outputs.
///
/// Returns `Ok(())` immediately when `frames` is empty.
///
/// # Limitations (M2)
/// Only the first frame is used for all surfaces. Multi-output support is planned.
pub fn run(frames: Vec<FrameBuffer>) -> anyhow::Result<()> {
    if frames.is_empty() {
        return Ok(());
    }

    // M2 limitation: use the first frame for all surfaces.
    let frame = frames.into_iter().next().expect("checked non-empty above");
    let width = frame.width;
    let height = frame.height;
    let rgba = frame.to_rgba();
    let handle = image::Handle::from_rgba(width, height, rgba);

    App::run(Settings {
        id: None,
        layer_settings: LayerShellSettings {
            layer: Layer::Overlay,
            anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
            exclusive_zone: -1,
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            start_mode: StartMode::AllScreens,
            size: Some((0, 0)),
            margin: (0, 0, 0, 0),
            events_transparent: false,
        },
        flags: handle,
        fonts: Vec::new(),
        default_font: iced::Font::default(),
        default_text_size: iced::Pixels(16.0),
        antialiasing: false,
        virtual_keyboard_support: None,
    })
    .map_err(|e| anyhow::anyhow!("iced_layershell error: {e}"))
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = image::Handle;

    fn new(handle: Self::Flags) -> (Self, IcedTask<Message>) {
        (App { image_handle: handle }, IcedTask::none())
    }

    fn namespace(&self) -> String {
        String::from("cosmic-shot")
    }

    fn update(&mut self, message: Message) -> IcedTask<Message> {
        match message {
            Message::Close => iced::exit(),
            // Catch-all for layer-shell variants injected by #[to_layer_message].
            _ => IcedTask::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        container(
            image::Image::new(self.image_handle.clone())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        keyboard::on_key_press(|key, _modifiers| match key {
            keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::Close),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_with_empty_frames_returns_ok() {
        // With no frames, run() should return Ok(()) immediately.
        // This tests the early-return path without needing a compositor.
        let result = run(vec![]);
        assert!(result.is_ok());
    }
}
