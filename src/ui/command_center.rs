use iced::{
    widget::{column, container, horizontal_rule, row, scrollable, text, text_input, button, Space},
    Alignment, Color, Command, Element, Length,
};

use crate::storage::vault::EncryptedVault;
use crate::ui::theme::{
    TEXT_DIM, TEXT_PRIMARY, TEXT_BRIGHT, STATUS_SECURE, STATUS_PENDING, STATUS_INSECURE,
    BgBlack, StatusBarStyle, PanelLeft, PanelMid, PanelRight,
    MessageHeaderStyle, ComposeBarStyle, UnlockCardStyle,
    FlatButton, ActiveFlatButton, SendButtonStyle, AccentButton, DarkInputStyle,
};

const MIN_PASSPHRASE_LEN: usize = 12;

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
    SetupPassphraseChanged(String),
    SetupConfirmChanged(String),
    SetupCreateAccount,
    UnlockPassphraseChanged(String),
    UnlockVault,
    WorkspaceSelected(usize),
    RoomSelected(usize),
    MessageInputChanged(String),
    MessageSendRequested,
    TorStateChanged(TorCircuitDisplay),
    ConnectAccount(String),
    DisconnectAccount(String),
    SafetyNumberVerified(String),
    ExportVaultRequested,
    ShowMigrationGuide,
    DismissMigrationGuide,
}

#[derive(Debug)]
enum AppPhase {
    AccountSetup {
        passphrase: String,
        confirm: String,
        error: Option<String>,
    },
    VaultUnlock {
        passphrase: String,
        error: Option<String>,
    },
    MainInterface,
}

#[derive(Debug)]
pub struct CommandCenter {
    phase: AppPhase,
    selected_workspace: usize,
    selected_room: Option<usize>,
    message_input: String,
    workspaces: Vec<WorkspaceEntry>,
    rooms: Vec<RoomEntry>,
    messages: Vec<MessageEntry>,
    tor_state: TorCircuitDisplay,
    local_fingerprint: String,
    vault_path: std::path::PathBuf,
    show_migration_guide: bool,
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
        let vault_path = EncryptedVault::default_path();
        let is_first_run = EncryptedVault::is_first_run(&vault_path);

        let phase = if is_first_run {
            AppPhase::AccountSetup {
                passphrase: String::new(),
                confirm: String::new(),
                error: None,
            }
        } else {
            AppPhase::VaultUnlock {
                passphrase: String::new(),
                error: None,
            }
        };

