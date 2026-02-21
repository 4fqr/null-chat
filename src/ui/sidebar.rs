use iced::{Element, widget::{column, button, text}, Length};
use crate::ui::command_center::UiMessage;

pub struct Sidebar;

impl Sidebar {
    pub fn view<'a>(entries: &'a [super::command_center::WorkspaceEntry], selected: usize) -> Element<'a, UiMessage> {
        let items: Vec<Element<'a, UiMessage>> = entries
            .iter()
            .enumerate()
            .map(|(i, ws)| {
                button(text(ws.label.as_str()).size(13))
                    .on_press(UiMessage::WorkspaceSelected(i))
                    .width(Length::Fill)
                    .into()
            })
            .collect();

        column(items).spacing(2).padding(8).into()
    }
}
