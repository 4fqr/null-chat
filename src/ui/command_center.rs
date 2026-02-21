use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use iced::{
    widget::{button, column, container, horizontal_rule, row, scrollable, text, text_input},
    Alignment, Color, Command, Element, Length,
};
use iced::widget::Space;

use crate::model::{
    now_unix, format_ts, user_color_for, user_initials, generate_server_code, short_id,
    Channel, ChannelMessage, DirectMessage, Friend, GroupChat, GroupMember, GroupMessage,
    Server, WireKind, WireMessage,
};
use crate::network::p2p::P2PStatus;
use crate::storage::vault::EncryptedVault;
use crate::ui::theme::{
    AvatarContainer, ActiveFlatButton, BlurpleButton, BgDark, BgDarkest, BgMain,
    CardStyle, ComposeBarStyle, DangerButton, DiscordInput, FlatButton, GhostButton,
    MessageHeaderStyle, ServerIconButton, ActiveServerIconButton,
    UnlockCardStyle,
    BLURPLE, GREEN, RED, YELLOW,
    TEXT_MUTED, TEXT_NORMAL, TEXT_WHITE,
    BgBlack, AccentButton, SendButtonStyle, DarkInputStyle,
};
use iced::Background;

const MIN_PASS: usize = 12;

// ─── Navigation ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SelectedPane { Home, Server(Uuid) }

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveView {
    Friends,
    DirectMessage(Uuid),
    GroupChat(Uuid),
    Channel(Uuid, Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Modal {
    None, AddFriend, NewGroup, NewServer, JoinServer,
    Profile, MigrateDevice, GroupAddMember(Uuid), ServerInfo(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorCircuitDisplay { Initializing, Building, Ready, Failed }
impl Default for TorCircuitDisplay { fn default() -> Self { Self::Initializing } }

// ─── UiMessage ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UiMessage {
    SetupPassChanged(String),
    SetupConfirmChanged(String),
    SetupNameChanged(String),
    SetupCreate,
    UnlockPassChanged(String),
    UnlockVault,
    SelectPane(SelectedPane),
    SelectView(ActiveView),
    OpenModal(Modal),
    CloseModal,
    ComposeChanged(String),
    SendMessage,
    Field1Changed(String),
    Field2Changed(String),
    SubmitAddFriend,
    SubmitNewGroup,
    SubmitNewServer,
    SubmitJoinServer,
    AddMemberToGroup(Uuid),
    IncomingP2P(WireMessage),
    TorStatusChanged(P2PStatus),
    // Legacy compat
    WorkspaceSelected(usize),
    RoomSelected(usize),
    MessageInputChanged(String),
    MessageSendRequested,
    TorStateChanged(TorCircuitDisplay),
    ConnectAccount(String),
    DisconnectAccount(String),
    SafetyNumberVerified(String),
    ShowMigrationGuide,
    DismissMigrationGuide,
    ExportVaultRequested,
    SetupPassphraseChanged(String),
    SetupConfirmChanged2(String),
    SetupCreateAccount,
    UnlockPassphraseChanged(String),
}

// ─── AppPhase ─────────────────────────────────────────────────────────────────

enum AppPhase {
    Setup { passphrase: String, confirm: String, display_name: String, error: Option<String> },
    Unlock { passphrase: String, error: Option<String> },
    Main,
}

// ─── CommandCenter ────────────────────────────────────────────────────────────

pub struct CommandCenter {
    phase: AppPhase,
    vault_path: PathBuf,
    local_fingerprint: String,
    my_name: String,
    my_user_id: String,
    selected_pane: SelectedPane,
    active_view: ActiveView,
    modal: Modal,
    friends: Vec<Friend>,
    groups: Vec<GroupChat>,
    servers: Vec<Server>,
    conversations: HashMap<Uuid, Vec<DirectMessage>>,
    tor_status: P2PStatus,
    incoming_queue: Arc<Mutex<Vec<WireMessage>>>,
    tor_socks: Option<String>,
    compose_text: String,
    modal_f1: String,
    modal_f2: String,
    modal_err: Option<String>,
    unread: HashMap<String, u32>,
}

impl CommandCenter {
    pub fn new() -> (Self, Command<UiMessage>) {
        let vault_path = EncryptedVault::default_path();
        let is_first = EncryptedVault::is_first_run(&vault_path);
        let fp = {
            let id = crate::crypto::identity::LocalIdentity::generate();
            id.fingerprint_hex()
        };

        let phase = if is_first {
            AppPhase::Setup { passphrase: String::new(), confirm: String::new(), display_name: String::new(), error: None }
        } else {
            AppPhase::Unlock { passphrase: String::new(), error: None }
        };

        let cc = CommandCenter {
            phase, vault_path,
            local_fingerprint: fp.clone(),
            my_name: String::from("Me"),
            my_user_id: fp,
            selected_pane: SelectedPane::Home,
            active_view: ActiveView::Friends,
            modal: Modal::None,
            friends: Vec::new(),
            groups: Vec::new(),
            servers: Vec::new(),
            conversations: HashMap::new(),
            tor_status: P2PStatus::Offline,
            incoming_queue: Arc::new(Mutex::new(Vec::new())),
            tor_socks: None,
            compose_text: String::new(),
            modal_f1: String::new(),
            modal_f2: String::new(),
            modal_err: None,
            unread: HashMap::new(),
        };
        (cc, Command::none())
    }

    pub fn incoming_queue(&self) -> Arc<Mutex<Vec<WireMessage>>> {
        self.incoming_queue.clone()
    }

    // ─── Update ─────────────────────────────────────────────────────────────

    pub fn update(&mut self, msg: UiMessage) -> Command<UiMessage> {
        match msg {
            UiMessage::SetupPassChanged(v) | UiMessage::SetupPassphraseChanged(v) => {
                if let AppPhase::Setup { passphrase, error, .. } = &mut self.phase { *passphrase = v; *error = None; }
            }
            UiMessage::SetupConfirmChanged(v) | UiMessage::SetupConfirmChanged2(v) => {
                if let AppPhase::Setup { confirm, error, .. } = &mut self.phase { *confirm = v; *error = None; }
            }
            UiMessage::SetupNameChanged(v) => {
                if let AppPhase::Setup { display_name, .. } = &mut self.phase { *display_name = v; }
            }
            UiMessage::SetupCreate | UiMessage::SetupCreateAccount => {
                if let AppPhase::Setup { passphrase, confirm, display_name, error } = &mut self.phase {
                    let p = passphrase.clone(); let c = confirm.clone(); let n = display_name.clone();
                    if n.trim().is_empty() { *error = Some("Enter a display name.".into()); }
                    else if p.len() < MIN_PASS { *error = Some(format!("Passphrase must be at least {} chars.", MIN_PASS)); }
                    else if p != c { *error = Some("Passphrases don\'t match.".into()); }
                    else if pass_trivial(&p) { *error = Some("Use uppercase, lowercase and digits.".into()); }
                    else {
                        let mut vault = EncryptedVault::new();
                        match vault.open(&self.vault_path.clone(), &p) {
                            Ok(()) => {
                                self.my_name = if n.is_empty() { "Anonymous".into() } else { n };
                                self.phase = AppPhase::Main;
                                return self.cmd_init_p2p();
                            }
                            Err(e) => { *error = Some(format!("Vault error: {}", e)); }
                        }
                    }
                }
            }
            UiMessage::UnlockPassChanged(v) | UiMessage::UnlockPassphraseChanged(v) => {
                if let AppPhase::Unlock { passphrase, error } = &mut self.phase { *passphrase = v; *error = None; }
            }
            UiMessage::UnlockVault => {
                if let AppPhase::Unlock { passphrase, error } = &mut self.phase {
                    let p = passphrase.clone();
                    if p.is_empty() { *error = Some("Enter your passphrase.".into()); }
                    else {
                        let mut vault = EncryptedVault::new();
                        match vault.open(&self.vault_path.clone(), &p) {
                            Ok(()) => { self.phase = AppPhase::Main; return self.cmd_init_p2p(); }
                            Err(crate::storage::vault::VaultError::Decryption) => {
                                *error = Some("Incorrect passphrase.".into());
                            }
                            Err(e) => { *error = Some(format!("Vault error: {}", e)); }
                        }
                    }
                }
            }
            UiMessage::SelectPane(p) => {
                self.selected_pane = p; self.active_view = ActiveView::Friends;
            }
            UiMessage::SelectView(v) => {
                let key = match &v {
                    ActiveView::DirectMessage(id) => id.to_string(),
                    ActiveView::GroupChat(id) => id.to_string(),
                    ActiveView::Channel(_, cid) => cid.to_string(),
                    _ => String::new(),
                };
                if !key.is_empty() { self.unread.remove(&key); }
                self.active_view = v; self.modal = Modal::None;
            }
            UiMessage::OpenModal(m) => {
                self.modal = m; self.modal_f1.clear(); self.modal_f2.clear(); self.modal_err = None;
            }
            UiMessage::CloseModal => { self.modal = Modal::None; }
            UiMessage::ComposeChanged(v) | UiMessage::MessageInputChanged(v) => {
                self.compose_text = v;
            }
            UiMessage::SendMessage | UiMessage::MessageSendRequested => {
                let body = self.compose_text.trim().to_string();
                if body.is_empty() { return Command::none(); }
                self.compose_text.clear();
                let from_id = self.my_user_id.clone();
                let from_name = self.my_name.clone();
                let ts = now_unix();
                match self.active_view.clone() {
                    ActiveView::DirectMessage(friend_id) => {
                        self.conversations.entry(friend_id).or_default().push(DirectMessage {
                            id: Uuid::new_v4(), from_id: from_id.clone(), body: body.clone(), timestamp: ts, outgoing: true,
                        });
                        if let Some(friend) = self.friends.iter().find(|f| f.id == friend_id) {
                            let peer = friend.user_id.clone();
                            let socks = self.tor_socks.clone();
                            let wire = WireMessage { kind: WireKind::DirectMessage, from_id, from_name, target_id: peer.clone(), body, timestamp: ts };
                            return Command::perform(async move { crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await.ok(); }, |_| UiMessage::CloseModal);
                        }
                    }
                    ActiveView::GroupChat(group_id) => {
                        if let Some(g) = self.groups.iter_mut().find(|x| x.id == group_id) {
                            g.messages.push(GroupMessage { id: Uuid::new_v4(), from_id: from_id.clone(), from_name: from_name.clone(), body: body.clone(), timestamp: ts });
                            let members: Vec<String> = g.members.iter().map(|m| m.user_id.clone()).filter(|uid| *uid != from_id).collect();
                            let gid = group_id.to_string();
                            let socks = self.tor_socks.clone();
                            let wire = WireMessage { kind: WireKind::GroupMessage { group_id: gid.clone() }, from_id, from_name, target_id: gid, body, timestamp: ts };
                            return Command::perform(async move { for peer in members { crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await.ok(); } }, |_| UiMessage::CloseModal);
                        }
                    }
                    ActiveView::Channel(server_id, channel_id) => {
                        if let Some(s) = self.servers.iter_mut().find(|x| x.id == server_id) {
                            if let Some(ch) = s.channels.iter_mut().find(|x| x.id == channel_id) {
                                ch.messages.push(ChannelMessage { id: Uuid::new_v4(), from_id: from_id.clone(), from_name: from_name.clone(), body: body.clone(), timestamp: ts });
                                let members = s.member_ids.clone();
                                let sid = server_id.to_string(); let cid_str = channel_id.to_string();
                                let socks = self.tor_socks.clone();
                                let wire = WireMessage { kind: WireKind::ChannelMessage { server_id: sid, channel_id: cid_str.clone() }, from_id, from_name, target_id: cid_str, body, timestamp: ts };
                                return Command::perform(async move { for peer in members { crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await.ok(); } }, |_| UiMessage::CloseModal);
                            }
                        }
                    }
                    _ => {}
                }
            }
            UiMessage::Field1Changed(v) => { self.modal_f1 = v; self.modal_err = None; }
            UiMessage::Field2Changed(v) => { self.modal_f2 = v; self.modal_err = None; }
            UiMessage::SubmitAddFriend => {
                let raw = self.modal_f1.trim().to_string(); let name = self.modal_f2.trim().to_string();
                if raw.is_empty() { self.modal_err = Some("Paste their User ID.".into()); }
                else if name.is_empty() { self.modal_err = Some("Enter a display name.".into()); }
                else if self.friends.iter().any(|f| f.user_id == raw) { self.modal_err = Some("Already your friend.".into()); }
                else { self.friends.push(Friend { id: Uuid::new_v4(), display_name: name, user_id: raw, added_at: now_unix() }); self.modal = Modal::None; }
            }
            UiMessage::SubmitNewGroup => {
                let name = self.modal_f1.trim().to_string();
                if name.is_empty() { self.modal_err = Some("Enter a group name.".into()); }
                else {
                    let gid = Uuid::new_v4();
                    self.groups.push(GroupChat { id: gid, name, members: vec![GroupMember { user_id: self.my_user_id.clone(), display_name: self.my_name.clone() }], messages: Vec::new(), created_at: now_unix() });
                    self.modal = Modal::None; self.active_view = ActiveView::GroupChat(gid);
                }
            }
            UiMessage::AddMemberToGroup(group_id) => {
                let uid = self.modal_f1.trim().to_string(); let uname = self.modal_f2.trim().to_string();
                if uid.is_empty() { self.modal_err = Some("Enter a User ID.".into()); }
                else {
                    if let Some(g) = self.groups.iter_mut().find(|x| x.id == group_id) {
                        if !g.members.iter().any(|m| m.user_id == uid) {
                            let nm = if uname.is_empty() { short_id(&uid) } else { uname };
                            g.members.push(GroupMember { user_id: uid, display_name: nm });
                        }
                    }
                    self.modal = Modal::None;
                }
            }
            UiMessage::SubmitNewServer => {
                let name = self.modal_f1.trim().to_string();
                if name.is_empty() { self.modal_err = Some("Enter a server name.".into()); }
                else {
                    let sid = Uuid::new_v4();
                    let ch = Channel { id: Uuid::new_v4(), name: "general".into(), messages: Vec::new() };
                    let fch = ch.id;
                    self.servers.push(Server { id: sid, name, server_code: generate_server_code(), owner_id: self.my_user_id.clone(), channels: vec![ch], member_ids: vec![self.my_user_id.clone()], created_at: now_unix(), is_owned: true });
                    self.modal = Modal::None; self.selected_pane = SelectedPane::Server(sid); self.active_view = ActiveView::Channel(sid, fch);
                }
            }
            UiMessage::SubmitJoinServer => {
                let code = self.modal_f1.trim().to_uppercase();
                if code.len() != 8 { self.modal_err = Some("Code is 8 characters.".into()); }
                else if let Some(s) = self.servers.iter_mut().find(|s| s.server_code == code) {
                    if !s.member_ids.contains(&self.my_user_id) { s.member_ids.push(self.my_user_id.clone()); }
                    let sid = s.id; let cid = s.channels.first().map(|c| c.id).unwrap_or_else(Uuid::new_v4);
                    self.modal = Modal::None; self.selected_pane = SelectedPane::Server(sid); self.active_view = ActiveView::Channel(sid, cid);
                } else { self.modal_err = Some("Server not found.".into()); }
            }
            UiMessage::IncomingP2P(wire) => { self.handle_incoming(wire); }
            UiMessage::TorStatusChanged(status) => {
                if let P2PStatus::TorReady { ref onion } = status {
                    if !onion.contains("system") { self.my_user_id = onion.clone(); }
                }
                self.tor_status = status;
            }
            UiMessage::WorkspaceSelected(_) | UiMessage::RoomSelected(_) | UiMessage::TorStateChanged(_)
            | UiMessage::ConnectAccount(_) | UiMessage::DisconnectAccount(_) | UiMessage::SafetyNumberVerified(_) => {}
            UiMessage::ShowMigrationGuide | UiMessage::ExportVaultRequested => { self.modal = Modal::MigrateDevice; }
            UiMessage::DismissMigrationGuide => { self.modal = Modal::None; }
        }
        Command::none()
    }

    fn cmd_init_p2p(&self) -> Command<UiMessage> {
        let data_dir = self.vault_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
        let queue = self.incoming_queue.clone();
        Command::perform(async move { crate::network::p2p::init_p2p(data_dir, queue).await }, |(status, _socks)| UiMessage::TorStatusChanged(status))
    }

    fn handle_incoming(&mut self, wire: WireMessage) {
        match &wire.kind {
            WireKind::DirectMessage => {
                let fid = if let Some(f) = self.friends.iter().find(|f| f.user_id == wire.from_id) {
                    f.id
                } else {
                    let fid = Uuid::new_v4();
                    self.friends.push(Friend { id: fid, display_name: wire.from_name.clone(), user_id: wire.from_id.clone(), added_at: now_unix() });
                    fid
                };
                self.conversations.entry(fid).or_default().push(DirectMessage { id: Uuid::new_v4(), from_id: wire.from_id.clone(), body: wire.body.clone(), timestamp: wire.timestamp, outgoing: false });
                if self.active_view != ActiveView::DirectMessage(fid) { *self.unread.entry(fid.to_string()).or_insert(0) += 1; }
            }
            WireKind::GroupMessage { group_id } => {
                if let Ok(gid) = Uuid::parse_str(group_id) {
                    if let Some(g) = self.groups.iter_mut().find(|x| x.id == gid) {
                        g.messages.push(GroupMessage { id: Uuid::new_v4(), from_id: wire.from_id.clone(), from_name: wire.from_name.clone(), body: wire.body.clone(), timestamp: wire.timestamp });
                        if self.active_view != ActiveView::GroupChat(gid) { *self.unread.entry(gid.to_string()).or_insert(0) += 1; }
                    }
                }
            }
            WireKind::ChannelMessage { server_id, channel_id } => {
                if let (Ok(sid), Ok(cid)) = (Uuid::parse_str(server_id), Uuid::parse_str(channel_id)) {
                    if let Some(s) = self.servers.iter_mut().find(|x| x.id == sid) {
                        if let Some(ch) = s.channels.iter_mut().find(|x| x.id == cid) {
                            ch.messages.push(ChannelMessage { id: Uuid::new_v4(), from_id: wire.from_id.clone(), from_name: wire.from_name.clone(), body: wire.body.clone(), timestamp: wire.timestamp });
                            if self.active_view != ActiveView::Channel(sid, cid) { *self.unread.entry(cid.to_string()).or_insert(0) += 1; }
                        }
                    }
                }
            }
            WireKind::FriendRequest => {
                if !self.friends.iter().any(|f| f.user_id == wire.from_id) {
                    self.friends.push(Friend { id: Uuid::new_v4(), display_name: wire.from_name.clone(), user_id: wire.from_id.clone(), added_at: now_unix() });
                }
            }
            WireKind::GroupInvite { group_id, group_name } => {
                if let Ok(gid) = Uuid::parse_str(group_id) {
                    if !self.groups.iter().any(|g| g.id == gid) {
                        self.groups.push(GroupChat { id: gid, name: group_name.clone(), members: vec![GroupMember { user_id: self.my_user_id.clone(), display_name: self.my_name.clone() }, GroupMember { user_id: wire.from_id.clone(), display_name: wire.from_name.clone() }], messages: Vec::new(), created_at: now_unix() });
                    }
                }
            }
            WireKind::Ping => {}
        }
    }

    // ─── View ────────────────────────────────────────────────────────────────

    pub fn view(&self) -> Element<'_, UiMessage> {
        match &self.phase {
            AppPhase::Setup { passphrase, confirm, display_name, error } => self.view_setup(passphrase, confirm, display_name, error.as_deref()),
            AppPhase::Unlock { passphrase, error } => self.view_unlock(passphrase, error.as_deref()),
            AppPhase::Main => self.view_main(),
        }
    }

    fn view_setup<'a>(&'a self, passphrase: &'a str, confirm: &'a str, display_name: &'a str, error: Option<&'a str>) -> Element<'a, UiMessage> {
        let card = container(column![
            text("Null Chat").size(32).style(iced::theme::Text::Color(TEXT_WHITE)),
            text("Post-quantum encrypted · Anonymous · Open Source").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            Space::with_height(Length::Fixed(20.0)),
            lbl("Display Name"),
            Space::with_height(Length::Fixed(4.0)),
            text_input("Your name (visible to contacts)", display_name).on_input(UiMessage::SetupNameChanged).on_submit(UiMessage::SetupCreate).size(14).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))),
            Space::with_height(Length::Fixed(12.0)),
            lbl("Passphrase"),
            Space::with_height(Length::Fixed(4.0)),
            text_input("Min 12 chars, mixed case + digits", passphrase).secure(true).on_input(UiMessage::SetupPassChanged).on_submit(UiMessage::SetupCreate).size(14).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))),
            Space::with_height(Length::Fixed(12.0)),
            lbl("Confirm Passphrase"),
            Space::with_height(Length::Fixed(4.0)),
            text_input("Confirm passphrase", confirm).secure(true).on_input(UiMessage::SetupConfirmChanged).on_submit(UiMessage::SetupCreate).size(14).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))),
            Space::with_height(Length::Fixed(16.0)),
            button(container(text("Create Account").size(14).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::SetupCreate).style(iced::theme::Button::Custom(Box::new(BlurpleButton))),
            Space::with_height(Length::Fixed(8.0)),
            err_row(error),
            Space::with_height(Length::Fixed(20.0)),
            text("AES-256-GCM · Argon2id · Kyber-1024 PQC").size(10).style(iced::theme::Text::Color(TEXT_MUTED)),
        ].spacing(0).padding(32).width(Length::Fixed(460.0))).style(iced::theme::Container::Custom(Box::new(UnlockCardStyle)));
        container(card).width(Length::Fill).height(Length::Fill).center_x().center_y().style(iced::theme::Container::Custom(Box::new(BgDarkest))).into()
    }

    fn view_unlock<'a>(&'a self, passphrase: &'a str, error: Option<&'a str>) -> Element<'a, UiMessage> {
        let card = container(column![
            text("Null Chat").size(36).style(iced::theme::Text::Color(TEXT_WHITE)),
            text("Welcome back.").size(13).style(iced::theme::Text::Color(TEXT_MUTED)),
            Space::with_height(Length::Fixed(24.0)),
            lbl("Passphrase"),
            Space::with_height(Length::Fixed(6.0)),
            text_input("Your passphrase", passphrase).secure(true).on_input(UiMessage::UnlockPassChanged).on_submit(UiMessage::UnlockVault).size(14).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))),
            Space::with_height(Length::Fixed(12.0)),
            button(container(text("Log In").size(14).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::UnlockVault).style(iced::theme::Button::Custom(Box::new(BlurpleButton))),
            Space::with_height(Length::Fixed(8.0)),
            err_row(error),
        ].spacing(0).padding(32).width(Length::Fixed(420.0))).style(iced::theme::Container::Custom(Box::new(UnlockCardStyle)));
        container(card).width(Length::Fill).height(Length::Fill).center_x().center_y().style(iced::theme::Container::Custom(Box::new(BgDarkest))).into()
    }

    fn view_main(&self) -> Element<'_, UiMessage> {
        let rail = self.view_rail();
        let sidebar = self.view_sidebar();
        let chat = self.view_chat();
        let base = container(row![rail, sidebar, chat].height(Length::Fill).spacing(0)).width(Length::Fill).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgDarkest)));
        if self.modal == Modal::None { return base.into(); }
        // iced 0.12 has no stack widget — show modal over full-screen backdrop
        container(self.view_modal()).width(Length::Fill).height(Length::Fill).center_x().center_y().style(iced::theme::Container::Custom(Box::new(Overlay))).into()
    }

    // ─── Server Rail ─────────────────────────────────────────────────────────

    fn view_rail(&self) -> Element<'_, UiMessage> {
        let home_active = self.selected_pane == SelectedPane::Home;
        let home_btn = button(container(text("⌂").size(22).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(48.0)).height(Length::Fixed(48.0)).center_x().center_y())
            .on_press(UiMessage::SelectPane(SelectedPane::Home))
            .style(if home_active { iced::theme::Button::Custom(Box::new(ActiveServerIconButton)) } else { iced::theme::Button::Custom(Box::new(ServerIconButton)) });

        let server_btns: Vec<Element<'_, UiMessage>> = self.servers.iter().map(|s| {
            let active = self.selected_pane == SelectedPane::Server(s.id);
            let color = user_color_for(&s.id.to_string());
            let av = container(text(user_initials(&s.name)).size(16).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(48.0)).height(Length::Fixed(48.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: if active { 8.0 } else { 24.0 } }))).center_x().center_y();
            let sid = s.id;
            container(button(av).on_press(UiMessage::SelectPane(SelectedPane::Server(sid))).style(if active { iced::theme::Button::Custom(Box::new(ActiveServerIconButton)) } else { iced::theme::Button::Custom(Box::new(ServerIconButton)) })).center_x().padding([2, 0]).into()
        }).collect();

        let add_btn = button(container(text("+").size(20).style(iced::theme::Text::Color(GREEN))).width(Length::Fixed(48.0)).height(Length::Fixed(48.0)).center_x().center_y()).on_press(UiMessage::OpenModal(Modal::NewServer)).style(iced::theme::Button::Custom(Box::new(ServerIconButton)));
        let join_btn = button(container(text("↩").size(18).style(iced::theme::Text::Color(TEXT_MUTED))).width(Length::Fixed(48.0)).height(Length::Fixed(48.0)).center_x().center_y()).on_press(UiMessage::OpenModal(Modal::JoinServer)).style(iced::theme::Button::Custom(Box::new(ServerIconButton)));

        let initials_me = user_initials(&self.my_name);
        let profile_btn = button(container(container(text(initials_me).size(14).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(40.0)).height(Length::Fixed(40.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color: BLURPLE, radius: 20.0 }))).center_x().center_y()).width(Length::Fixed(48.0)).height(Length::Fixed(48.0)).center_x().center_y()).on_press(UiMessage::OpenModal(Modal::Profile)).style(iced::theme::Button::Custom(Box::new(ServerIconButton)));

        let (dot_char, dot_col) = match &self.tor_status { P2PStatus::TorReady { .. } => ("●", GREEN), P2PStatus::TorConnecting => ("◑", YELLOW), P2PStatus::DirectMode => ("◈", YELLOW), _ => ("○", RED) };
        let tor_dot = container(text(dot_char).size(10).style(iced::theme::Text::Color(dot_col))).center_x();

        let mut items: Vec<Element<'_, UiMessage>> = vec![
            Space::with_height(Length::Fixed(8.0)).into(),
            container(home_btn).center_x().into(),
            container(horizontal_rule(1)).padding([4, 8]).into(),
        ];
        items.extend(server_btns);
        items.push(Space::with_height(Length::Fixed(4.0)).into());
        items.push(container(add_btn).center_x().into());
        items.push(Space::with_height(Length::Fixed(2.0)).into());
        items.push(container(join_btn).center_x().into());
        items.push(Space::with_height(Length::Fill).into());
        items.push(tor_dot.into());
        items.push(Space::with_height(Length::Fixed(4.0)).into());
        items.push(container(profile_btn).center_x().into());
        items.push(Space::with_height(Length::Fixed(8.0)).into());

        container(column(items).spacing(0)).width(Length::Fixed(72.0)).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgDarkest))).into()
    }

    // ─── Sidebar ─────────────────────────────────────────────────────────────

    fn view_sidebar(&self) -> Element<'_, UiMessage> {
        match &self.selected_pane {
            SelectedPane::Home => self.view_home_sidebar(),
            SelectedPane::Server(sid) => self.view_server_sidebar(*sid),
        }
    }

    fn view_home_sidebar(&self) -> Element<'_, UiMessage> {
        let header = container(row![text("Direct Messages").size(14).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fill), button(text("+").size(16).style(iced::theme::Text::Color(TEXT_MUTED))).on_press(UiMessage::OpenModal(Modal::AddFriend)).style(iced::theme::Button::Custom(Box::new(FlatButton)))].align_items(Alignment::Center).padding([12, 16])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgDark)));

        let friends_active = self.active_view == ActiveView::Friends;
        let friends_btn = button(row![text("👥").size(14), Space::with_width(Length::Fixed(8.0)), text("Friends").size(13).style(iced::theme::Text::Color(if friends_active { TEXT_WHITE } else { TEXT_MUTED }))].align_items(Alignment::Center).padding([6, 12]).width(Length::Fill)).width(Length::Fill).on_press(UiMessage::SelectView(ActiveView::Friends)).style(if friends_active { iced::theme::Button::Custom(Box::new(ActiveFlatButton)) } else { iced::theme::Button::Custom(Box::new(FlatButton)) });

        let dm_list: Vec<Element<'_, UiMessage>> = self.friends.iter().map(|f| {
            let fid = f.id; let active = self.active_view == ActiveView::DirectMessage(fid);
            let unread = self.unread.get(&fid.to_string()).copied().unwrap_or(0);
            let color = user_color_for(&f.user_id);
            let av = container(text(user_initials(&f.display_name)).size(11).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(32.0)).height(Length::Fixed(32.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 16.0 }))).center_x().center_y();
            let nc = if active || unread > 0 { TEXT_WHITE } else { TEXT_MUTED };
            let badge: Element<'_, UiMessage> = if unread > 0 { container(text(format!("{}", unread)).size(10).style(iced::theme::Text::Color(TEXT_WHITE))).padding([2, 5]).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color: RED, radius: 8.0 }))).into() } else { Space::with_width(Length::Shrink).into() };
            button(row![av, Space::with_width(Length::Fixed(8.0)), text(f.display_name.as_str()).size(13).style(iced::theme::Text::Color(nc)), Space::with_width(Length::Fill), badge].align_items(Alignment::Center).padding([5, 12]).width(Length::Fill)).width(Length::Fill).on_press(UiMessage::SelectView(ActiveView::DirectMessage(fid))).style(if active { iced::theme::Button::Custom(Box::new(ActiveFlatButton)) } else { iced::theme::Button::Custom(Box::new(FlatButton)) }).into()
        }).collect();

        let groups_hdr: Element<'_, UiMessage> = container(row![text("GROUP CHATS").size(10).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_width(Length::Fill), button(text("+").size(14).style(iced::theme::Text::Color(TEXT_MUTED))).on_press(UiMessage::OpenModal(Modal::NewGroup)).style(iced::theme::Button::Custom(Box::new(FlatButton)))].align_items(Alignment::Center).padding([8, 12, 4, 16])).width(Length::Fill).into();

        let group_list: Vec<Element<'_, UiMessage>> = self.groups.iter().map(|g| {
            let gid = g.id; let active = self.active_view == ActiveView::GroupChat(gid);
            let unread = self.unread.get(&gid.to_string()).copied().unwrap_or(0);
            let nc = if active || unread > 0 { TEXT_WHITE } else { TEXT_MUTED };
            let badge: Element<'_, UiMessage> = if unread > 0 { container(text(format!("{}", unread)).size(10).style(iced::theme::Text::Color(TEXT_WHITE))).padding([2, 5]).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color: RED, radius: 8.0 }))).into() } else { Space::with_width(Length::Shrink).into() };
            button(row![text("◎").size(12).style(iced::theme::Text::Color(nc)), Space::with_width(Length::Fixed(8.0)), text(g.name.as_str()).size(13).style(iced::theme::Text::Color(nc)), Space::with_width(Length::Fill), badge].align_items(Alignment::Center).padding([5, 12]).width(Length::Fill)).width(Length::Fill).on_press(UiMessage::SelectView(ActiveView::GroupChat(gid))).style(if active { iced::theme::Button::Custom(Box::new(ActiveFlatButton)) } else { iced::theme::Button::Custom(Box::new(FlatButton)) }).into()
        }).collect();

        let items: Vec<Element<'_, UiMessage>> = std::iter::once(friends_btn.into())
            .chain(std::iter::once(Space::with_height(Length::Fixed(4.0)).into()))
            .chain(std::iter::once(container(horizontal_rule(1)).padding([4, 12]).width(Length::Fill).into()))
            .chain(std::iter::once(groups_hdr))
            .chain(group_list.into_iter())
            .collect();

        container(column![header, Space::with_height(Length::Fixed(4.0)), scrollable(column(items).spacing(1)).height(Length::Fill)].spacing(0).height(Length::Fill)).width(Length::Fixed(240.0)).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgDark))).into()
    }

    fn view_server_sidebar(&self, sid: Uuid) -> Element<'_, UiMessage> {
        let server = match self.servers.iter().find(|s| s.id == sid) { Some(s) => s, None => return Space::with_width(Length::Fixed(240.0)).into() };
        let settings_btn: Element<'_, UiMessage> = if server.is_owned {
            button(text("⚙").size(14).style(iced::theme::Text::Color(TEXT_MUTED))).on_press(UiMessage::OpenModal(Modal::ServerInfo(sid))).style(iced::theme::Button::Custom(Box::new(FlatButton))).into()
        } else {
            Space::with_width(Length::Shrink).into()
        };
        let header = container(row![text(server.name.as_str()).size(14).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fill), settings_btn].align_items(Alignment::Center).padding([14, 16])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgDark)));
        let ch_hdr: Element<'_, UiMessage> = container(text("TEXT CHANNELS").size(10).style(iced::theme::Text::Color(TEXT_MUTED))).padding([8, 16, 4, 16]).width(Length::Fill).into();
        let chs: Vec<Element<'_, UiMessage>> = server.channels.iter().map(|ch| {
            let cid = ch.id; let active = self.active_view == ActiveView::Channel(sid, cid);
            let unread = self.unread.get(&cid.to_string()).copied().unwrap_or(0);
            let col = if active || unread > 0 { TEXT_WHITE } else { TEXT_MUTED };
            let badge: Element<'_, UiMessage> = if unread > 0 { container(text(format!("{}", unread)).size(10).style(iced::theme::Text::Color(TEXT_WHITE))).padding([2, 5]).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color: RED, radius: 8.0 }))).into() } else { Space::with_width(Length::Shrink).into() };
            button(row![text("#").size(14).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_width(Length::Fixed(6.0)), text(ch.name.as_str()).size(13).style(iced::theme::Text::Color(col)), Space::with_width(Length::Fill), badge].align_items(Alignment::Center).padding([5, 12]).width(Length::Fill)).width(Length::Fill).on_press(UiMessage::SelectView(ActiveView::Channel(sid, cid))).style(if active { iced::theme::Button::Custom(Box::new(ActiveFlatButton)) } else { iced::theme::Button::Custom(Box::new(FlatButton)) }).into()
        }).collect();
        let mc = text(format!("{} member(s)", server.member_ids.len())).size(10).style(iced::theme::Text::Color(TEXT_MUTED));
        container(column![header, Space::with_height(Length::Fixed(2.0)), scrollable(column(std::iter::once(ch_hdr).chain(chs.into_iter()).collect::<Vec<_>>()).spacing(1)).height(Length::Fill), container(column![horizontal_rule(1), container(mc).padding([8, 16])]).width(Length::Fill)].spacing(0).height(Length::Fill)).width(Length::Fixed(240.0)).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgDark))).into()
    }

    // ─── Chat ─────────────────────────────────────────────────────────────────

    fn view_chat(&self) -> Element<'_, UiMessage> {
        match &self.active_view {
            ActiveView::Friends => self.view_friends_page(),
            ActiveView::DirectMessage(fid) => self.view_dm(*fid),
            ActiveView::GroupChat(gid) => self.view_group(*gid),
            ActiveView::Channel(sid, cid) => self.view_channel(*sid, *cid),
        }
    }

    fn view_friends_page(&self) -> Element<'_, UiMessage> {
        let hdr = container(row![text("  Friends").size(16).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fill), button(text("  + Add Friend  ").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).on_press(UiMessage::OpenModal(Modal::AddFriend)).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].align_items(Alignment::Center).padding([14, 20])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(MessageHeaderStyle)));
        let rows: Vec<Element<'_, UiMessage>> = self.friends.iter().map(|f| {
            let fid = f.id; let color = user_color_for(&f.user_id);
            let av = container(text(user_initials(&f.display_name)).size(16).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(44.0)).height(Length::Fixed(44.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 22.0 }))).center_x().center_y();
            container(row![av, Space::with_width(Length::Fixed(12.0)), column![text(f.display_name.as_str()).size(14).style(iced::theme::Text::Color(TEXT_WHITE)), text(short_id(&f.user_id)).size(11).style(iced::theme::Text::Color(TEXT_MUTED))].spacing(2), Space::with_width(Length::Fill), button(text("Message").size(12).style(iced::theme::Text::Color(TEXT_WHITE))).on_press(UiMessage::SelectView(ActiveView::DirectMessage(fid))).style(iced::theme::Button::Custom(Box::new(BlurpleButton))), Space::with_width(Length::Fixed(8.0))].align_items(Alignment::Center).padding([10, 20]).width(Length::Fill)).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgMain))).into()
        }).collect();
        let body: Element<'_, UiMessage> = if self.friends.is_empty() { container(column![text("No friends yet!").size(18).style(iced::theme::Text::Color(TEXT_MUTED)), text("Click + Add Friend and paste their User ID.").size(13).style(iced::theme::Text::Color(TEXT_MUTED))].align_items(Alignment::Center).spacing(8)).width(Length::Fill).height(Length::Fill).center_x().center_y().into() } else { scrollable(column(rows).spacing(1).padding([8, 0])).height(Length::Fill).into() };
        let footer = container(column![horizontal_rule(1), container(row![text("Your ID:  ").size(11).style(iced::theme::Text::Color(TEXT_MUTED)), text(short_id(&self.my_user_id)).size(11).style(iced::theme::Text::Color(BLURPLE)), Space::with_width(Length::Fill), text("(share with contacts)").size(10).style(iced::theme::Text::Color(TEXT_MUTED))].align_items(Alignment::Center)).padding([8, 20])]).width(Length::Fill);
        container(column![hdr, body, footer].spacing(0)).width(Length::Fill).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgMain))).into()
    }

    fn view_dm(&self, fid: Uuid) -> Element<'_, UiMessage> {
        let friend = match self.friends.iter().find(|f| f.id == fid) { Some(f) => f, None => return self.view_friends_page() };
        let color = user_color_for(&friend.user_id);
        let av = container(text(user_initials(&friend.display_name)).size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(32.0)).height(Length::Fixed(32.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 16.0 }))).center_x().center_y();
        let hdr = container(row![av, Space::with_width(Length::Fixed(10.0)), text(friend.display_name.as_str()).size(15).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fill), text(self.tor_status.label()).size(11).style(iced::theme::Text::Color(TEXT_MUTED))].align_items(Alignment::Center).padding([10, 20])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(MessageHeaderStyle)));
        let rows = if let Some(msgs) = self.conversations.get(&fid) {
            self.render_dms(msgs.as_slice(), &friend.user_id)
        } else {
            empty_chat("No messages yet. Say hello!")
        };
        let compose = self.compose_bar(&format!("Message @{}", friend.display_name));
        container(column![hdr, rows, compose].spacing(0)).width(Length::Fill).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgMain))).into()
    }

    fn view_group(&self, gid: Uuid) -> Element<'_, UiMessage> {
        let group = match self.groups.iter().find(|g| g.id == gid) { Some(g) => g, None => return self.view_friends_page() };
        let hdr = container(row![text("◎  ").size(16).style(iced::theme::Text::Color(BLURPLE)), text(group.name.as_str()).size(15).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fixed(10.0)), text(format!("{} members", group.members.len())).size(12).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_width(Length::Fill), button(text("+ Member").size(12).style(iced::theme::Text::Color(TEXT_WHITE))).on_press(UiMessage::OpenModal(Modal::GroupAddMember(gid))).style(iced::theme::Button::Custom(Box::new(BlurpleButton))), Space::with_width(Length::Fixed(8.0))].align_items(Alignment::Center).padding([10, 20])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(MessageHeaderStyle)));
        let rows = self.render_group_msgs(&group.messages);
        let compose = self.compose_bar(&format!("Message #{}", group.name));
        container(column![hdr, rows, compose].spacing(0)).width(Length::Fill).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgMain))).into()
    }

    fn view_channel(&self, sid: Uuid, cid: Uuid) -> Element<'_, UiMessage> {
        let server = match self.servers.iter().find(|s| s.id == sid) { Some(s) => s, None => return self.view_friends_page() };
        let ch = match server.channels.iter().find(|c| c.id == cid) { Some(c) => c, None => return self.view_friends_page() };
        let hdr = container(row![text("#  ").size(16).style(iced::theme::Text::Color(TEXT_MUTED)), text(ch.name.as_str()).size(15).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fill), text(format!("{} members · {}", server.member_ids.len(), server.name.as_str())).size(11).style(iced::theme::Text::Color(TEXT_MUTED))].align_items(Alignment::Center).padding([10, 20])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(MessageHeaderStyle)));
        let rows = self.render_ch_msgs(&ch.messages);
        let compose = self.compose_bar(&format!("Message #{}", ch.name));
        container(column![hdr, rows, compose].spacing(0)).width(Length::Fill).height(Length::Fill).style(iced::theme::Container::Custom(Box::new(BgMain))).into()
    }

    fn render_dms<'a>(&'a self, msgs: &'a [DirectMessage], peer_uid: &'a str) -> Element<'a, UiMessage> {
        if msgs.is_empty() { return empty_chat("No messages yet. Say hello!"); }
        let rows: Vec<Element<'_, UiMessage>> = msgs.iter().map(|m| {
            let is_mine = m.outgoing;
            let name = if is_mine { self.my_name.as_str() } else { self.friends.iter().find(|f| f.user_id == m.from_id).map(|f| f.display_name.as_str()).unwrap_or("Unknown") };
            let uid = if is_mine { self.my_user_id.as_str() } else { peer_uid };
            self.msg_row(name, uid, &m.body, m.timestamp)
        }).collect();
        scrollable(column(rows).spacing(0).padding([8, 0, 8, 0])).height(Length::Fill).into()
    }

    fn render_group_msgs<'a>(&'a self, msgs: &'a [GroupMessage]) -> Element<'a, UiMessage> {
        if msgs.is_empty() { return empty_chat("No messages yet."); }
        let rows: Vec<Element<'_, UiMessage>> = msgs.iter().map(|m| self.msg_row(&m.from_name, &m.from_id, &m.body, m.timestamp)).collect();
        scrollable(column(rows).spacing(0).padding([8, 0, 8, 0])).height(Length::Fill).into()
    }

    fn render_ch_msgs<'a>(&'a self, msgs: &'a [ChannelMessage]) -> Element<'a, UiMessage> {
        if msgs.is_empty() { return empty_chat("Be the first to send a message!"); }
        let rows: Vec<Element<'_, UiMessage>> = msgs.iter().map(|m| self.msg_row(&m.from_name, &m.from_id, &m.body, m.timestamp)).collect();
        scrollable(column(rows).spacing(0).padding([8, 0, 8, 0])).height(Length::Fill).into()
    }

    fn msg_row<'a>(&'a self, name: &'a str, uid: &'a str, body: &'a str, ts: u64) -> Element<'a, UiMessage> {
        let color = user_color_for(uid);
        let av = container(text(user_initials(name)).size(12).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(36.0)).height(Length::Fixed(36.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 18.0 }))).center_x().center_y();
        let nc = if uid == self.my_user_id { BLURPLE } else { color };
        row![av, Space::with_width(Length::Fixed(12.0)), column![row![text(name).size(13).style(iced::theme::Text::Color(nc)), Space::with_width(Length::Fixed(8.0)), text(format_ts(ts)).size(10).style(iced::theme::Text::Color(TEXT_MUTED))].align_items(Alignment::Center).spacing(0), text(body).size(14).style(iced::theme::Text::Color(TEXT_NORMAL))].spacing(2)].align_items(Alignment::Start).padding([6, 16]).width(Length::Fill).into()
    }

    fn compose_bar(&self, placeholder: &str) -> Element<'_, UiMessage> {
        container(row![text_input(placeholder, &self.compose_text).on_input(UiMessage::ComposeChanged).on_submit(UiMessage::SendMessage).size(14).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_width(Length::Fixed(8.0)), button(text("Send").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).on_press(UiMessage::SendMessage).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].align_items(Alignment::Center).padding([10, 16])).width(Length::Fill).style(iced::theme::Container::Custom(Box::new(ComposeBarStyle))).into()
    }

    // ─── Modals ───────────────────────────────────────────────────────────────

    fn view_modal(&self) -> Element<'_, UiMessage> {
        match &self.modal {
            Modal::None => Space::with_height(Length::Shrink).into(),
            Modal::AddFriend => self.modal_add_friend(),
            Modal::NewGroup => self.modal_new_group(),
            Modal::NewServer => self.modal_new_server(),
            Modal::JoinServer => self.modal_join_server(),
            Modal::Profile => self.modal_profile(),
            Modal::MigrateDevice => self.modal_migrate(),
            Modal::GroupAddMember(gid) => self.modal_add_member(*gid),
            Modal::ServerInfo(sid) => self.modal_server_info(*sid),
        }
    }

    fn modal_wrap<'a>(&'a self, title: &'a str, body: Element<'a, UiMessage>) -> Element<'a, UiMessage> {
        container(column![row![text(title).size(18).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_width(Length::Fill), button(text("✕").size(14).style(iced::theme::Text::Color(TEXT_MUTED))).on_press(UiMessage::CloseModal).style(iced::theme::Button::Custom(Box::new(FlatButton)))].align_items(Alignment::Center), Space::with_height(Length::Fixed(16.0)), body].padding(24).width(Length::Fixed(440.0)).spacing(0)).style(iced::theme::Container::Custom(Box::new(CardStyle))).into()
    }

    fn modal_add_friend(&self) -> Element<'_, UiMessage> {
        let b = column![lbl("Their User ID"), Space::with_height(Length::Fixed(4.0)), text_input("onion address or fingerprint", &self.modal_f1).on_input(UiMessage::Field1Changed).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(12.0)), lbl("Display Name"), Space::with_height(Length::Fixed(4.0)), text_input("What to call them", &self.modal_f2).on_input(UiMessage::Field2Changed).on_submit(UiMessage::SubmitAddFriend).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(16.0)), err_row(self.modal_err.as_deref()), button(container(text("Add Friend").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::SubmitAddFriend).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].spacing(0).into();
        self.modal_wrap("Add Friend", b)
    }

    fn modal_new_group(&self) -> Element<'_, UiMessage> {
        let b = column![lbl("Group Name"), Space::with_height(Length::Fixed(4.0)), text_input("e.g. The Team", &self.modal_f1).on_input(UiMessage::Field1Changed).on_submit(UiMessage::SubmitNewGroup).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(16.0)), err_row(self.modal_err.as_deref()), button(container(text("Create Group").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::SubmitNewGroup).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].spacing(0).into();
        self.modal_wrap("New Group Chat", b)
    }

    fn modal_add_member(&self, gid: Uuid) -> Element<'_, UiMessage> {
        let b = column![lbl("User ID"), Space::with_height(Length::Fixed(4.0)), text_input("Their onion address", &self.modal_f1).on_input(UiMessage::Field1Changed).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(12.0)), lbl("Display Name (optional)"), Space::with_height(Length::Fixed(4.0)), text_input("Their name", &self.modal_f2).on_input(UiMessage::Field2Changed).on_submit(UiMessage::AddMemberToGroup(gid)).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(16.0)), err_row(self.modal_err.as_deref()), button(container(text("Add to Group").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::AddMemberToGroup(gid)).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].spacing(0).into();
        self.modal_wrap("Add Member", b)
    }

    fn modal_new_server(&self) -> Element<'_, UiMessage> {
        let b = column![lbl("Server Name"), Space::with_height(Length::Fixed(4.0)), text_input("e.g. My Community", &self.modal_f1).on_input(UiMessage::Field1Changed).on_submit(UiMessage::SubmitNewServer).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(12.0)), text("A #general channel will be created automatically.").size(12).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_height(Length::Fixed(16.0)), err_row(self.modal_err.as_deref()), button(container(text("Create Server").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::SubmitNewServer).style(iced::theme::Button::Custom(Box::new(BlurpleButton))), Space::with_height(Length::Fixed(8.0)), horizontal_rule(1), Space::with_height(Length::Fixed(8.0)), button(container(text("Join a Server Instead").size(13).style(iced::theme::Text::Color(TEXT_MUTED))).width(Length::Fill).center_x().padding([6, 0])).width(Length::Fill).on_press(UiMessage::OpenModal(Modal::JoinServer)).style(iced::theme::Button::Custom(Box::new(GhostButton)))].spacing(0).into();
        self.modal_wrap("Create a Server", b)
    }

    fn modal_join_server(&self) -> Element<'_, UiMessage> {
        let b = column![lbl("Server Code"), Space::with_height(Length::Fixed(4.0)), text_input("8-character invite code", &self.modal_f1).on_input(UiMessage::Field1Changed).on_submit(UiMessage::SubmitJoinServer).size(13).width(Length::Fill).style(iced::theme::TextInput::Custom(Box::new(DiscordInput))), Space::with_height(Length::Fixed(12.0)), text("Get the code from the server owner.").size(12).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_height(Length::Fixed(16.0)), err_row(self.modal_err.as_deref()), button(container(text("Join Server").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::SubmitJoinServer).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].spacing(0).into();
        self.modal_wrap("Join a Server", b)
    }

    fn modal_server_info(&self, sid: Uuid) -> Element<'_, UiMessage> {
        let server = match self.servers.iter().find(|s| s.id == sid) { Some(s) => s, None => return Space::with_height(Length::Shrink).into() };
        let b = column![text(server.name.as_str()).size(16).style(iced::theme::Text::Color(TEXT_WHITE)), Space::with_height(Length::Fixed(12.0)), lbl("Server Code (share to invite)"), Space::with_height(Length::Fixed(4.0)), container(text(server.server_code.as_str()).size(24).style(iced::theme::Text::Color(BLURPLE))).padding([8, 16]), Space::with_height(Length::Fixed(16.0)), text(format!("{} member(s)  ·  {} channel(s)", server.member_ids.len(), server.channels.len())).size(12).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_height(Length::Fixed(16.0)), button(container(text("Close").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::CloseModal).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].spacing(0).into();
        self.modal_wrap("Server Info", b)
    }

    fn modal_profile(&self) -> Element<'_, UiMessage> {
        let color = user_color_for(&self.my_user_id);
        let av = container(text(user_initials(&self.my_name)).size(28).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fixed(72.0)).height(Length::Fixed(72.0)).style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 36.0 }))).center_x().center_y();
        let tor_text = match &self.tor_status { P2PStatus::TorReady { onion } => format!("Tor: {}", onion), P2PStatus::DirectMode => "Direct mode (no Tor)".into(), P2PStatus::Error(e) => format!("Error: {}", e), _ => "Connecting...".into() };
        let b = column![container(av).center_x().width(Length::Fill), Space::with_height(Length::Fixed(12.0)), container(text(self.my_name.as_str()).size(18).style(iced::theme::Text::Color(TEXT_WHITE))).center_x().width(Length::Fill), Space::with_height(Length::Fixed(16.0)), horizontal_rule(1), Space::with_height(Length::Fixed(12.0)), lbl("Your User ID"), Space::with_height(Length::Fixed(4.0)), text(self.my_user_id.as_str()).size(11).style(iced::theme::Text::Color(BLURPLE)), Space::with_height(Length::Fixed(12.0)), lbl("Network"), Space::with_height(Length::Fixed(4.0)), text(tor_text).size(12).style(iced::theme::Text::Color(TEXT_MUTED)), Space::with_height(Length::Fixed(20.0)), button(container(text("Migrate Device").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::OpenModal(Modal::MigrateDevice)).style(iced::theme::Button::Custom(Box::new(GhostButton))), Space::with_height(Length::Fixed(8.0)), button(container(text("Close").size(13).style(iced::theme::Text::Color(TEXT_MUTED))).width(Length::Fill).center_x().padding([6, 0])).width(Length::Fill).on_press(UiMessage::CloseModal).style(iced::theme::Button::Custom(Box::new(FlatButton)))].spacing(0).into();
        self.modal_wrap("Your Profile", b)
    }

    fn modal_migrate(&self) -> Element<'_, UiMessage> {
        let vp = self.vault_path.display().to_string();
        let b = column![text("To migrate Null Chat to another device:").size(14).style(iced::theme::Text::Color(TEXT_NORMAL)), Space::with_height(Length::Fixed(16.0)), lbl("Step 1 — Copy vault"), Space::with_height(Length::Fixed(4.0)), container(text(format!("rsync -av {}/ user@newhost:~/.local/share/null-chat/vault/", vp)).size(11).style(iced::theme::Text::Color(BLURPLE))).padding([6, 10]), Space::with_height(Length::Fixed(12.0)), lbl("Step 2 — Install Null Chat on the new device"), Space::with_height(Length::Fixed(12.0)), lbl("Step 3 — Unlock with the same passphrase"), Space::with_height(Length::Fixed(16.0)), text("⚠  Without your passphrase, the vault cannot be recovered.").size(11).style(iced::theme::Text::Color(YELLOW)), Space::with_height(Length::Fixed(16.0)), button(container(text("Close").size(13).style(iced::theme::Text::Color(TEXT_WHITE))).width(Length::Fill).center_x().padding([8, 0])).width(Length::Fill).on_press(UiMessage::CloseModal).style(iced::theme::Button::Custom(Box::new(BlurpleButton)))].spacing(0).into();
        self.modal_wrap("Migrate Device", b)
    }
}