        let state = Self {
            phase,
            selected_workspace: 0,
            selected_room: None,
            message_input: String::new(),
            local_fingerprint: fingerprint,
            vault_path,
            show_migration_guide: false,
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
            UiMessage::SetupPassphraseChanged(p) => {
                if let AppPhase::AccountSetup { passphrase, error, .. } = &mut self.phase {
                    *passphrase = p;
                    *error = None;
                }
            }
            UiMessage::SetupConfirmChanged(c) => {
                if let AppPhase::AccountSetup { confirm, error, .. } = &mut self.phase {
                    *confirm = c;
                    *error = None;
                }
            }
            UiMessage::SetupCreateAccount => {
                if let AppPhase::AccountSetup { passphrase, confirm, error } = &mut self.phase {
                    let p = passphrase.clone();
                    let c = confirm.clone();
                    if p.len() < MIN_PASSPHRASE_LEN {
                        *error = Some(format!(
                            "PASSPHRASE TOO SHORT  //  MINIMUM {} CHARACTERS REQUIRED",
                            MIN_PASSPHRASE_LEN
                        ));
                    } else if p != c {
                        *error = Some(String::from(
                            "PASSPHRASE MISMATCH  //  ENTRIES DO NOT MATCH",
                        ));
                    } else if passphrase_is_trivial(&p) {
                        *error = Some(String::from(
                            "PASSPHRASE TOO SIMPLE  //  USE MIXED CHARACTER CLASSES",
                        ));
                    } else {
                        let mut vault = EncryptedVault::new();
                        match vault.open(&self.vault_path, &p) {
                            Ok(()) => {
                                self.phase = AppPhase::MainInterface;
                            }
                            Err(e) => {
                                *error = Some(format!("VAULT CREATION FAILED  //  {}", e));
                            }
                        }
                    }
                }
            }
            UiMessage::UnlockPassphraseChanged(p) => {
                if let AppPhase::VaultUnlock { passphrase, error } = &mut self.phase {
                    *passphrase = p;
                    *error = None;
                }
            }
            UiMessage::UnlockVault => {
                if let AppPhase::VaultUnlock { passphrase, error } = &mut self.phase {
                    let p = passphrase.clone();
                    if p.is_empty() {
                        *error = Some(String::from("PASSPHRASE REQUIRED  //  FIELD IS EMPTY"));
                    } else {
                        let mut vault = EncryptedVault::new();
                        match vault.open(&self.vault_path, &p) {
                            Ok(()) => {
                                self.phase = AppPhase::MainInterface;
                            }
                            Err(crate::storage::vault::VaultError::Decryption) => {
                                *error = Some(String::from(
                                    "INCORRECT PASSPHRASE  //  VAULT COULD NOT BE DECRYPTED",
                                ));
                            }
                            Err(e) => {
                                *error = Some(format!("VAULT ERROR  //  {}", e));
                            }
                        }
                    }
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
            UiMessage::ShowMigrationGuide => {
                self.show_migration_guide = true;
            }
            UiMessage::DismissMigrationGuide => {
                self.show_migration_guide = false;
            }
            UiMessage::ExportVaultRequested => {
                self.show_migration_guide = true;
            }
            UiMessage::ConnectAccount(_) | UiMessage::DisconnectAccount(_) => {}
            UiMessage::SafetyNumberVerified(_) => {}
        }
        Command::none()
    }

    pub fn view(&self) -> Element<'_, UiMessage> {
        match &self.phase {
            AppPhase::AccountSetup { passphrase, confirm, error } => {
                self.view_account_setup(passphrase, confirm, error.as_deref())
            }
            AppPhase::VaultUnlock { passphrase, error } => {
                self.view_vault_unlock(passphrase, error.as_deref())
            }
            AppPhase::MainInterface => self.view_main_interface(),
        }
    }

    fn view_account_setup(
        &self,
        passphrase: &str,
        confirm: &str,
        error: Option<&str>,
    ) -> Element<'_, UiMessage> {
        let title = text("NULLCHAT  //  FIRST RUN")
            .size(22)
            .style(iced::theme::Text::Color(TEXT_BRIGHT));

        let subtitle = text("Create your encrypted vault. This passphrase cannot be recovered.")
            .size(11)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let fp_label = text("YOUR IDENTITY FINGERPRINT")
            .size(9)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let fp_value = text(self.local_fingerprint.as_str())
            .size(10)
            .style(iced::theme::Text::Color(STATUS_SECURE));

        let pw_label = text("PASSPHRASE  (12 chars minimum, mixed case + symbols recommended)")
            .size(9)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let pw_input = text_input("create passphrase...", passphrase)
            .secure(true)
            .on_input(UiMessage::SetupPassphraseChanged)
            .on_submit(UiMessage::SetupCreateAccount)
            .size(13)
            .width(Length::Fill)
            .style(iced::theme::TextInput::Custom(Box::new(DarkInputStyle)));

        let confirm_label = text("CONFIRM PASSPHRASE")
            .size(9)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let confirm_input = text_input("repeat passphrase...", confirm)
            .secure(true)
            .on_input(UiMessage::SetupConfirmChanged)
            .on_submit(UiMessage::SetupCreateAccount)
            .size(13)
            .width(Length::Fill)
            .style(iced::theme::TextInput::Custom(Box::new(DarkInputStyle)));

