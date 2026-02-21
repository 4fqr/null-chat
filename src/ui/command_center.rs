use iced::{
    widget::{column, container, horizontal_rule, row, scrollable, text, text_input, button, Space},
    Alignment, Color, Command, Element, Length,
};

use crate::ui::theme::{
    TEXT_DIM, TEXT_PRIMARY, TEXT_BRIGHT, STATUS_SECURE, STATUS_PENDING, STATUS_INSECURE,
    BgBlack, StatusBarStyle, PanelLeft, PanelMid, PanelRight,
    MessageHeaderStyle, ComposeBarStyle, UnlockCardStyle,
    FlatButton, ActiveFlatButton, SendButtonStyle, AccentButton, DarkInputStyle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorCircuitDisplay {
    Initializing,
    Building,
    Ready,
    Failed,
}

impl Default for TorCircuitDisplay {
    fn default() -> Self {
        Self::Initializing
    }
}

#[derive(Debug, Clone)]
pub enum UiMessage {
    PassphraseChanged(String),
    UnlockVault,
    WorkspaceSelected(usize),
    RoomSelected(usize),
    MessageInputChanged(String),
    MessageSendRequested,
    TorStateChanged(TorCircuitDisplay),
    ConnectAccount(String),
    DisconnectAccount(String),
    SafetyNumberVerified(String),
}

#[derive(Debug, Default)]
enum AppPhase {
    #[default]
    VaultUnlock,
    MainInterface,
}

#[derive(Debug)]
pub struct CommandCenter {
    phase: AppPhase,
    passphrase: String,
    unlock_error: Option<String>,
    selected_workspace: usize,
    selected_room: Option<usize>,
    message_input: String,
    workspaces: Vec<WorkspaceEntry>,
    rooms: Vec<RoomEntry>,
    messages: Vec<MessageEntry>,
    tor_state: TorCircuitDisplay,
    local_fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceEntry {
    pub id: String,
    pub label: String,
    pub protocol: String,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct RoomEntry {
    pub id: String,
    pub label: String,
    pub unread: u32,
    pub is_encrypted: bool,
}

#[derive(Debug, Clone)]
pub struct MessageEntry {
    pub sender_id: String,
    pub sender_fingerprint: String,
    pub body: String,
    pub timestamp_utc: String,
    pub is_verified: bool,
}

impl CommandCenter {
    pub fn new() -> (Self, Command<UiMessage>) {
        let identity = crate::crypto::LocalIdentity::generate();
        let fingerprint = identity.fingerprint_hex();
        let state = Self {
            phase: AppPhase::VaultUnlock,
            passphrase: String::new(),
            unlock_error: None,
            selected_workspace: 0,
            selected_room: None,
            message_input: String::new(),
            local_fingerprint: fingerprint,
            workspaces: vec![
                WorkspaceEntry {
                    id: String::from("ncp-sovereign"),
                    label: String::from("NULL NETWORK"),
                    protocol: String::from("NCP v1"),
                    connected: false,
                },
                WorkspaceEntry {
                    id: String::from("discord-gateway"),
                    label: String::from("DISCORD"),
                    protocol: String::from("GATEWAY"),
                    connected: false,
                },
                WorkspaceEntry {
                    id: String::from("matrix-homeserver"),
                    label: String::from("MATRIX"),
                    protocol: String::from("CS API"),
                    connected: false,
                },
            ],
            rooms: vec![
                RoomEntry {
                    id: String::from("null-main"),
                    label: String::from("null-main"),
                    unread: 0,
                    is_encrypted: true,
                },
                RoomEntry {
                    id: String::from("null-ops"),
                    label: String::from("null-ops"),
                    unread: 0,
                    is_encrypted: true,
                },
            ],
            messages: Vec::new(),
            tor_state: TorCircuitDisplay::Initializing,
        };
        (state, Command::none())
    }

    pub fn update(&mut self, message: UiMessage) -> Command<UiMessage> {
        match message {
            UiMessage::PassphraseChanged(p) => {
                self.passphrase = p;
                self.unlock_error = None;
            }
            UiMessage::UnlockVault => {
                if self.passphrase.len() < 8 {
                    self.unlock_error =
                        Some(String::from("PASSPHRASE TOO SHORT  //  MINIMUM 8 CHARACTERS"));
                } else {
                    self.phase = AppPhase::MainInterface;
                    self.passphrase.clear();
                    self.unlock_error = None;
                }
            }
            UiMessage::WorkspaceSelected(idx) => {
                self.selected_workspace = idx;
                self.selected_room = None;
            }
            UiMessage::RoomSelected(idx) => {
                self.selected_room = Some(idx);
            }
            UiMessage::MessageInputChanged(s) => {
                self.message_input = s;
            }
            UiMessage::MessageSendRequested => {
                let body = self.message_input.trim().to_string();
                if !body.is_empty() {
                    let fp = self.local_fingerprint.clone();
                    self.messages.push(MessageEntry {
                        sender_id: String::from("local"),
                        sender_fingerprint: fp,
                        body,
                        timestamp_utc: utc_time_string(),
                        is_verified: true,
                    });
                    self.message_input.clear();
                }
            }
            UiMessage::TorStateChanged(state) => {
                self.tor_state = state;
            }
            UiMessage::ConnectAccount(_) | UiMessage::DisconnectAccount(_) => {}
            UiMessage::SafetyNumberVerified(_) => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, UiMessage> {
        match self.phase {
            AppPhase::VaultUnlock => self.view_vault_unlock(),
            AppPhase::MainInterface => self.view_main_interface(),
        }
    }

    fn view_vault_unlock(&self) -> Element<'_, UiMessage> {
        let title = text("NULLCHAT")
            .size(30)
            .style(iced::theme::Text::Color(TEXT_BRIGHT));

        let subtitle = text("v0.1.0  //  SOVEREIGN SECURE MESSENGER")
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let passphrase_label = text("VAULT PASSPHRASE")
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let passphrase_input = text_input("enter passphrase...", &self.passphrase)
            .secure(true)
            .on_input(UiMessage::PassphraseChanged)
            .on_submit(UiMessage::UnlockVault)
            .size(13)
            .width(Length::Fill)
            .style(iced::theme::TextInput::Custom(Box::new(DarkInputStyle)));

        let unlock_btn = button(
            container(
                text("UNLOCK VAULT")
                    .size(12)
                    .style(iced::theme::Text::Color(Color::BLACK)),
            )
            .width(Length::Fill)
            .center_x()
            .padding([6, 0, 6, 0]),
        )
        .width(Length::Fill)
        .on_press(UiMessage::UnlockVault)
        .style(iced::theme::Button::Custom(Box::new(AccentButton)));

        let error_row: Element<'_, UiMessage> = match &self.unlock_error {
            Some(err) => text(err.as_str())
                .size(10)
                .style(iced::theme::Text::Color(STATUS_INSECURE))
                .into(),
            None => Space::with_height(Length::Fixed(14.0)).into(),
        };

        let security_note = text(
            "Argon2id key derivation  //  AES-256-GCM at rest  //  Double Ratchet per session",
        )
        .size(9)
        .style(iced::theme::Text::Color(TEXT_DIM));

        let card = container(
            column![
                title,
                subtitle,
                Space::with_height(Length::Fixed(28.0)),
                passphrase_label,
                Space::with_height(Length::Fixed(5.0)),
                passphrase_input,
                Space::with_height(Length::Fixed(10.0)),
                unlock_btn,
                error_row,
                Space::with_height(Length::Fixed(18.0)),
                horizontal_rule(1),
                Space::with_height(Length::Fixed(10.0)),
                security_note,
            ]
            .spacing(0)
            .padding(28)
            .width(Length::Fixed(420.0)),
        )
        .style(iced::theme::Container::Custom(Box::new(UnlockCardStyle)));

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(iced::theme::Container::Custom(Box::new(BgBlack)))
            .into()
    }

    fn view_main_interface(&self) -> Element<'_, UiMessage> {
        let status_bar = self.view_status_bar();
        let left_col = self.view_workspace_column();
        let mid_col = self.view_room_column();
        let right_col = self.view_message_column();

        container(
            column![
                status_bar,
                row![left_col, mid_col, right_col]
                    .height(Length::Fill)
                    .spacing(0),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(BgBlack)))
        .into()
    }

    fn view_status_bar(&self) -> Element<'_, UiMessage> {
        let (tor_glyph, tor_color, tor_label) = match self.tor_state {
            TorCircuitDisplay::Ready => ("●", STATUS_SECURE, "CIRCUIT READY"),
            TorCircuitDisplay::Building => ("◑", STATUS_PENDING, "BUILDING CIRCUIT"),
            TorCircuitDisplay::Initializing => ("○", TEXT_DIM, "TOR INITIALIZING"),
            TorCircuitDisplay::Failed => ("✗", STATUS_INSECURE, "CIRCUIT FAILED"),
        };

        let fp_display = truncate_fingerprint(&self.local_fingerprint);

        container(
            row![
                text("NULLCHAT")
                    .size(11)
                    .style(iced::theme::Text::Color(TEXT_BRIGHT)),
                text("  //  TOR:")
                    .size(10)
                    .style(iced::theme::Text::Color(TEXT_DIM)),
                text(tor_glyph)
                    .size(10)
                    .style(iced::theme::Text::Color(tor_color)),
                text(tor_label)
                    .size(10)
                    .style(iced::theme::Text::Color(tor_color)),
                Space::with_width(Length::Fill),
                text("ID:")
                    .size(10)
                    .style(iced::theme::Text::Color(TEXT_DIM)),
                text(fp_display)
                    .size(10)
                    .style(iced::theme::Text::Color(TEXT_DIM)),
                text("  //  PANIC: CTRL+ALT+SHIFT+X")
                    .size(10)
                    .style(iced::theme::Text::Color(STATUS_INSECURE)),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .padding([5, 12, 5, 12]),
        )
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(StatusBarStyle)))
        .into()
    }

    fn view_workspace_column(&self) -> Element<'_, UiMessage> {
        let network_header = text("NETWORKS")
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let workspace_buttons: Vec<Element<'_, UiMessage>> = self
            .workspaces
            .iter()
            .enumerate()
            .map(|(i, ws)| {
                let dot = if ws.connected { "● " } else { "○ " };
                let dot_color = if ws.connected { STATUS_SECURE } else { TEXT_DIM };
                let proto_tag = text(ws.protocol.as_str())
                    .size(9)
                    .style(iced::theme::Text::Color(TEXT_DIM));
                let ws_label = text(ws.label.as_str())
                    .size(12)
                    .style(iced::theme::Text::Color(TEXT_PRIMARY));
                let indicator = text(dot)
                    .size(10)
                    .style(iced::theme::Text::Color(dot_color));

                let btn_content = row![
                    indicator,
                    column![ws_label, proto_tag].spacing(1),
                ]
                .spacing(5)
                .align_items(Alignment::Center)
                .padding([5, 10, 5, 10])
                .width(Length::Fill);

                let btn = button(btn_content).width(Length::Fill).on_press(UiMessage::WorkspaceSelected(i));

                if i == self.selected_workspace {
                    btn.style(iced::theme::Button::Custom(Box::new(ActiveFlatButton))).into()
                } else {
                    btn.style(iced::theme::Button::Custom(Box::new(FlatButton))).into()
                }
            })
            .collect();

        let identity_header = text("IDENTITY")
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let fp_short = truncate_fingerprint_short(&self.local_fingerprint);
        let fingerprint_text = text(fp_short)
            .size(9)
            .style(iced::theme::Text::Color(TEXT_DIM));

        container(
            column![
                container(
                    column![network_header, horizontal_rule(1)].spacing(4),
                )
                .padding([10, 10, 6, 10]),
                column(workspace_buttons).spacing(1),
                Space::with_height(Length::Fixed(18.0)),
                container(
                    column![identity_header, horizontal_rule(1), fingerprint_text].spacing(4),
                )
                .padding([0, 10, 10, 10]),
            ]
            .spacing(0)
            .height(Length::Fill),
        )
        .width(Length::Fixed(200.0))
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(PanelLeft)))
        .into()
    }

    fn view_room_column(&self) -> Element<'_, UiMessage> {
        let ws_name = self
            .workspaces
            .get(self.selected_workspace)
            .map(|ws| ws.label.as_str())
            .unwrap_or("UNKNOWN");

        let room_header = text(ws_name)
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let room_buttons: Vec<Element<'_, UiMessage>> = self
            .rooms
            .iter()
            .enumerate()
            .map(|(i, room)| {
                let prefix = if room.unread > 0 { "▸" } else { "#" };
                let label = if room.unread > 0 {
                    format!("{} {}  [{}]", prefix, room.label, room.unread)
                } else {
                    format!("{} {}", prefix, room.label)
                };
                let label_color = if room.unread > 0 { TEXT_BRIGHT } else { TEXT_PRIMARY };
                let lock_indicator = if room.is_encrypted {
                    text("[E]").size(9).style(iced::theme::Text::Color(STATUS_SECURE))
                } else {
                    text("[!]").size(9).style(iced::theme::Text::Color(STATUS_PENDING))
                };

                let btn_content = row![
                    text(label).size(12).style(iced::theme::Text::Color(label_color)),
                    Space::with_width(Length::Fill),
                    lock_indicator,
                ]
                .align_items(Alignment::Center)
                .padding([5, 10, 5, 10])
                .width(Length::Fill);

                let btn = button(btn_content).width(Length::Fill).on_press(UiMessage::RoomSelected(i));

                if self.selected_room == Some(i) {
                    btn.style(iced::theme::Button::Custom(Box::new(ActiveFlatButton))).into()
                } else {
                    btn.style(iced::theme::Button::Custom(Box::new(FlatButton))).into()
                }
            })
            .collect();

        container(
            column![
                container(
                    column![room_header, horizontal_rule(1)].spacing(4),
                )
                .padding([10, 10, 6, 10]),
                column(room_buttons).spacing(1),
                Space::with_height(Length::Fill),
            ]
            .spacing(0)
            .height(Length::Fill),
        )
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(PanelMid)))
        .into()
    }

    fn view_message_column(&self) -> Element<'_, UiMessage> {
        let room = self.rooms.get(self.selected_room.unwrap_or(usize::MAX));

        let channel_label = room
            .map(|r| format!("  #  {}", r.label))
            .unwrap_or_else(|| String::from("  SELECT A CHANNEL"));

        let security_badge: Element<'_, UiMessage> = match room {
            Some(r) if r.is_encrypted => text("  E2E SECURE  ")
                .size(10)
                .style(iced::theme::Text::Color(STATUS_SECURE))
                .into(),
            Some(_) => text("  UNENCRYPTED  ")
                .size(10)
                .style(iced::theme::Text::Color(STATUS_PENDING))
                .into(),
            None => Space::with_width(Length::Shrink).into(),
        };

        let message_channel_header = container(
            row![
                text(channel_label)
                    .size(13)
                    .style(iced::theme::Text::Color(TEXT_BRIGHT)),
                Space::with_width(Length::Fill),
                security_badge,
            ]
            .align_items(Alignment::Center)
            .padding([8, 14, 8, 14]),
        )
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(MessageHeaderStyle)));

        let message_rows: Vec<Element<'_, UiMessage>> = self
            .messages
            .iter()
            .map(|m| {
                let sender_color = if m.sender_id == "local" {
                    STATUS_SECURE
                } else {
                    TEXT_PRIMARY
                };
                let fp_prefix = &m.sender_fingerprint[..8.min(m.sender_fingerprint.len())];
                let verified_tag = if m.is_verified {
                    text("[✓]").size(9).style(iced::theme::Text::Color(STATUS_SECURE))
                } else {
                    text("[?]").size(9).style(iced::theme::Text::Color(STATUS_PENDING))
                };

                row![
                    text(m.timestamp_utc.as_str())
                        .size(10)
                        .style(iced::theme::Text::Color(TEXT_DIM)),
                    text(fp_prefix)
                        .size(10)
                        .style(iced::theme::Text::Color(sender_color)),
                    text("  ").size(10),
                    text(m.body.as_str())
                        .size(13)
                        .style(iced::theme::Text::Color(TEXT_PRIMARY)),
                    Space::with_width(Length::Fill),
                    verified_tag,
                ]
                .spacing(6)
                .align_items(Alignment::Center)
                .padding([4, 14, 4, 14])
                .into()
            })
            .collect();

        let message_scroll = scrollable(column(message_rows).spacing(0).padding([6, 0, 6, 0]))
            .height(Length::Fill);

        let placeholder_text = if self.selected_room.is_some() {
            "compose encrypted message..."
        } else {
            "select a channel to begin..."
        };

        let compose_input = text_input(placeholder_text, &self.message_input)
            .on_input(UiMessage::MessageInputChanged)
            .on_submit(UiMessage::MessageSendRequested)
            .size(12)
            .width(Length::Fill)
            .style(iced::theme::TextInput::Custom(Box::new(DarkInputStyle)));

        let send_btn = button(
            text("SEND \u{2192}")
                .size(11)
                .style(iced::theme::Text::Color(Color::BLACK)),
        )
        .on_press(UiMessage::MessageSendRequested)
        .style(iced::theme::Button::Custom(Box::new(SendButtonStyle)));

        let compose_bar = container(
            row![compose_input, send_btn]
                .spacing(8)
                .align_items(Alignment::Center)
                .padding([8, 14, 8, 14]),
        )
        .width(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(ComposeBarStyle)));

        container(
            column![
                message_channel_header,
                horizontal_rule(1),
                message_scroll,
                horizontal_rule(1),
                compose_bar,
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced::theme::Container::Custom(Box::new(PanelRight)))
        .into()
    }
}

fn utc_time_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn truncate_fingerprint(fp: &str) -> String {
    if fp.len() >= 16 {
        format!("{}...{}", &fp[..8], &fp[fp.len() - 8..])
    } else {
        fp.to_string()
    }
}

fn truncate_fingerprint_short(fp: &str) -> String {
    if fp.len() > 14 {
        format!("{}...", &fp[..12])
    } else {
        fp.to_string()
    }
}
