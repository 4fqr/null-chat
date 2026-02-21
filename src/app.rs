use std::any::TypeId;
use std::time::Duration;
use iced::futures::SinkExt;

use iced::{
    Application, Command, Element, Event, Settings, Subscription, Theme,
    event, keyboard, window,
};

use crate::model::WireMessage;
use crate::network::p2p::P2PStatus;
use crate::ui::command_center::{CommandCenter, UiMessage};

pub fn run() -> iced::Result {
    NullChatApp::run(Settings {
        window: window::Settings {
            size: iced::Size::new(1280.0, 800.0),
            min_size: Some(iced::Size::new(900.0, 600.0)),
            ..Default::default()
        },
        ..Default::default()
    })
}

pub struct NullChatApp {
    command_center: CommandCenter,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ui(UiMessage),
    PanicButtonTriggered,
    EventOccurred(Event),
}

impl Application for NullChatApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let (command_center, cmd) = CommandCenter::new();
        (
            Self { command_center },
            cmd.map(Message::Ui),
        )
    }

    fn title(&self) -> String {
        String::from("NULLCHAT \u{25cf} SOVEREIGN NETWORK")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Ui(msg) => self.command_center.update(msg).map(Message::Ui),
            Message::PanicButtonTriggered => crate::panic_engine::PanicEngine::execute(),
            Message::EventOccurred(Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(ref ch),
                modifiers,
                ..
            })) if modifiers.control() && modifiers.alt() && modifiers.shift() && ch.as_str() == "x" => {
                crate::panic_engine::PanicEngine::execute()
            }
            Message::EventOccurred(_) => Command::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.command_center.view().map(Message::Ui)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        let event_sub = event::listen().map(Message::EventOccurred);

        let queue = self.command_center.incoming_queue();
        let p2p_sub = iced::subscription::channel(
            TypeId::of::<WireMessage>(),
            64,
            move |mut sender| async move {
                loop {
                    let msgs: Vec<WireMessage> = {
                        let mut q = queue.lock().await;
                        q.drain(..).collect::<Vec<_>>()
                    };
                    for msg in msgs {
                        let _ = sender.send(Message::Ui(UiMessage::IncomingP2P(msg))).await;
                    }
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            },
        );

        Subscription::batch([event_sub, p2p_sub])
    }
}