        let create_btn = button(
            container(
                text("CREATE ACCOUNT")
                    .size(12)
                    .style(iced::theme::Text::Color(Color::BLACK)),
            )
            .width(Length::Fill)
            .center_x()
            .padding([6, 0, 6, 0]),
        )
        .width(Length::Fill)
        .on_press(UiMessage::SetupCreateAccount)
        .style(iced::theme::Button::Custom(Box::new(AccentButton)));

        let error_row: Element<'_, UiMessage> = match error {
            Some(msg) => text(msg)
                .size(10)
                .style(iced::theme::Text::Color(STATUS_INSECURE))
                .into(),
            None => Space::with_height(Length::Fixed(14.0)).into(),
        };

        let security_note = text(
            "Argon2id (65 MiB, t=3, p=4)  //  AES-256-GCM  //  Double Ratchet + Kyber-1024",
        )
        .size(9)
        .style(iced::theme::Text::Color(TEXT_DIM));

        let card = container(
            column![
                title,
                Space::with_height(Length::Fixed(4.0)),
                subtitle,
                Space::with_height(Length::Fixed(16.0)),
                horizontal_rule(1),
                Space::with_height(Length::Fixed(12.0)),
                fp_label,
                Space::with_height(Length::Fixed(3.0)),
                fp_value,
                Space::with_height(Length::Fixed(16.0)),
                pw_label,
                Space::with_height(Length::Fixed(4.0)),
                pw_input,
                Space::with_height(Length::Fixed(10.0)),
                confirm_label,
                Space::with_height(Length::Fixed(4.0)),
                confirm_input,
                Space::with_height(Length::Fixed(12.0)),
                create_btn,
                error_row,
                Space::with_height(Length::Fixed(14.0)),
                horizontal_rule(1),
                Space::with_height(Length::Fixed(10.0)),
                security_note,
            ]
            .spacing(0)
            .padding(28)
            .width(Length::Fixed(500.0)),
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

    fn view_vault_unlock(
        &self,
        passphrase: &str,
        error: Option<&str>,
    ) -> Element<'_, UiMessage> {
        let title = text("NULLCHAT")
            .size(30)
            .style(iced::theme::Text::Color(TEXT_BRIGHT));

        let subtitle = text("v0.2.0  //  SOVEREIGN SECURE MESSENGER")
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let vault_path_label = text(
            format!("VAULT: {}", self.vault_path.display()),
        )
        .size(9)
        .style(iced::theme::Text::Color(TEXT_DIM));

        let passphrase_label = text("VAULT PASSPHRASE")
            .size(10)
            .style(iced::theme::Text::Color(TEXT_DIM));

        let passphrase_input = text_input("enter passphrase...", passphrase)
            .secure(true)
            .on_input(UiMessage::UnlockPassphraseChanged)
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

        let error_row: Element<'_, UiMessage> = match error {
            Some(msg) => text(msg)
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
                Space::with_height(Length::Fixed(6.0)),
                vault_path_label,
                Space::with_height(Length::Fixed(22.0)),
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
        if self.show_migration_guide {
            return self.view_migration_guide();
        }

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

    fn view_migration_guide(&self) -> Element<'_, UiMessage> {
        let title = text("DEVICE MIGRATION  //  VAULT EXPORT PROCEDURE")
            .size(16)
            .style(iced::theme::Text::Color(TEXT_BRIGHT));

        let vault_path_str = self.vault_path.display().to_string();

        let step1 = text("STEP 1  —  SECURE COPY YOUR VAULT DIRECTORY")
            .size(11)
            .style(iced::theme::Text::Color(STATUS_SECURE));

        let cmd1 = text(format!("  rsync -av --progress {}/ user@newdevice:~/.local/share/null-chat/vault/", vault_path_str))
            .size(11)
            .style(iced::theme::Text::Color(TEXT_PRIMARY));

        let step2 = text("STEP 2  —  VERIFY INTEGRITY ON TARGET DEVICE")
            .size(11)
            .style(iced::theme::Text::Color(STATUS_SECURE));

        let cmd2 = text(format!("  sha3sum {}/data.mdb", vault_path_str))
            .size(11)
            .style(iced::theme::Text::Color(TEXT_PRIMARY));

        let step3 = text("STEP 3  —  INSTALL NULLCHAT ON TARGET DEVICE AND UNLOCK WITH YOUR PASSPHRASE")
            .size(11)
            .style(iced::theme::Text::Color(STATUS_SECURE));

        let warning = text(
            "WARNING: The vault is encrypted with AES-256-GCM. Without your passphrase, it is irrecoverable. There is no password reset."
        )
        .size(10)
        .style(iced::theme::Text::Color(STATUS_PENDING));

        let dismiss_btn = button(
            text("CLOSE").size(11).style(iced::theme::Text::Color(Color::BLACK)),
        )
        .on_press(UiMessage::DismissMigrationGuide)
        .style(iced::theme::Button::Custom(Box::new(AccentButton)));

        let card = container(
            column![
                title,
                Space::with_height(Length::Fixed(16.0)),
                horizontal_rule(1),
                Space::with_height(Length::Fixed(14.0)),
                step1,
                Space::with_height(Length::Fixed(6.0)),
                cmd1,
                Space::with_height(Length::Fixed(14.0)),
                step2,
                Space::with_height(Length::Fixed(6.0)),
                cmd2,
                Space::with_height(Length::Fixed(14.0)),
                step3,
                Space::with_height(Length::Fixed(18.0)),
                horizontal_rule(1),
                Space::with_height(Length::Fixed(12.0)),
                warning,
                Space::with_height(Length::Fixed(14.0)),
                dismiss_btn,
            ]
            .spacing(0)
            .padding(32)
            .width(Length::Fixed(680.0)),
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
                text("  //  v0.2.0")
                    .size(10)
                    .style(iced::theme::Text::Color(TEXT_DIM)),
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

                let btn = button(btn_content)
                    .width(Length::Fill)
                    .on_press(UiMessage::WorkspaceSelected(i));

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

        let migrate_btn = button(
            text("MIGRATE DEVICE")
                .size(9)
                .style(iced::theme::Text::Color(TEXT_DIM)),
        )
        .on_press(UiMessage::ShowMigrationGuide)
        .style(iced::theme::Button::Custom(Box::new(FlatButton)));

        container(
            column![
                container(
                    column![network_header, horizontal_rule(1)].spacing(4),
                )
                .padding([10, 10, 6, 10]),
                column(workspace_buttons).spacing(1),
                Space::with_height(Length::Fill),
                container(
                    column![identity_header, horizontal_rule(1), fingerprint_text, Space::with_height(Length::Fixed(8.0)), migrate_btn].spacing(4),
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

                let btn = button(btn_content)
                    .width(Length::Fill)
                    .on_press(UiMessage::RoomSelected(i));

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
                let fp_len = m.sender_fingerprint.len();
                let fp_prefix = &m.sender_fingerprint[..8.min(fp_len)];
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

        let empty_hint: Element<'_, UiMessage> = if self.messages.is_empty() && self.selected_room.is_some() {
            container(
                text("NO MESSAGES  //  ENCRYPTION ACTIVE  //  BEGIN COMPOSING")
                    .size(10)
                    .style(iced::theme::Text::Color(TEXT_DIM)),
            )
            .width(Length::Fill)
            .center_x()
            .padding([18, 0, 0, 0])
            .into()
        } else {
            Space::with_height(Length::Shrink).into()
        };

        let message_scroll = scrollable(
            column![empty_hint, column(message_rows).spacing(0)]
                .padding([6, 0, 6, 0]),
        )
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

fn passphrase_is_trivial(p: &str) -> bool {
    let has_upper = p.chars().any(|c| c.is_uppercase());
    let has_lower = p.chars().any(|c| c.is_lowercase());
    let has_digit = p.chars().any(|c| c.is_ascii_digit());
    !(has_upper && has_lower && has_digit)
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