// ─── Overlay ─────────────────────────────────────────────────────────────────

struct Overlay;
impl iced::widget::container::StyleSheet for Overlay {
    type Style = iced::Theme;
    fn appearance(&self, _: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance { background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.8 })), ..Default::default() }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn lbl<'a>(s: &'a str) -> Element<'a, UiMessage> {
    text(s).size(12).style(iced::theme::Text::Color(TEXT_MUTED)).into()
}

fn err_row<'a>(e: Option<&'a str>) -> Element<'a, UiMessage> {
    match e {
        Some(msg) => container(row![text("⚠  ").size(12).style(iced::theme::Text::Color(RED)), text(msg).size(12).style(iced::theme::Text::Color(RED))]).padding([0, 0, 8, 0]).into(),
        None => Space::with_height(Length::Fixed(0.0)).into(),
    }
}

fn empty_chat<'a>(msg: &'a str) -> Element<'a, UiMessage> {
    container(text(msg).size(13).style(iced::theme::Text::Color(TEXT_MUTED))).width(Length::Fill).height(Length::Fill).center_x().center_y().into()
}

fn pass_trivial(p: &str) -> bool {
    !(p.chars().any(|c| c.is_uppercase()) && p.chars().any(|c| c.is_lowercase()) && p.chars().any(|c| c.is_ascii_digit()))
}
