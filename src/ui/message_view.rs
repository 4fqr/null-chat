use iced::{Element, widget::{column, row, text, scrollable}, Length};
use crate::ui::command_center::UiMessage;

pub struct MessageView;

impl MessageView {
    pub fn view<'a>(messages: &'a [super::command_center::MessageEntry]) -> Element<'a, UiMessage> {
        let rows: Vec<Element<'a, UiMessage>> = messages
            .iter()
            .map(|m| {
                row![
                    text(m.timestamp_utc.as_str()).size(10),
                    text(&m.sender_fingerprint[..8.min(m.sender_fingerprint.len())]).size(12),
                    text(m.body.as_str()).size(13),
                ]
                .spacing(8)
                .into()
            })
            .collect();

        scrollable(column(rows).spacing(4).padding(10))
            .height(Length::Fill)
            .into()
    }
}
