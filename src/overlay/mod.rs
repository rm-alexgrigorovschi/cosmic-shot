use iced::widget::{container, image};
use iced::{keyboard, window, Element, Length, Task as IcedTask, Theme};

use crate::types::FrameBuffer;

struct App {
    image_handle: image::Handle,
}

#[derive(Debug, Clone)]
enum Message {
    Close,
}

/// Display the captured frame in a window. Closes on Escape.
pub fn run(frame: FrameBuffer) -> anyhow::Result<()> {
    let width = frame.width as f32;
    let height = frame.height as f32;
    let rgba = frame.to_rgba();

    iced::application("cosmic-shot", App::update, App::view)
        .subscription(App::subscription)
        .window(window::Settings {
            size: iced::Size::new(width, height),
            decorations: false,
            ..Default::default()
        })
        .theme(|_| Theme::Dark)
        .run_with(move || {
            let handle = image::Handle::from_rgba(frame.width, frame.height, rgba);
            (App { image_handle: handle }, IcedTask::none())
        })
        .map_err(|e| anyhow::anyhow!("iced error: {e}"))?;

    Ok(())
}

impl App {
    fn update(&mut self, message: Message) -> IcedTask<Message> {
        match message {
            Message::Close => iced::exit(),
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
