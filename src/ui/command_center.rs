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
    now_unix, format_ts, user_color_for, user_initials,
    generate_server_code, short_id,
    Channel, ChannelMessage, ChannelType, DirectMessage, Friend, GroupChat,
    GroupMember, GroupMessage, GroupRole, ModerationAction as ModAction,
    Notification, NotifKind, Server, ServerMember, ServerRole,
    UserStatus, WireKind, WireMessage,
};
use crate::network::p2p::P2PStatus;
use crate::storage::vault::EncryptedVault;
use crate::ui::theme::{
    AvatarContainer, RoleBadge, UnreadBadge, InlineTagStyle,
    CardStyle, UnlockCardStyle,
    MessageHeaderStyle, ComposeBarStyle, StatusBarStyle, MemberCardStyle,
    NotifInfo, NotifSuccess, NotifError, NotifWarn,
    StaffChannelStyle, AnnouncementChannelStyle,
    BlurpleButton, DangerButton, SuccessButton, GhostButton, FlatButton,
    ActiveFlatButton, ServerIconButton, ActiveServerIconButton, IconButton,
    DestructiveFlatButton, DiscordInput, SlimScrollbar,
    BG_DARKEST, BG_DARK, BG_MAIN, DIVIDER,
    TEXT_NORMAL, TEXT_MUTED, TEXT_WHITE, BLURPLE, GREEN, RED, YELLOW, ORANGE,
};
use iced::Background;

const MIN_PASS: usize = 12;

// ─── Enums ────────────────────────────────────────────────────────────────────

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
    None,
    AddFriend,
    NewGroup,
    NewServer,
    JoinServer,
    Profile,
    EditProfile,
    MigrateDevice,
    GroupAddMember(Uuid),
    ServerInfo(Uuid),
    EditServer(Uuid),
    CreateChannel(Uuid),
    ManageMembers(Uuid),
    MemberDetail { context_id: Uuid, user_id: String, is_server: bool },
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
    Field3Changed(String),
    SelectChanged(usize),
    SubmitAddFriend,
    SubmitNewGroup,
    AddMemberToGroup(Uuid),
    SubmitNewServer,
    SubmitJoinServer,
    CreateChannel(Uuid),
    SaveServerEdit(Uuid),
    SetRole { context_id: Uuid, user_id: String, role_label: String, is_server: bool },
    KickUser { context_id: Uuid, user_id: String, is_server: bool },
    BanUser { context_id: Uuid, user_id: String, is_server: bool },
    UnbanUser { context_id: Uuid, user_id: String },
    MuteUser { context_id: Uuid, user_id: String, is_server: bool },
    UnmuteUser { context_id: Uuid, user_id: String, is_server: bool },
    SaveProfile,
    SetMyStatus(UserStatus),
    IncomingP2P(WireMessage),
    TorStatusChanged(P2PStatus),
    SearchChanged(String),
    DismissNotif(Uuid),
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
    Setup {
        passphrase: String,
        confirm: String,
        display_name: String,
        error: Option<String>,
    },
    Unlock {
        passphrase: String,
        error: Option<String>,
    },
    Main,
}

// ─── CommandCenter ────────────────────────────────────────────────────────────

pub struct CommandCenter {
    phase: AppPhase,
    vault_path: PathBuf,
    my_name: String,
    my_user_id: String,
    my_nick: Option<String>,
    my_status: UserStatus,
    my_bio: Option<String>,
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
    modal_f3: String,
    modal_select: usize,
    modal_err: Option<String>,
    notifications: Vec<Notification>,
    unread: HashMap<String, u32>,
    search_query: String,
}

impl CommandCenter {
    pub fn new() -> (Self, Command<UiMessage>) {
        let vault_path = EncryptedVault::default_path();
        let is_first = EncryptedVault::is_first_run(&vault_path);
        let fp = crate::crypto::identity::LocalIdentity::generate().fingerprint_hex();

        let phase = if is_first {
            AppPhase::Setup {
                passphrase: String::new(),
                confirm: String::new(),
                display_name: String::new(),
                error: None,
            }
        } else {
            AppPhase::Unlock { passphrase: String::new(), error: None }
        };

        let cc = CommandCenter {
            phase,
            vault_path,
            my_name: String::from("Anonymous"),
            my_user_id: fp,
            my_nick: None,
            my_status: UserStatus::Online,
            my_bio: None,
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
            modal_f3: String::new(),
            modal_select: 0,
            modal_err: None,
            notifications: Vec::new(),
            unread: HashMap::new(),
            search_query: String::new(),
        };
        (cc, Command::none())
    }

    pub fn incoming_queue(&self) -> Arc<Mutex<Vec<WireMessage>>> {
        self.incoming_queue.clone()
    }

    fn display_name(&self) -> &str {
        self.my_nick.as_deref().unwrap_or(&self.my_name)
    }

    fn push_notif(&mut self, n: Notification) {
        self.notifications.push(n);
        if self.notifications.len() > 5 {
            self.notifications.remove(0);
        }
    }

    fn cmd_init_p2p(&self) -> Command<UiMessage> {
        let data_dir = self.vault_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let queue = self.incoming_queue.clone();
        Command::perform(
            async move { crate::network::p2p::init_p2p(data_dir, queue).await },
            |(status, socks)| UiMessage::TorStatusChanged(status),
        )
    }

    fn my_role_in_server(&self, sid: Uuid) -> ServerRole {
        self.servers
            .iter()
            .find(|s| s.id == sid)
            .map(|s| s.my_role(&self.my_user_id))
            .unwrap_or(ServerRole::Member)
    }

    fn my_role_in_group(&self, gid: Uuid) -> GroupRole {
        self.groups
            .iter()
            .find(|g| g.id == gid)
            .and_then(|g| g.members.iter().find(|m| m.user_id == self.my_user_id))
            .map(|m| m.role.clone())
            .unwrap_or(GroupRole::Member)
    }

    fn am_muted_in_server(&self, sid: Uuid) -> bool {
        self.servers
            .iter()
            .find(|s| s.id == sid)
            .and_then(|s| s.members.iter().find(|m| m.user_id == self.my_user_id))
            .map(|m| m.muted)
            .unwrap_or(false)
    }

    fn am_muted_in_group(&self, gid: Uuid) -> bool {
        self.groups
            .iter()
            .find(|g| g.id == gid)
            .and_then(|g| g.members.iter().find(|m| m.user_id == self.my_user_id))
            .map(|m| m.muted)
            .unwrap_or(false)
    }

    // ─── Update ──────────────────────────────────────────────────────────────

    pub fn update(&mut self, msg: UiMessage) -> Command<UiMessage> {
        match msg {
            UiMessage::SetupPassChanged(v) | UiMessage::SetupPassphraseChanged(v) => {
                if let AppPhase::Setup { passphrase, error, .. } = &mut self.phase {
                    *passphrase = v;
                    *error = None;
                }
            }
            UiMessage::SetupConfirmChanged(v) | UiMessage::SetupConfirmChanged2(v) => {
                if let AppPhase::Setup { confirm, error, .. } = &mut self.phase {
                    *confirm = v;
                    *error = None;
                }
            }
            UiMessage::SetupNameChanged(v) => {
                if let AppPhase::Setup { display_name, .. } = &mut self.phase {
                    *display_name = v;
                }
            }
            UiMessage::SetupCreate | UiMessage::SetupCreateAccount => {
                if let AppPhase::Setup { passphrase, confirm, display_name, error } = &mut self.phase {
                    let p = passphrase.clone();
                    let c = confirm.clone();
                    let n = display_name.clone();
                    if n.trim().is_empty() {
                        *error = Some("Enter a display name.".into());
                    } else if p.len() < MIN_PASS {
                        *error = Some(format!("Passphrase must be ≥{} characters.", MIN_PASS));
                    } else if p != c {
                        *error = Some("Passphrases don't match.".into());
                    } else if pass_trivial(&p) {
                        *error = Some("Use uppercase, lowercase and digits.".into());
                    } else {
                        let mut vault = EncryptedVault::new();
                        match vault.open(&self.vault_path.clone(), &p) {
                            Ok(()) => {
                                self.my_name = n;
                                self.phase = AppPhase::Main;
                                return self.cmd_init_p2p();
                            }
                            Err(e) => {
                                *error = Some(format!("Vault error: {}", e));
                            }
                        }
                    }
                }
            }
            UiMessage::UnlockPassChanged(v) | UiMessage::UnlockPassphraseChanged(v) => {
                if let AppPhase::Unlock { passphrase, error } = &mut self.phase {
                    *passphrase = v;
                    *error = None;
                }
            }
            UiMessage::UnlockVault => {
                if let AppPhase::Unlock { passphrase, error } = &mut self.phase {
                    let p = passphrase.clone();
                    if p.is_empty() {
                        *error = Some("Enter your passphrase.".into());
                    } else {
                        let mut vault = EncryptedVault::new();
                        match vault.open(&self.vault_path.clone(), &p) {
                            Ok(()) => {
                                self.phase = AppPhase::Main;
                                return self.cmd_init_p2p();
                            }
                            Err(crate::storage::vault::VaultError::Decryption) => {
                                *error = Some("Incorrect passphrase.".into());
                            }
                            Err(e) => {
                                *error = Some(format!("Vault error: {}", e));
                            }
                        }
                    }
                }
            }
            UiMessage::SelectPane(p) => {
                self.selected_pane = p;
                self.active_view = ActiveView::Friends;
                self.modal = Modal::None;
            }
            UiMessage::SelectView(v) => {
                let key = match &v {
                    ActiveView::DirectMessage(id) => id.to_string(),
                    ActiveView::GroupChat(id) => id.to_string(),
                    ActiveView::Channel(_, cid) => cid.to_string(),
                    _ => String::new(),
                };
                if !key.is_empty() { self.unread.remove(&key); }
                self.active_view = v;
                self.modal = Modal::None;
            }
            UiMessage::OpenModal(m) => {
                // Pre-fill profile fields
                if matches!(m, Modal::Profile | Modal::EditProfile) {
                    self.modal_f1 = self.my_name.clone();
                    self.modal_f2 = self.my_nick.clone().unwrap_or_default();
                    self.modal_f3 = self.my_bio.clone().unwrap_or_default();
                }
                if let Modal::EditServer(sid) = &m {
                    if let Some(s) = self.servers.iter().find(|s| s.id == *sid) {
                        self.modal_f1 = s.name.clone();
                        self.modal_f2 = s.description.clone().unwrap_or_default();
                    }
                }
                self.modal_err = None;
                self.modal = m;
            }
            UiMessage::CloseModal => {
                self.modal = Modal::None;
                self.modal_f1.clear();
                self.modal_f2.clear();
                self.modal_f3.clear();
                self.modal_err = None;
            }
            UiMessage::ComposeChanged(v) | UiMessage::MessageInputChanged(v) => {
                self.compose_text = v;
            }
            UiMessage::SendMessage | UiMessage::MessageSendRequested => {
                let body = self.compose_text.trim().to_string();
                if body.is_empty() { return Command::none(); }
                self.compose_text.clear();
                let from_id = self.my_user_id.clone();
                let from_name = self.display_name().to_string();
                let ts = now_unix();
                match self.active_view.clone() {
                    ActiveView::DirectMessage(fid) => {
                        self.conversations.entry(fid).or_default().push(DirectMessage {
                            id: Uuid::new_v4(),
                            from_id: from_id.clone(),
                            body: body.clone(),
                            timestamp: ts,
                            outgoing: true,
                            edited: false,
                            reactions: Vec::new(),
                        });
                        if let Some(friend) = self.friends.iter().find(|f| f.id == fid) {
                            let peer = friend.user_id.clone();
                            let socks = self.tor_socks.clone();
                            let wire = WireMessage {
                                kind: WireKind::DirectMessage,
                                from_id,
                                from_name,
                                target_id: peer.clone(),
                                body,
                                timestamp: ts,
                            };
                            return Command::perform(
                                async move {
                                    crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await.ok();
                                },
                                |_| UiMessage::CloseModal,
                            );
                        }
                    }
                    ActiveView::GroupChat(gid) => {
                        if !self.am_muted_in_group(gid) {
                            if let Some(g) = self.groups.iter_mut().find(|x| x.id == gid) {
                                g.messages.push(GroupMessage {
                                    id: Uuid::new_v4(),
                                    from_id: from_id.clone(),
                                    from_name: from_name.clone(),
                                    body: body.clone(),
                                    timestamp: ts,
                                    edited: false,
                                    reactions: Vec::new(),
                                });
                                let members: Vec<String> = g.members.iter()
                                    .map(|m| m.user_id.clone())
                                    .filter(|uid| uid != &from_id)
                                    .collect();
                                let gid_str = gid.to_string();
                                let socks = self.tor_socks.clone();
                                let wire = WireMessage {
                                    kind: WireKind::GroupMessage { group_id: gid_str.clone() },
                                    from_id,
                                    from_name,
                                    target_id: gid_str,
                                    body,
                                    timestamp: ts,
                                };
                                return Command::perform(
                                    async move {
                                        for peer in members {
                                            crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await.ok();
                                        }
                                    },
                                    |_| UiMessage::CloseModal,
                                );
                            }
                        }
                    }
                    ActiveView::Channel(sid, cid) => {
                        let my_role = self.my_role_in_server(sid);
                        let can_send = if let Some(s) = self.servers.iter().find(|s| s.id == sid) {
                            if let Some(ch) = s.channels.iter().find(|c| c.id == cid) {
                                s.can_send_in(ch, &my_role)
                            } else { false }
                        } else { false };
                        if can_send && !self.am_muted_in_server(sid) {
                            if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                                if let Some(ch) = s.channels.iter_mut().find(|c| c.id == cid) {
                                    ch.messages.push(ChannelMessage {
                                        id: Uuid::new_v4(),
                                        from_id: from_id.clone(),
                                        from_name: from_name.clone(),
                                        body: body.clone(),
                                        timestamp: ts,
                                        edited: false,
                                        reactions: Vec::new(),
                                    });
                                }
                                let members: Vec<String> = s.members.iter()
                                    .map(|m| m.user_id.clone())
                                    .filter(|uid| uid != &from_id)
                                    .collect();
                                let sid_str = sid.to_string();
                                let cid_str = cid.to_string();
                                let socks = self.tor_socks.clone();
                                let wire = WireMessage {
                                    kind: WireKind::ChannelMessage {
                                        server_id: sid_str,
                                        channel_id: cid_str.clone(),
                                    },
                                    from_id,
                                    from_name,
                                    target_id: cid_str,
                                    body,
                                    timestamp: ts,
                                };
                                return Command::perform(
                                    async move {
                                        for peer in members {
                                            crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await.ok();
                                        }
                                    },
                                    |_| UiMessage::CloseModal,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            UiMessage::Field1Changed(v) => { self.modal_f1 = v; self.modal_err = None; }
            UiMessage::Field2Changed(v) => { self.modal_f2 = v; self.modal_err = None; }
            UiMessage::Field3Changed(v) => { self.modal_f3 = v; self.modal_err = None; }
            UiMessage::SelectChanged(i) => { self.modal_select = i; }
            UiMessage::SubmitAddFriend => {
                let uid = self.modal_f1.trim().to_string();
                let name = self.modal_f2.trim().to_string();
                if uid.is_empty() {
                    self.modal_err = Some("Paste their User ID.".into());
                } else if name.is_empty() {
                    self.modal_err = Some("Enter a display name.".into());
                } else if self.friends.iter().any(|f| f.user_id == uid) {
                    self.modal_err = Some("Already in your friends list.".into());
                } else {
                    self.friends.push(Friend {
                        id: Uuid::new_v4(),
                        display_name: name,
                        user_id: uid,
                        note: None,
                        added_at: now_unix(),
                        last_seen: None,
                    });
                    self.push_notif(Notification::success("Friend added!"));
                    self.modal = Modal::None;
                }
            }
            UiMessage::SubmitNewGroup => {
                let name = self.modal_f1.trim().to_string();
                if name.is_empty() {
                    self.modal_err = Some("Enter a group name.".into());
                } else {
                    let gid = Uuid::new_v4();
                    self.groups.push(GroupChat {
                        id: gid,
                        name,
                        description: if self.modal_f2.is_empty() { None } else { Some(self.modal_f2.clone()) },
                        owner_id: self.my_user_id.clone(),
                        members: vec![GroupMember {
                            user_id: self.my_user_id.clone(),
                            display_name: self.display_name().to_string(),
                            role: GroupRole::Owner,
                            muted: false,
                            banned: false,
                            joined_at: now_unix(),
                        }],
                        messages: Vec::new(),
                        created_at: now_unix(),
                    });
                    self.modal = Modal::None;
                    self.active_view = ActiveView::GroupChat(gid);
                    self.push_notif(Notification::success("Group created!"));
                }
            }
            UiMessage::AddMemberToGroup(gid) => {
                let uid = self.modal_f1.trim().to_string();
                let uname = self.modal_f2.trim().to_string();
                let role_idx = self.modal_select;
                if uid.is_empty() {
                    self.modal_err = Some("Enter a User ID.".into());
                } else {
                    if let Some(g) = self.groups.iter_mut().find(|x| x.id == gid) {
                        if !g.members.iter().any(|m| m.user_id == uid) {
                            let nm = if uname.is_empty() { short_id(&uid) } else { uname };
                            let role = match role_idx {
                                2 => GroupRole::Admin,
                                1 => GroupRole::Moderator,
                                _ => GroupRole::Member,
                            };
                            g.members.push(GroupMember {
                                user_id: uid,
                                display_name: nm,
                                role,
                                muted: false,
                                banned: false,
                                joined_at: now_unix(),
                            });
                        }
                    }
                    self.modal = Modal::None;
                    self.push_notif(Notification::success("Member added!"));
                }
            }
            UiMessage::SubmitNewServer => {
                let name = self.modal_f1.trim().to_string();
                if name.is_empty() {
                    self.modal_err = Some("Enter a server name.".into());
                } else {
                    let sid = Uuid::new_v4();
                    let ch = Channel {
                        id: Uuid::new_v4(),
                        name: String::from("general"),
                        topic: Some(String::from("General discussion")),
                        channel_type: ChannelType::Public,
                        messages: Vec::new(),
                        position: 0,
                    };
                    let first_ch = ch.id;
                    self.servers.push(Server {
                        id: sid,
                        name,
                        description: if self.modal_f2.is_empty() { None } else { Some(self.modal_f2.clone()) },
                        server_code: generate_server_code(),
                        owner_id: self.my_user_id.clone(),
                        channels: vec![ch],
                        members: vec![ServerMember {
                            user_id: self.my_user_id.clone(),
                            display_name: self.display_name().to_string(),
                            role: ServerRole::Owner,
                            muted: false,
                            banned: false,
                            joined_at: now_unix(),
                        }],
                        banned_ids: Vec::new(),
                        created_at: now_unix(),
                        is_owned: true,
                    });
                    self.modal = Modal::None;
                    self.selected_pane = SelectedPane::Server(sid);
                    self.active_view = ActiveView::Channel(sid, first_ch);
                    self.push_notif(Notification::success("Server created!"));
                }
            }
            UiMessage::SubmitJoinServer => {
                let code = self.modal_f1.trim().to_uppercase();
                if code.len() != 8 {
                    self.modal_err = Some("Server code is 8 characters.".into());
                } else {
                    let my_id = self.my_user_id.clone();
                    let my_name = self.display_name().to_string();
                    let found = self.servers.iter().position(|s| s.server_code == code);
                    if let Some(idx) = found {
                        let banned = self.servers[idx].banned_ids.contains(&my_id);
                        if banned {
                            self.modal_err = Some("You are banned from this server.".into());
                        } else {
                            if !self.servers[idx].members.iter().any(|m| m.user_id == my_id) {
                                self.servers[idx].members.push(ServerMember {
                                    user_id: my_id,
                                    display_name: my_name,
                                    role: ServerRole::Member,
                                    muted: false,
                                    banned: false,
                                    joined_at: now_unix(),
                                });
                            }
                            let sid = self.servers[idx].id;
                            let cid = self.servers[idx].channels.first().map(|c| c.id).unwrap_or_else(Uuid::new_v4);
                            self.modal = Modal::None;
                            self.selected_pane = SelectedPane::Server(sid);
                            self.active_view = ActiveView::Channel(sid, cid);
                            self.push_notif(Notification::success("Joined server!"));
                        }
                    } else {
                        self.modal_err = Some("Server not found. Check the code and try again.".into());
                    }
                }
            }
            UiMessage::CreateChannel(sid) => {
                let my_role = self.my_role_in_server(sid);
                if !my_role.can_manage_channels() {
                    self.modal_err = Some("You don't have permission to create channels.".into());
                } else {
                    let name = self.modal_f1.trim().to_lowercase().replace(' ', "-");
                    if name.is_empty() {
                        self.modal_err = Some("Enter a channel name.".into());
                    } else {
                        let ch_type = match self.modal_select {
                            1 => ChannelType::ReadOnly,
                            2 => ChannelType::StaffOnly,
                            3 => ChannelType::Announcement,
                            _ => ChannelType::Public,
                        };
                        if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                            let pos = s.channels.len() as u32;
                            s.channels.push(Channel {
                                id: Uuid::new_v4(),
                                name,
                                topic: None,
                                channel_type: ch_type,
                                messages: Vec::new(),
                                position: pos,
                            });
                        }
                        self.modal = Modal::None;
                        self.push_notif(Notification::success("Channel created!"));
                    }
                }
            }
            UiMessage::SaveServerEdit(sid) => {
                let my_role = self.my_role_in_server(sid);
                if !my_role.can_manage_channels() {
                    self.modal_err = Some("You don't have permission to edit this server.".into());
                } else {
                    let name = self.modal_f1.trim().to_string();
                    if name.is_empty() {
                        self.modal_err = Some("Server name cannot be empty.".into());
                    } else {
                        if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                            s.name = name;
                            s.description = if self.modal_f2.is_empty() { None } else { Some(self.modal_f2.clone()) };
                        }
                        self.modal = Modal::None;
                        self.push_notif(Notification::success("Server updated!"));
                    }
                }
            }
            UiMessage::SetRole { context_id, user_id, role_label, is_server } => {
                if is_server {
                    let my_role = self.my_role_in_server(context_id);
                    if my_role.can_manage_roles() {
                        if let Some(s) = self.servers.iter_mut().find(|s| s.id == context_id) {
                            if let Some(m) = s.members.iter_mut().find(|m| m.user_id == user_id) {
                                m.role = match role_label.as_str() {
                                    "Co-Owner" => ServerRole::CoOwner,
                                    "Admin" => ServerRole::Admin,
                                    "Moderator" => ServerRole::Moderator,
                                    _ => ServerRole::Member,
                                };
                            }
                        }
                        self.push_notif(Notification::success(format!("Role updated to {}.", role_label)));
                    }
                } else {
                    let my_role = self.my_role_in_group(context_id);
                    if my_role.can_moderate() {
                        if let Some(g) = self.groups.iter_mut().find(|g| g.id == context_id) {
                            if let Some(m) = g.members.iter_mut().find(|m| m.user_id == user_id) {
                                m.role = match role_label.as_str() {
                                    "Admin" => GroupRole::Admin,
                                    "Moderator" => GroupRole::Moderator,
                                    _ => GroupRole::Member,
                                };
                            }
                        }
                        self.push_notif(Notification::success(format!("Role updated to {}.", role_label)));
                    }
                }
            }
            UiMessage::KickUser { context_id, user_id, is_server } => {
                if is_server {
                    let my_role = self.my_role_in_server(context_id);
                    if my_role.can_moderate() {
                        if let Some(s) = self.servers.iter_mut().find(|s| s.id == context_id) {
                            s.members.retain(|m| m.user_id != user_id);
                        }
                        self.push_notif(Notification::warn(format!("Kicked {}.", short_id(&user_id))));
                    }
                } else {
                    let my_role = self.my_role_in_group(context_id);
                    if my_role.can_moderate() {
                        if let Some(g) = self.groups.iter_mut().find(|g| g.id == context_id) {
                            g.members.retain(|m| m.user_id != user_id);
                        }
                        self.push_notif(Notification::warn(format!("Kicked {}.", short_id(&user_id))));
                    }
                }
                self.modal = Modal::None;
            }
            UiMessage::BanUser { context_id, user_id, is_server } => {
                if is_server {
                    let my_role = self.my_role_in_server(context_id);
                    if my_role.can_moderate() {
                        if let Some(s) = self.servers.iter_mut().find(|s| s.id == context_id) {
                            s.members.retain(|m| m.user_id != user_id);
                            if !s.banned_ids.contains(&user_id) {
                                s.banned_ids.push(user_id.clone());
                            }
                        }
                        self.push_notif(Notification::error(format!("Banned {}.", short_id(&user_id))));
                    }
                } else {
                    let my_role = self.my_role_in_group(context_id);
                    if my_role.can_moderate() {
                        if let Some(g) = self.groups.iter_mut().find(|g| g.id == context_id) {
                            if let Some(m) = g.members.iter_mut().find(|m| m.user_id == user_id) {
                                m.banned = true;
                            }
                        }
                        self.push_notif(Notification::error(format!("Banned {}.", short_id(&user_id))));
                    }
                }
                self.modal = Modal::None;
            }
            UiMessage::UnbanUser { context_id, user_id } => {
                if let Some(s) = self.servers.iter_mut().find(|s| s.id == context_id) {
                    s.banned_ids.retain(|id| id != &user_id);
                    if let Some(m) = s.members.iter_mut().find(|m| m.user_id == user_id) {
                        m.banned = false;
                    }
                }
                self.push_notif(Notification::info(format!("Unbanned {}.", short_id(&user_id))));
            }
            UiMessage::MuteUser { context_id, user_id, is_server } => {
                if is_server {
                    if let Some(s) = self.servers.iter_mut().find(|s| s.id == context_id) {
                        if let Some(m) = s.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = true;
                        }
                    }
                } else {
                    if let Some(g) = self.groups.iter_mut().find(|g| g.id == context_id) {
                        if let Some(m) = g.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = true;
                        }
                    }
                }
                self.push_notif(Notification::warn(format!("Muted {}.", short_id(&user_id))));
                self.modal = Modal::None;
            }
            UiMessage::UnmuteUser { context_id, user_id, is_server } => {
                if is_server {
                    if let Some(s) = self.servers.iter_mut().find(|s| s.id == context_id) {
                        if let Some(m) = s.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = false;
                        }
                    }
                } else {
                    if let Some(g) = self.groups.iter_mut().find(|g| g.id == context_id) {
                        if let Some(m) = g.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = false;
                        }
                    }
                }
                self.push_notif(Notification::info(format!("Unmuted {}.", short_id(&user_id))));
                self.modal = Modal::None;
            }
            UiMessage::SaveProfile => {
                let name = self.modal_f1.trim().to_string();
                if !name.is_empty() { self.my_name = name; }
                self.my_nick = if self.modal_f2.trim().is_empty() { None } else { Some(self.modal_f2.trim().to_string()) };
                self.my_bio = if self.modal_f3.trim().is_empty() { None } else { Some(self.modal_f3.trim().to_string()) };
                self.modal = Modal::None;
                self.push_notif(Notification::success("Profile saved!"));
            }
            UiMessage::SetMyStatus(s) => { self.my_status = s; }
            UiMessage::IncomingP2P(wire) => { self.handle_incoming(wire); }
            UiMessage::TorStatusChanged(status) => {
                if let P2PStatus::TorReady { ref onion } = status {
                    self.my_user_id = onion.clone();
                }
                let label = status.label().to_string();
                self.tor_status = status;
                self.push_notif(Notification::info(format!("Network: {}", label)));
            }
            UiMessage::SearchChanged(v) => { self.search_query = v; }
            UiMessage::DismissNotif(id) => {
                self.notifications.retain(|n| n.id != id);
            }
            _ => {}
        }
        Command::none()
    }

    // ─── Incoming P2P Handler ─────────────────────────────────────────────────

    fn handle_incoming(&mut self, wire: WireMessage) {
        match &wire.kind.clone() {
            WireKind::DirectMessage => {
                let fid = if let Some(f) = self.friends.iter().find(|f| f.user_id == wire.from_id) {
                    f.id
                } else {
                    let fid = Uuid::new_v4();
                    self.friends.push(Friend {
                        id: fid,
                        display_name: wire.from_name.clone(),
                        user_id: wire.from_id.clone(),
                        note: None,
                        added_at: now_unix(),
                        last_seen: Some(wire.timestamp),
                    });
                    fid
                };
                self.conversations.entry(fid).or_default().push(DirectMessage {
                    id: Uuid::new_v4(),
                    from_id: wire.from_id.clone(),
                    body: wire.body.clone(),
                    timestamp: wire.timestamp,
                    outgoing: false,
                    edited: false,
                    reactions: Vec::new(),
                });
                if self.active_view != ActiveView::DirectMessage(fid) {
                    *self.unread.entry(fid.to_string()).or_insert(0) += 1;
                    self.push_notif(Notification::info(format!("New message from {}", wire.from_name)));
                }
            }
            WireKind::GroupMessage { group_id } => {
                if let Ok(gid) = Uuid::parse_str(group_id) {
                    if let Some(g) = self.groups.iter_mut().find(|x| x.id == gid) {
                        g.messages.push(GroupMessage {
                            id: Uuid::new_v4(),
                            from_id: wire.from_id.clone(),
                            from_name: wire.from_name.clone(),
                            body: wire.body.clone(),
                            timestamp: wire.timestamp,
                            edited: false,
                            reactions: Vec::new(),
                        });
                        if self.active_view != ActiveView::GroupChat(gid) {
                            *self.unread.entry(gid.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            WireKind::ChannelMessage { server_id, channel_id } => {
                if let (Ok(sid), Ok(cid)) = (Uuid::parse_str(server_id), Uuid::parse_str(channel_id)) {
                    if let Some(s) = self.servers.iter_mut().find(|x| x.id == sid) {
                        if let Some(ch) = s.channels.iter_mut().find(|x| x.id == cid) {
                            ch.messages.push(ChannelMessage {
                                id: Uuid::new_v4(),
                                from_id: wire.from_id.clone(),
                                from_name: wire.from_name.clone(),
                                body: wire.body.clone(),
                                timestamp: wire.timestamp,
                                edited: false,
                                reactions: Vec::new(),
                            });
                        }
                        if self.active_view != ActiveView::Channel(sid, cid) {
                            *self.unread.entry(cid.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
            WireKind::FriendRequest => {
                if !self.friends.iter().any(|f| f.user_id == wire.from_id) {
                    self.friends.push(Friend {
                        id: Uuid::new_v4(),
                        display_name: wire.from_name.clone(),
                        user_id: wire.from_id.clone(),
                        note: None,
                        added_at: now_unix(),
                        last_seen: None,
                    });
                    self.push_notif(Notification::info(format!("{} added you as a friend!", wire.from_name)));
                }
            }
            WireKind::GroupInvite { group_id, group_name } => {
                if let Ok(gid) = Uuid::parse_str(group_id) {
                    if !self.groups.iter().any(|g| g.id == gid) {
                        self.groups.push(GroupChat {
                            id: gid,
                            name: group_name.clone(),
                            description: None,
                            owner_id: wire.from_id.clone(),
                            members: vec![
                                GroupMember { user_id: wire.from_id.clone(), display_name: wire.from_name.clone(), role: GroupRole::Owner, muted: false, banned: false, joined_at: now_unix() },
                                GroupMember { user_id: self.my_user_id.clone(), display_name: self.display_name().to_string(), role: GroupRole::Member, muted: false, banned: false, joined_at: now_unix() },
                            ],
                            messages: Vec::new(),
                            created_at: now_unix(),
                        });
                        self.push_notif(Notification::info(format!("Invited to group: {}", group_name)));
                    }
                }
            }
            WireKind::ModerationAction { target_uid, action, context_id } => {
                if target_uid == &self.my_user_id {
                    match action {
                        ModAction::Kick | ModAction::Ban => {
                            if let Ok(sid) = Uuid::parse_str(context_id) {
                                self.servers.retain(|s| s.id != sid);
                                if self.selected_pane == SelectedPane::Server(sid) {
                                    self.selected_pane = SelectedPane::Home;
                                    self.active_view = ActiveView::Friends;
                                }
                                self.push_notif(Notification::error("You were removed from a server."));
                            }
                        }
                        ModAction::Mute => {
                            if let Ok(sid) = Uuid::parse_str(context_id) {
                                if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                                    if let Some(m) = s.members.iter_mut().find(|m| m.user_id == *target_uid) {
                                        m.muted = true;
                                    }
                                }
                            }
                            self.push_notif(Notification::warn("You have been muted."));
                        }
                        ModAction::Unmute => {
                            if let Ok(sid) = Uuid::parse_str(context_id) {
                                if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                                    if let Some(m) = s.members.iter_mut().find(|m| m.user_id == *target_uid) {
                                        m.muted = false;
                                    }
                                }
                            }
                            self.push_notif(Notification::info("You have been unmuted."));
                        }
                        _ => {}
                    }
                }
            }
            WireKind::NicknameUpdate { new_nick } => {
                if let Some(f) = self.friends.iter_mut().find(|f| f.user_id == wire.from_id) {
                    f.display_name = new_nick.clone();
                }
            }
            WireKind::RoleAssignment { target_uid, role_label, context_id } => {
                if target_uid == &self.my_user_id {
                    self.push_notif(Notification::info(format!("Your role was set to {}.", role_label)));
                }
                if let Ok(sid) = Uuid::parse_str(context_id) {
                    if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                        if let Some(m) = s.members.iter_mut().find(|m| m.user_id == *target_uid) {
                            m.role = match role_label.as_str() {
                                "Co-Owner" => ServerRole::CoOwner,
                                "Admin" => ServerRole::Admin,
                                "Moderator" => ServerRole::Moderator,
                                _ => ServerRole::Member,
                            };
                        }
                    }
                }
            }
            WireKind::Ping => {}
        }
    }
    // ─── View ─────────────────────────────────────────────────────────────────

    pub fn view(&self) -> Element<'_, UiMessage> {
        match &self.phase {
            AppPhase::Setup { passphrase, confirm, display_name, error } => {
                self.view_setup(passphrase, confirm, display_name, error.as_deref())
            }
            AppPhase::Unlock { passphrase, error } => {
                self.view_unlock(passphrase, error.as_deref())
            }
            AppPhase::Main => self.view_main(),
        }
    }

    fn view_setup<'a>(&self, pass: &str, confirm: &str, name: &str, error: Option<&str>) -> Element<'a, UiMessage> {
        let header = text("Null Chat").size(32).style(iced::theme::Text::Color(TEXT_WHITE));
        let subtitle = text("Sovereign end-to-end encrypted messenger over Tor").size(14).style(iced::theme::Text::Color(TEXT_MUTED));

        let name_input = text_input("Choose a display name…", name)
            .on_input(UiMessage::SetupNameChanged)
            .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
            .padding(10)
            .size(15);

        let pass_input = text_input("Passphrase (≥12 chars)…", pass)
            .on_input(UiMessage::SetupPassChanged)
            .secure(true)
            .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
            .padding(10)
            .size(15);

        let confirm_input = text_input("Confirm passphrase…", confirm)
            .on_input(UiMessage::SetupConfirmChanged)
            .secure(true)
            .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
            .padding(10)
            .size(15);

        let mut create_btn = button(text("Create Vault").size(15))
            .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
            .padding([10, 20]);

        if !name.is_empty() && !pass.is_empty() && !confirm.is_empty() {
            create_btn = create_btn.on_press(UiMessage::SetupCreate);
        }

        let form = column![
            header,
            subtitle,
            Space::with_height(16),
            text("Display Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            name_input,
            Space::with_height(8),
            text("Vault Passphrase").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            pass_input,
            Space::with_height(8),
            text("Confirm Passphrase").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            confirm_input,
        ]
        .spacing(4)
        .max_width(420);

        let form = if let Some(e) = error {
            form.push(Space::with_height(8))
                .push(text(e).size(13).style(iced::theme::Text::Color(RED)))
        } else {
            form
        };

        let form = form.push(Space::with_height(16)).push(create_btn);

        container(form)
            .style(iced::theme::Container::Custom(Box::new(CardStyle)))
            .padding(40)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_unlock<'a>(&self, pass: &str, error: Option<&str>) -> Element<'a, UiMessage> {
        let header = text("Null Chat").size(32).style(iced::theme::Text::Color(TEXT_WHITE));
        let sub = text("Enter your vault passphrase").size(14).style(iced::theme::Text::Color(TEXT_MUTED));

        let pass_input = text_input("Passphrase…", pass)
            .on_input(UiMessage::UnlockPassChanged)
            .on_submit(UiMessage::UnlockVault)
            .secure(true)
            .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
            .padding(10)
            .size(15);

        let unlock_btn = button(text("Unlock").size(15))
            .on_press(UiMessage::UnlockVault)
            .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
            .padding([10, 20]);

        let mut form = column![header, sub, Space::with_height(16), pass_input].spacing(4).max_width(380);

        if let Some(e) = error {
            form = form.push(Space::with_height(8))
                       .push(text(e).size(13).style(iced::theme::Text::Color(RED)));
        }

        let form = form.push(Space::with_height(16)).push(unlock_btn);

        container(form)
            .style(iced::theme::Container::Custom(Box::new(UnlockCardStyle)))
            .padding(40)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_main(&self) -> Element<'_, UiMessage> {
        let rail = self.view_rail();
        let sidebar = self.view_sidebar();
        let main_area = self.view_main_area();
        let content = row![rail, sidebar, main_area].height(Length::Fill);

        let notifs = self.view_notifications();
        let base = column![content, notifs];

        if self.modal != Modal::None {
            let modal_overlay = self.view_modal();
            // Stack modal overlay on top; iced 0.12 doesn't have a built-in stack widget
            // so we layer by putting modal as a container over the base layout
            let full = column![
                container(
                    container(modal_overlay)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        .style(iced::theme::Container::Custom(Box::new(BgModal)))
                )
                .width(Length::Fill)
                .height(Length::Fill)
            ];
            return full.into();
        }

        base.into()
    }

    // ─── Server Rail ─────────────────────────────────────────────────────────

    fn view_rail(&self) -> Element<'_, UiMessage> {
        let home_active = self.selected_pane == SelectedPane::Home;
        let home_btn = button(
            container(
                text("⌂").size(22).style(iced::theme::Text::Color(if home_active { TEXT_WHITE } else { TEXT_MUTED }))
            ).width(48).height(48).center_x().center_y()
        )
        .on_press(UiMessage::SelectPane(SelectedPane::Home))
        .style(iced::theme::Button::Custom(
            if home_active { Box::new(ActiveServerIconButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
            else { Box::new(ServerIconButton) }
        ))
        .padding(0);

        let mut rail_btns: Vec<Element<UiMessage>> = vec![
            home_btn.into(),
            horizontal_rule(1).into(),
        ];

        for srv in &self.servers {
            let srv_active = self.selected_pane == SelectedPane::Server(srv.id);
            let initials = user_initials(&srv.name);
            let color = user_color_for(&srv.id.to_string());
            let srv_btn = button(
                container(
                    container(text(&initials).size(16).style(iced::theme::Text::Color(TEXT_WHITE)))
                        .style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: if srv_active { 16.0 } else { 24.0 } })))
                        .width(48)
                        .height(48)
                        .center_x()
                        .center_y()
                ).width(48).height(48).center_x().center_y()
            )
            .on_press(UiMessage::SelectPane(SelectedPane::Server(srv.id)))
            .style(iced::theme::Button::Custom(Box::new(ServerIconButton)))
            .padding(0);
            rail_btns.push(srv_btn.into());
        }

        let add_srv_btn = button(
            container(text("+").size(22).style(iced::theme::Text::Color(GREEN))).width(48).height(48).center_x().center_y()
        )
        .on_press(UiMessage::OpenModal(Modal::NewServer))
        .style(iced::theme::Button::Custom(Box::new(ServerIconButton)))
        .padding(0);

        rail_btns.push(Space::with_height(Length::Fill).into());
        rail_btns.push(add_srv_btn.into());

        // Status indicator at bottom
        let status_dot = container(Space::with_width(10)).width(10).height(10);
        rail_btns.push(status_dot.into());

        let rail_col = column(rail_btns).spacing(4).padding([8, 4]);

        container(
            scrollable(rail_col)
                .style(iced::theme::Scrollable::Custom(Box::new(SlimScrollbar)))
        )
        .style(iced::theme::Container::Custom(Box::new(BgDarkest)))
        .width(64)
        .height(Length::Fill)
        .into()
    }

    // ─── Sidebar ──────────────────────────────────────────────────────────────

    fn view_sidebar(&self) -> Element<'_, UiMessage> {
        match &self.selected_pane {
            SelectedPane::Home => self.view_home_sidebar(),
            SelectedPane::Server(sid) => self.view_server_sidebar(*sid),
        }
    }

    fn view_home_sidebar(&self) -> Element<'_, UiMessage> {
        let title = text("Direct Messages").size(12).style(iced::theme::Text::Color(TEXT_MUTED));

        let add_friend = button(text("+ Add Friend").size(13))
            .on_press(UiMessage::OpenModal(Modal::AddFriend))
            .style(iced::theme::Button::Custom(Box::new(FlatButton)))
            .padding([4, 10]);

        let mut items: Vec<Element<UiMessage>> = vec![];

        // DMs
        if !self.friends.is_empty() {
            items.push(text("Friends").size(11).style(iced::theme::Text::Color(TEXT_MUTED)).into());
        }
        for f in &self.friends {
            let unread = self.unread.get(&f.id.to_string()).copied().unwrap_or(0);
            let active = self.active_view == ActiveView::DirectMessage(f.id);
            let name_col = column![
                text(&f.display_name).size(14).style(iced::theme::Text::Color(if active { TEXT_WHITE } else { TEXT_NORMAL })),
                text(format!("…{}", &f.user_id[f.user_id.len().saturating_sub(8)..]))
                    .size(11).style(iced::theme::Text::Color(TEXT_MUTED)),
            ].spacing(1);

            let mut row_items: Vec<Element<UiMessage>> = vec![
                Space::with_width(8).into(),
                name_col.into(),
                Space::with_width(Length::Fill).into(),
            ];

            if unread > 0 {
                row_items.push(
                    container(text(format!("{}", unread)).size(11).style(iced::theme::Text::Color(TEXT_WHITE)))
                        .style(iced::theme::Container::Custom(Box::new(UnreadBadge)))
                        .padding([2, 6])
                        .into()
                );
            }

            let row_content = row(row_items).align_items(Alignment::Center);
            let btn = button(row_content)
                .on_press(UiMessage::SelectView(ActiveView::DirectMessage(f.id)))
                .style(iced::theme::Button::Custom(
                    if active { Box::new(ActiveFlatButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                    else { Box::new(FlatButton) }
                ))
                .width(Length::Fill)
                .padding([6, 4]);
            items.push(btn.into());
        }

        // Groups
        if !self.groups.is_empty() {
            items.push(Space::with_height(8).into());
            items.push(text("Groups").size(11).style(iced::theme::Text::Color(TEXT_MUTED)).into());
        }
        for g in &self.groups {
            let unread = self.unread.get(&g.id.to_string()).copied().unwrap_or(0);
            let active = self.active_view == ActiveView::GroupChat(g.id);
            let mut row_items: Vec<Element<UiMessage>> = vec![
                Space::with_width(8).into(),
                text(&g.name).size(14).style(iced::theme::Text::Color(if active { TEXT_WHITE } else { TEXT_NORMAL })).into(),
                Space::with_width(Length::Fill).into(),
            ];
            if unread > 0 {
                row_items.push(
                    container(text(format!("{}", unread)).size(11).style(iced::theme::Text::Color(TEXT_WHITE)))
                        .style(iced::theme::Container::Custom(Box::new(UnreadBadge)))
                        .padding([2, 6]).into()
                );
            }
            let row_content = row(row_items).align_items(Alignment::Center);

            let manage_btn: Element<UiMessage> = if matches!(self.my_role_in_group(g.id), GroupRole::Owner | GroupRole::Admin) {
                button(text("⚙").size(12))
                    .on_press(UiMessage::OpenModal(Modal::ManageMembers(g.id)))
                    .style(iced::theme::Button::Custom(Box::new(FlatButton)))
                    .padding([2, 6])
                    .into()
            } else {
                Space::with_width(0).into()
            };

            let full_row = row![
                button(row_content)
                    .on_press(UiMessage::SelectView(ActiveView::GroupChat(g.id)))
                    .style(iced::theme::Button::Custom(
                        if active { Box::new(ActiveFlatButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                        else { Box::new(FlatButton) }
                    ))
                    .width(Length::Fill)
                    .padding([6, 4]),
                manage_btn,
            ].align_items(Alignment::Center);
            items.push(full_row.into());
        }

        let create_group = button(text("+ New Group").size(13))
            .on_press(UiMessage::OpenModal(Modal::NewGroup))
            .style(iced::theme::Button::Custom(Box::new(FlatButton)))
            .padding([4, 10]);

        let header = row![title, Space::with_width(Length::Fill), add_friend].align_items(Alignment::Center).padding([8, 8, 4, 8]);

        let list = scrollable(column(items).spacing(2).padding([0, 4]))
            .style(iced::theme::Scrollable::Custom(Box::new(SlimScrollbar)));

        let me_area = self.view_me_area();

        container(
            column![
                header,
                list,
                Space::with_height(Length::Fill),
                horizontal_rule(1),
                Space::with_height(4),
                container(create_group).padding([0, 4]),
                Space::with_height(4),
                me_area,
            ]
        )
        .style(iced::theme::Container::Custom(Box::new(BgDark)))
        .width(240)
        .height(Length::Fill)
        .into()
    }

    fn view_server_sidebar(&self, sid: Uuid) -> Element<'_, UiMessage> {
        let server = match self.servers.iter().find(|s| s.id == sid) {
            Some(s) => s,
            None => return Space::with_width(240).into(),
        };
        let my_role = self.my_role_in_server(sid);

        let server_name = button(text(&server.name).size(15).style(iced::theme::Text::Color(TEXT_WHITE)))
            .on_press(UiMessage::OpenModal(Modal::ServerInfo(sid)))
            .style(iced::theme::Button::Custom(Box::new(FlatButton)))
            .width(Length::Fill)
            .padding([10, 12]);

        let mut channel_items: Vec<Element<UiMessage>> = vec![
            text("Channels").size(11).style(iced::theme::Text::Color(TEXT_MUTED)).into(),
        ];

        for ch in &server.channels {
            if !server.can_view_channel(ch, &my_role) {
                continue;
            }
            let active = self.active_view == ActiveView::Channel(sid, ch.id);
            let unread = self.unread.get(&ch.id.to_string()).copied().unwrap_or(0);
            let icon = ch.channel_type.icon();
            let name_txt = text(format!("{} {}", icon, ch.name)).size(14)
                .style(iced::theme::Text::Color(if active { TEXT_WHITE } else { TEXT_MUTED }));

            let mut row_ch: Vec<Element<UiMessage>> = vec![
                Space::with_width(8).into(),
                name_txt.into(),
                Space::with_width(Length::Fill).into(),
            ];
            if unread > 0 {
                row_ch.push(
                    container(text(format!("{}", unread)).size(11).style(iced::theme::Text::Color(TEXT_WHITE)))
                        .style(iced::theme::Container::Custom(Box::new(UnreadBadge)))
                        .padding([2, 5]).into()
                );
            }

            let btn_style: Box<dyn button::StyleSheet<Style = iced::Theme>> = match ch.channel_type {
                ChannelType::StaffOnly => {
                    if active { Box::new(ActiveFlatButton) } else { Box::new(FlatButton) }
                }
                _ => {
                    if active { Box::new(ActiveFlatButton) } else { Box::new(FlatButton) }
                }
            };

            let ch_btn = button(row(row_ch).align_items(Alignment::Center))
                .on_press(UiMessage::SelectView(ActiveView::Channel(sid, ch.id)))
                .style(iced::theme::Button::Custom(btn_style))
                .width(Length::Fill)
                .padding([5, 4]);

            channel_items.push(ch_btn.into());
        }

        let channel_list = scrollable(
            column(channel_items).spacing(1).padding([0, 4])
        ).style(iced::theme::Scrollable::Custom(Box::new(SlimScrollbar)));

        let mut action_row: Vec<Element<UiMessage>> = vec![Space::with_width(Length::Fill).into()];

        if my_role.can_manage_channels() {
            action_row.push(
                button(text("+ Ch").size(12))
                    .on_press(UiMessage::OpenModal(Modal::CreateChannel(sid)))
                    .style(iced::theme::Button::Custom(Box::new(FlatButton)))
                    .padding([3, 8])
                    .into()
            );
        }
        if my_role.can_moderate() {
            action_row.push(
                button(text("👥").size(12))
                    .on_press(UiMessage::OpenModal(Modal::ManageMembers(sid)))
                    .style(iced::theme::Button::Custom(Box::new(FlatButton)))
                    .padding([3, 6])
                    .into()
            );
        }
        if my_role.can_manage_channels() {
            action_row.push(
                button(text("⚙").size(12))
                    .on_press(UiMessage::OpenModal(Modal::EditServer(sid)))
                    .style(iced::theme::Button::Custom(Box::new(FlatButton)))
                    .padding([3, 6])
                    .into()
            );
        }

        let actions = row(action_row).align_items(Alignment::Center).padding([0, 4]);

        let invite_code = container(
            column![
                text("Server Code").size(11).style(iced::theme::Text::Color(TEXT_MUTED)),
                text(&server.server_code).size(13)
                    .style(iced::theme::Text::Color(BLURPLE)),
            ].spacing(2)
        ).padding([4, 8]);

        let me_area = self.view_me_area();

        container(
            column![
                server_name,
                horizontal_rule(1),
                actions,
                Space::with_height(4),
                channel_list,
                Space::with_height(Length::Fill),
                horizontal_rule(1),
                invite_code,
                horizontal_rule(1),
                me_area,
            ]
        )
        .style(iced::theme::Container::Custom(Box::new(BgDark)))
        .width(240)
        .height(Length::Fill)
        .into()
    }

    fn view_me_area(&self) -> Element<'_, UiMessage> {
        let color = user_color_for(&self.my_user_id);
        let initials = user_initials(self.display_name());
        let avatar = container(
            container(text(&initials).size(12).style(iced::theme::Text::Color(TEXT_WHITE)))
                .style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 16.0 })))
                .width(32)
                .height(32)
                .center_x()
                .center_y()
        );

        let status_label = text(self.my_status.label()).size(11).style(iced::theme::Text::Color(TEXT_MUTED));
        let name_label = text(self.display_name()).size(13).style(iced::theme::Text::Color(TEXT_WHITE));

        let edit_btn = button(text("✏").size(14))
            .on_press(UiMessage::OpenModal(Modal::EditProfile))
            .style(iced::theme::Button::Custom(Box::new(FlatButton)))
            .padding([2, 4]);

        row![
            avatar,
            Space::with_width(8),
            column![name_label, status_label].spacing(1),
            Space::with_width(Length::Fill),
            edit_btn,
        ]
        .align_items(Alignment::Center)
        .padding([8, 8])
        .into()
    }

    // ─── Main Area ────────────────────────────────────────────────────────────

    fn view_main_area(&self) -> Element<'_, UiMessage> {
        match &self.active_view {
            ActiveView::Friends => self.view_friends_home(),
            ActiveView::DirectMessage(fid) => self.view_dm(*fid),
            ActiveView::GroupChat(gid) => self.view_group(*gid),
            ActiveView::Channel(sid, cid) => self.view_channel(*sid, *cid),
        }
    }

    fn view_friends_home(&self) -> Element<'_, UiMessage> {
        let header = row![
            text("Friends").size(20).style(iced::theme::Text::Color(TEXT_WHITE)),
            Space::with_width(Length::Fill),
            button(text("Add Friend").size(14))
                .on_press(UiMessage::OpenModal(Modal::AddFriend))
                .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                .padding([6, 14]),
        ]
        .align_items(Alignment::Center)
        .padding([12, 16]);

        if self.friends.is_empty() {
            let empty = container(
                column![
                    text("No friends yet").size(22).style(iced::theme::Text::Color(TEXT_MUTED)),
                    Space::with_height(8),
                    text("Add a friend to start messaging").size(14).style(iced::theme::Text::Color(TEXT_MUTED)),
                ]
                .align_items(Alignment::Center)
                .spacing(4)
            )
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill);

            return container(column![header, horizontal_rule(1), empty])
                .style(iced::theme::Container::Custom(Box::new(BgMain)))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        let mut cards: Vec<Element<UiMessage>> = Vec::new();
        for f in &self.friends {
            let color = user_color_for(&f.user_id);
            let initials = user_initials(&f.display_name);
            let av = container(
                container(text(&initials).size(14).style(iced::theme::Text::Color(TEXT_WHITE)))
                    .style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 20.0 })))
                    .width(40).height(40).center_x().center_y()
            );

            let msg_btn = button(text("Message").size(12))
                .on_press(UiMessage::SelectView(ActiveView::DirectMessage(f.id)))
                .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                .padding([4, 12]);

            let card = container(
                row![
                    av,
                    Space::with_width(10),
                    column![
                        text(&f.display_name).size(15).style(iced::theme::Text::Color(TEXT_WHITE)),
                        text(format!("ID: {}", short_id(&f.user_id))).size(11).style(iced::theme::Text::Color(TEXT_MUTED)),
                    ].spacing(2),
                    Space::with_width(Length::Fill),
                    msg_btn,
                ]
                .align_items(Alignment::Center)
                .padding([12, 16])
            )
            .style(iced::theme::Container::Custom(Box::new(MemberCardStyle)))
            .width(Length::Fill);

            cards.push(card.into());
        }

        let list = scrollable(column(cards).spacing(4).padding(16))
            .style(iced::theme::Scrollable::Custom(Box::new(SlimScrollbar)));

        container(column![header, horizontal_rule(1), list])
            .style(iced::theme::Container::Custom(Box::new(BgMain)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_dm(&self, fid: Uuid) -> Element<'_, UiMessage> {
        let friend = match self.friends.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => return Space::with_width(Length::Fill).into(),
        };

        let messages = self.conversations.get(&fid).map(|v| v.as_slice()).unwrap_or(&[]);
        self.view_chat_area(&friend.display_name, messages.iter().map(|m| ChatMsg {
            from_id: m.from_id.clone(),
            from_name: if m.outgoing { self.display_name().to_string() } else { friend.display_name.clone() },
            body: m.body.clone(),
            timestamp: m.timestamp,
            outgoing: m.outgoing,
            edited: m.edited,
        }).collect(), None, false)
    }

    fn view_group(&self, gid: Uuid) -> Element<'_, UiMessage> {
        let group = match self.groups.iter().find(|g| g.id == gid) {
            Some(g) => g,
            None => return Space::with_width(Length::Fill).into(),
        };

        let am_muted = self.am_muted_in_group(gid);
        let msgs: Vec<ChatMsg> = group.messages.iter().map(|m| ChatMsg {
            from_id: m.from_id.clone(),
            from_name: m.from_name.clone(),
            body: m.body.clone(),
            timestamp: m.timestamp,
            outgoing: m.from_id == self.my_user_id,
            edited: m.edited,
        }).collect();

        let manage_btn: Option<Element<UiMessage>> = if matches!(self.my_role_in_group(gid), GroupRole::Owner | GroupRole::Admin | GroupRole::Moderator) {
            Some(button(text("Manage Members").size(13))
                .on_press(UiMessage::OpenModal(Modal::ManageMembers(gid)))
                .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                .padding([4, 12])
                .into())
        } else { None };

        self.view_chat_area_ex(&group.name, msgs, manage_btn, am_muted, Some(gid))
    }

    fn view_channel(&self, sid: Uuid, cid: Uuid) -> Element<'_, UiMessage> {
        let server = match self.servers.iter().find(|s| s.id == sid) {
            Some(s) => s,
            None => return Space::with_width(Length::Fill).into(),
        };
        let channel = match server.channels.iter().find(|c| c.id == cid) {
            Some(c) => c,
            None => return Space::with_width(Length::Fill).into(),
        };
        let my_role = self.my_role_in_server(sid);
        let can_send = server.can_send_in(channel, &my_role) && !self.am_muted_in_server(sid);

        let mut header_extras: Vec<Element<UiMessage>> = Vec::new();
        if let Some(topic) = &channel.topic {
            header_extras.push(
                text(format!("│ {}", topic)).size(13).style(iced::theme::Text::Color(TEXT_MUTED)).into()
            );
        }
        let ch_type_badge = container(
            text(channel.channel_type.label()).size(11).style(iced::theme::Text::Color(TEXT_MUTED))
        )
        .style(iced::theme::Container::Custom(Box::new(InlineTagStyle { color: BG_DARKEST })))
        .padding([2, 6]);
        header_extras.push(ch_type_badge.into());

        let msgs: Vec<ChatMsg> = channel.messages.iter().map(|m| ChatMsg {
            from_id: m.from_id.clone(),
            from_name: m.from_name.clone(),
            body: m.body.clone(),
            timestamp: m.timestamp,
            outgoing: m.from_id == self.my_user_id,
            edited: m.edited,
        }).collect();

        let head_extra: Option<Element<UiMessage>> = if header_extras.is_empty() { None } else {
            Some(row(header_extras).spacing(6).align_items(Alignment::Center).into())
        };

        self.view_chat_area_with_header(&format!("{} {}", channel.channel_type.icon(), channel.name), msgs, head_extra, !can_send, None)
    }

    fn view_chat_area<'a>(&'a self, title: &str, msgs: Vec<ChatMsg>, extra_header: Option<Element<'a, UiMessage>>, read_only: bool) -> Element<'a, UiMessage> {
        self.view_chat_area_with_header(title, msgs, extra_header, read_only, None)
    }

    fn view_chat_area_ex<'a>(&'a self, title: &str, msgs: Vec<ChatMsg>, extra_header: Option<Element<'a, UiMessage>>, read_only: bool, _gid: Option<Uuid>) -> Element<'a, UiMessage> {
        self.view_chat_area_with_header(title, msgs, extra_header, read_only, None)
    }

    fn view_chat_area_with_header<'a>(&'a self, title: &str, msgs: Vec<ChatMsg>, extra_header: Option<Element<'a, UiMessage>>, read_only: bool, _ctx: Option<Uuid>) -> Element<'a, UiMessage> {
        let title_txt = text(title).size(18).style(iced::theme::Text::Color(TEXT_WHITE));

        let mut header_row_items: Vec<Element<UiMessage>> = vec![
            Space::with_width(16).into(),
            title_txt.into(),
            Space::with_width(8).into(),
        ];
        if let Some(extra) = extra_header {
            header_row_items.push(extra);
        }
        header_row_items.push(Space::with_width(Length::Fill).into());

        let header = container(
            row(header_row_items).align_items(Alignment::Center)
        )
        .style(iced::theme::Container::Custom(Box::new(MessageHeaderStyle)))
        .width(Length::Fill)
        .padding([0, 0, 0, 0])
        .height(48);

        let msg_items: Vec<Element<UiMessage>> = if msgs.is_empty() {
            vec![
                container(
                    text("No messages yet. Say hello!").size(14).style(iced::theme::Text::Color(TEXT_MUTED))
                ).padding(20).into()
            ]
        } else {
            msgs.iter().map(|m| self.view_message_row(m)).collect()
        };

        let msg_list = scrollable(
            column(msg_items).spacing(1).padding([8, 16])
        )
        .style(iced::theme::Scrollable::Custom(Box::new(SlimScrollbar)))
        .height(Length::Fill);

        let compose = if read_only {
            container(
                text("You cannot send messages here.").size(13).style(iced::theme::Text::Color(TEXT_MUTED))
            )
            .style(iced::theme::Container::Custom(Box::new(ComposeBarStyle)))
            .width(Length::Fill)
            .padding([14, 16])
        } else {
            let input = text_input("Message…", &self.compose_text)
                .on_input(UiMessage::ComposeChanged)
                .on_submit(UiMessage::SendMessage)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(12)
                .size(15)
                .width(Length::Fill);

            let send_btn = button(text("Send").size(14))
                .on_press(UiMessage::SendMessage)
                .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                .padding([6, 14]);

            container(
                row![input, Space::with_width(8), send_btn]
                    .align_items(Alignment::Center)
            )
            .style(iced::theme::Container::Custom(Box::new(ComposeBarStyle)))
            .width(Length::Fill)
            .padding([8, 12])
        };

        container(column![header, horizontal_rule(1), msg_list, compose])
            .style(iced::theme::Container::Custom(Box::new(BgMain)))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_message_row(&self, m: &ChatMsg) -> Element<'_, UiMessage> {
        let color = user_color_for(&m.from_id);
        let initials = user_initials(&m.from_name);

        let avatar = container(
            container(text(&initials).size(11).style(iced::theme::Text::Color(TEXT_WHITE)))
                .style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 14.0 })))
                .width(28).height(28).center_x().center_y()
        );

        let name_color = if m.outgoing { BLURPLE } else { color };
        let name_txt = text(&m.from_name).size(13)
            .style(iced::theme::Text::Color(name_color));
        let time_txt = text(format!(" — {}", format_ts(m.timestamp))).size(11)
            .style(iced::theme::Text::Color(TEXT_MUTED));

        let edited_txt: Option<Element<UiMessage>> = if m.edited {
            Some(text(" (edited)").size(11).style(iced::theme::Text::Color(TEXT_MUTED)).into())
        } else { None };

        let body = text(&m.body).size(15).style(iced::theme::Text::Color(TEXT_NORMAL));

        let mut body_row: Vec<Element<UiMessage>> = vec![body.into()];
        if let Some(e) = edited_txt { body_row.push(e); }

        let msg_col = column![
            row![name_txt, time_txt].align_items(Alignment::Center),
            row(body_row).align_items(Alignment::Center),
        ].spacing(1);

        row![
            avatar,
            Space::with_width(8),
            msg_col,
        ]
        .align_items(Alignment::Start)
        .padding([4, 0])
        .into()
    }

    // ─── Notifications ────────────────────────────────────────────────────────

    fn view_notifications(&self) -> Element<'_, UiMessage> {
        if self.notifications.is_empty() {
            return Space::with_height(0).into();
        }

        let items: Vec<Element<UiMessage>> = self.notifications.iter().rev().take(3).map(|n| {
            let style: Box<dyn container::StyleSheet<Style=iced::Theme>> = match n.kind {
                NotifKind::Info => Box::new(NotifInfo),
                NotifKind::Success => Box::new(NotifSuccess),
                NotifKind::Error => Box::new(NotifError),
                NotifKind::Warning => Box::new(NotifWarn),
            };
            let dismiss_btn = button(text("×").size(14))
                .on_press(UiMessage::DismissNotif(n.id))
                .style(iced::theme::Button::Custom(Box::new(FlatButton)))
                .padding([0, 4]);
            container(
                row![
                    text(&n.message).size(13).style(iced::theme::Text::Color(TEXT_WHITE)),
                    Space::with_width(Length::Fill),
                    dismiss_btn,
                ].align_items(Alignment::Center).padding([4, 10])
            )
            .style(iced::theme::Container::Custom(style))
            .width(Length::Fill)
            .into()
        }).collect();

        container(column(items).spacing(2))
            .width(Length::Fill)
            .padding([0, 8, 4, 8])
            .into()
    }

    // ─── Modals ───────────────────────────────────────────────────────────────

    fn view_modal(&self) -> Element<'_, UiMessage> {
        let content: Element<UiMessage> = match &self.modal {
            Modal::None => return Space::with_height(0).into(),
            Modal::AddFriend => self.view_modal_add_friend(),
            Modal::NewGroup => self.view_modal_new_group(),
            Modal::NewServer => self.view_modal_new_server(),
            Modal::JoinServer => self.view_modal_join_server(),
            Modal::Profile | Modal::EditProfile => self.view_modal_edit_profile(),
            Modal::MigrateDevice => self.view_modal_migrate(),
            Modal::ServerInfo(sid) => self.view_modal_server_info(*sid),
            Modal::EditServer(sid) => self.view_modal_edit_server(*sid),
            Modal::CreateChannel(sid) => self.view_modal_create_channel(*sid),
            Modal::ManageMembers(ctx_id) => self.view_modal_manage_members(*ctx_id),
            Modal::GroupAddMember(gid) => self.view_modal_add_group_member(*gid),
            Modal::MemberDetail { context_id, user_id, is_server } => {
                self.view_modal_member_detail(*context_id, user_id, *is_server)
            }
        };

        container(
            column![
                row![
                    Space::with_width(Length::Fill),
                    button(text("✕").size(16))
                        .on_press(UiMessage::CloseModal)
                        .style(iced::theme::Button::Custom(Box::new(FlatButton)))
                        .padding([4, 8]),
                ],
                content,
            ]
            .spacing(4)
        )
        .style(iced::theme::Container::Custom(Box::new(CardStyle)))
        .max_width(480)
        .padding(24)
        .into()
    }

    fn modal_header<'a>(title: &'a str) -> Element<'a, UiMessage> {
        text(title).size(20).style(iced::theme::Text::Color(TEXT_WHITE)).into()
    }

    fn view_modal_add_friend(&self) -> Element<'_, UiMessage> {
        column![
            Self::modal_header("Add Friend"),
            Space::with_height(12),
            text("Their User ID (onion address)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("xxxx...onion", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Display Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("A name for them…", &self.modal_f2)
                .on_input(UiMessage::Field2Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Add Friend").size(14))
                    .on_press(UiMessage::SubmitAddFriend)
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_new_group(&self) -> Element<'_, UiMessage> {
        column![
            Self::modal_header("Create Group"),
            Space::with_height(12),
            text("Group Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("My Group…", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Description (optional)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("What's this group about?", &self.modal_f2)
                .on_input(UiMessage::Field2Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Create").size(14))
                    .on_press(UiMessage::SubmitNewGroup)
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_new_server(&self) -> Element<'_, UiMessage> {
        column![
            Self::modal_header("Create Server"),
            Space::with_height(12),
            text("Server Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("My Server…", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Description (optional)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Server description…", &self.modal_f2)
                .on_input(UiMessage::Field2Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Create Server").size(14))
                    .on_press(UiMessage::SubmitNewServer)
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_join_server(&self) -> Element<'_, UiMessage> {
        column![
            Self::modal_header("Join Server"),
            Space::with_height(12),
            text("Server Code (8 characters)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("ABCD1234…", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Join").size(14))
                    .on_press(UiMessage::SubmitJoinServer)
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_edit_profile(&self) -> Element<'_, UiMessage> {
        let user_id_row = row![
            text("Your ID:").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            Space::with_width(8),
            text(short_id(&self.my_user_id)).size(12).style(iced::theme::Text::Color(BLURPLE)),
        ].align_items(Alignment::Center);

        column![
            Self::modal_header("Edit Profile"),
            Space::with_height(8),
            user_id_row,
            Space::with_height(12),
            text("Display Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Your name…", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Nickname (optional override)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Short nickname…", &self.modal_f2)
                .on_input(UiMessage::Field2Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Bio (optional)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Tell others about yourself…", &self.modal_f3)
                .on_input(UiMessage::Field3Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Status").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            row![
                button(text("Online").size(12))
                    .on_press(UiMessage::SetMyStatus(UserStatus::Online))
                    .style(iced::theme::Button::Custom(
                        if self.my_status == UserStatus::Online { Box::new(BlurpleButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                        else { Box::new(GhostButton) }
                    ))
                    .padding([4, 10]),
                Space::with_width(4),
                button(text("Away").size(12))
                    .on_press(UiMessage::SetMyStatus(UserStatus::Away))
                    .style(iced::theme::Button::Custom(
                        if self.my_status == UserStatus::Away { Box::new(BlurpleButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                        else { Box::new(GhostButton) }
                    ))
                    .padding([4, 10]),
                Space::with_width(4),
                button(text("DND").size(12))
                    .on_press(UiMessage::SetMyStatus(UserStatus::DoNotDisturb))
                    .style(iced::theme::Button::Custom(
                        if self.my_status == UserStatus::DoNotDisturb { Box::new(BlurpleButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                        else { Box::new(GhostButton) }
                    ))
                    .padding([4, 10]),
            ].spacing(2),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Save").size(14))
                    .on_press(UiMessage::SaveProfile)
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_migrate(&self) -> Element<'_, UiMessage> {
        column![
            Self::modal_header("Migrate Device"),
            Space::with_height(12),
            text("Export your vault to migrate to a new device.").size(14).style(iced::theme::Text::Color(TEXT_NORMAL)),
            Space::with_height(8),
            text("1. Export the vault file from this device.").size(13).style(iced::theme::Text::Color(TEXT_MUTED)),
            text("2. Copy to the new device.").size(13).style(iced::theme::Text::Color(TEXT_MUTED)),
            text("3. Open the vault with your passphrase.").size(13).style(iced::theme::Text::Color(TEXT_MUTED)),
            Space::with_height(12),
            row![
                button(text("Close").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
            ],
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_server_info(&self, sid: Uuid) -> Element<'_, UiMessage> {
        let server = match self.servers.iter().find(|s| s.id == sid) {
            Some(s) => s,
            None => return text("Server not found.").into(),
        };
        let my_role = self.my_role_in_server(sid);

        let mut col = column![
            Self::modal_header(&server.name),
            Space::with_height(8),
            text(format!("Members: {}", server.members.len())).size(14).style(iced::theme::Text::Color(TEXT_MUTED)),
            text(format!("Channels: {}", server.channels.len())).size(14).style(iced::theme::Text::Color(TEXT_MUTED)),
            text(format!("Code: {}", server.server_code)).size(14).style(iced::theme::Text::Color(BLURPLE)),
            text(format!("Your role: {}", my_role.label())).size(14).style(iced::theme::Text::Color(my_role.badge_color())),
        ]
        .spacing(4);

        if let Some(desc) = &server.description {
            col = col.push(text(desc.as_str()).size(13).style(iced::theme::Text::Color(TEXT_MUTED)));
        }

        col.push(Space::with_height(12))
           .push(button(text("Close").size(14))
               .on_press(UiMessage::CloseModal)
               .style(iced::theme::Button::Custom(Box::new(GhostButton)))
               .padding([8, 16]))
           .into()
    }

    fn view_modal_edit_server(&self, sid: Uuid) -> Element<'_, UiMessage> {
        column![
            Self::modal_header("Edit Server"),
            Space::with_height(12),
            text("Server Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Server name…", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Description").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Server description…", &self.modal_f2)
                .on_input(UiMessage::Field2Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Save").size(14))
                    .on_press(UiMessage::SaveServerEdit(sid))
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_create_channel(&self, sid: Uuid) -> Element<'_, UiMessage> {
        let ch_type_options = ["Public", "Read-Only", "Staff Only", "Announcement"];
        let type_buttons: Vec<Element<UiMessage>> = ch_type_options.iter().enumerate().map(|(i, label)| {
            let active = self.modal_select == i;
            button(text(*label).size(12))
                .on_press(UiMessage::SelectChanged(i))
                .style(iced::theme::Button::Custom(
                    if active { Box::new(BlurpleButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                    else { Box::new(GhostButton) }
                ))
                .padding([4, 10])
                .into()
        }).collect();

        column![
            Self::modal_header("Create Channel"),
            Space::with_height(12),
            text("Channel Name").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("channel-name…", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Channel Type").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            row(type_buttons).spacing(4),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Create Channel").size(14))
                    .on_press(UiMessage::CreateChannel(sid))
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_manage_members(&self, ctx_id: Uuid) -> Element<'_, UiMessage> {
        // Could be a server or a group
        let is_server = self.servers.iter().any(|s| s.id == ctx_id);
        let my_can_moderate = if is_server {
            self.my_role_in_server(ctx_id).can_moderate()
        } else {
            self.my_role_in_group(ctx_id).can_moderate()
        };

        let members_list: Vec<Element<UiMessage>> = if is_server {
            let server = self.servers.iter().find(|s| s.id == ctx_id);
            server.map(|s| {
                s.members.iter().map(|m| {
                    self.view_member_row(ctx_id, &m.user_id, &m.display_name, &m.role.label().to_string(), m.muted, true, my_can_moderate)
                }).collect()
            }).unwrap_or_default()
        } else {
            let group = self.groups.iter().find(|g| g.id == ctx_id);
            group.map(|g| {
                g.members.iter().map(|m| {
                    self.view_member_row(ctx_id, &m.user_id, &m.display_name, &m.role.label().to_string(), m.muted, false, my_can_moderate)
                }).collect()
            }).unwrap_or_default()
        };

        let mut col = column![Self::modal_header("Manage Members"), Space::with_height(8)];

        // Show banned list for servers
        if is_server {
            if let Some(srv) = self.servers.iter().find(|s| s.id == ctx_id) {
                if !srv.banned_ids.is_empty() {
                    col = col.push(text(format!("Banned ({}):", srv.banned_ids.len())).size(12).style(iced::theme::Text::Color(RED)));
                    for uid in &srv.banned_ids {
                        let uid_clone = uid.clone();
                        let ctx_clone = ctx_id;
                        col = col.push(
                            row![
                                text(short_id(uid)).size(13).style(iced::theme::Text::Color(TEXT_MUTED)),
                                Space::with_width(Length::Fill),
                                button(text("Unban").size(12))
                                    .on_press(UiMessage::UnbanUser { context_id: ctx_clone, user_id: uid_clone.clone() })
                                    .style(iced::theme::Button::Custom(Box::new(SuccessButton)))
                                    .padding([2, 8]),
                            ]
                            .align_items(Alignment::Center)
                            .padding([2, 0])
                        );
                    }
                    col = col.push(Space::with_height(8));
                }
            }
        }

        if !is_server {
            let add_btn = button(text("+ Add Member").size(13))
                .on_press(UiMessage::OpenModal(Modal::GroupAddMember(ctx_id)))
                .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                .padding([5, 12]);
            col = col.push(add_btn).push(Space::with_height(8));
        }

        for item in members_list { col = col.push(item); }

        col.push(Space::with_height(12))
           .push(button(text("Close").size(14))
               .on_press(UiMessage::CloseModal)
               .style(iced::theme::Button::Custom(Box::new(GhostButton)))
               .padding([8, 16]))
           .spacing(2)
           .into()
    }

    fn view_member_row(&self, ctx_id: Uuid, uid: &str, name: &str, role: &str, muted: bool, is_server: bool, can_moderate: bool) -> Element<'_, UiMessage> {
        let uid = uid.to_string();
        let color = user_color_for(&uid);
        let initials = user_initials(name);

        let av = container(
            container(text(&initials).size(11).style(iced::theme::Text::Color(TEXT_WHITE)))
                .style(iced::theme::Container::Custom(Box::new(AvatarContainer { color, radius: 12.0 })))
                .width(24).height(24).center_x().center_y()
        );

        let mut row_items: Vec<Element<UiMessage>> = vec![
            av.into(),
            Space::with_width(8).into(),
            column![
                text(name).size(13).style(iced::theme::Text::Color(TEXT_NORMAL)),
                text(role).size(11).style(iced::theme::Text::Color(TEXT_MUTED)),
            ].spacing(0).into(),
            Space::with_width(Length::Fill).into(),
        ];

        if muted {
            row_items.push(text("🔇").size(12).into());
            row_items.push(Space::with_width(4).into());
        }

        if can_moderate && uid != self.my_user_id {
            if muted {
                let uid2 = uid.clone();
                row_items.push(
                    button(text("Unmute").size(11))
                        .on_press(UiMessage::UnmuteUser { context_id: ctx_id, user_id: uid2, is_server })
                        .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                        .padding([2, 6]).into()
                );
            } else {
                let uid2 = uid.clone();
                row_items.push(
                    button(text("Mute").size(11))
                        .on_press(UiMessage::MuteUser { context_id: ctx_id, user_id: uid2, is_server })
                        .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                        .padding([2, 6]).into()
                );
            }
            row_items.push(Space::with_width(2).into());
            let uid2 = uid.clone();
            row_items.push(
                button(text("Kick").size(11))
                    .on_press(UiMessage::KickUser { context_id: ctx_id, user_id: uid2, is_server })
                    .style(iced::theme::Button::Custom(Box::new(DestructiveFlatButton)))
                    .padding([2, 6]).into()
            );
            if is_server {
                row_items.push(Space::with_width(2).into());
                let uid2 = uid.clone();
                row_items.push(
                    button(text("Ban").size(11))
                        .on_press(UiMessage::BanUser { context_id: ctx_id, user_id: uid2, is_server })
                        .style(iced::theme::Button::Custom(Box::new(DangerButton)))
                        .padding([2, 6]).into()
                );
            }
        }

        container(row(row_items).align_items(Alignment::Center).padding([4, 8]))
            .style(iced::theme::Container::Custom(Box::new(MemberCardStyle)))
            .width(Length::Fill)
            .into()
    }

    fn view_modal_add_group_member(&self, gid: Uuid) -> Element<'_, UiMessage> {
        let role_opts = ["Member", "Moderator", "Admin"];
        let role_btns: Vec<Element<UiMessage>> = role_opts.iter().enumerate().map(|(i, label)| {
            let active = self.modal_select == i;
            button(text(*label).size(12))
                .on_press(UiMessage::SelectChanged(i))
                .style(iced::theme::Button::Custom(
                    if active { Box::new(BlurpleButton) as Box<dyn button::StyleSheet<Style=iced::Theme>> }
                    else { Box::new(GhostButton) }
                ))
                .padding([4, 10])
                .into()
        }).collect();

        column![
            Self::modal_header("Add Member"),
            Space::with_height(12),
            text("User ID").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("xxxx...onion", &self.modal_f1)
                .on_input(UiMessage::Field1Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Display Name (optional)").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            text_input("Their name…", &self.modal_f2)
                .on_input(UiMessage::Field2Changed)
                .style(iced::theme::TextInput::Custom(Box::new(DiscordInput)))
                .padding(10).size(14),
            Space::with_height(8),
            text("Role").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            row(role_btns).spacing(4),
            Space::with_height(4),
            self.modal_error_label(),
            Space::with_height(12),
            row![
                button(text("Cancel").size(14))
                    .on_press(UiMessage::CloseModal)
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([8, 16]),
                Space::with_width(8),
                button(text("Add").size(14))
                    .on_press(UiMessage::AddMemberToGroup(gid))
                    .style(iced::theme::Button::Custom(Box::new(BlurpleButton)))
                    .padding([8, 16]),
            ].align_items(Alignment::Center),
        ]
        .spacing(4)
        .into()
    }

    fn view_modal_member_detail(&self, ctx_id: Uuid, user_id: &str, is_server: bool) -> Element<'_, UiMessage> {
        let roles = if is_server {
            vec!["Member", "Moderator", "Admin", "Co-Owner"]
        } else {
            vec!["Member", "Moderator", "Admin"]
        };

        let role_btns: Vec<Element<UiMessage>> = roles.iter().map(|label| {
            let uid = user_id.to_string();
            let lbl = label.to_string();
            button(text(*label).size(12))
                .on_press(UiMessage::SetRole {
                    context_id: ctx_id,
                    user_id: uid,
                    role_label: lbl,
                    is_server,
                })
                .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                .padding([4, 10])
                .into()
        }).collect();

        let uid = user_id.to_string();
        let uid2 = user_id.to_string();

        column![
            Self::modal_header("Member Actions"),
            Space::with_height(8),
            text(format!("User: {}", short_id(user_id))).size(13).style(iced::theme::Text::Color(TEXT_NORMAL)),
            Space::with_height(12),
            text("Assign Role").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            row(role_btns).spacing(4),
            Space::with_height(12),
            text("Moderation").size(12).style(iced::theme::Text::Color(TEXT_MUTED)),
            row![
                button(text("Kick").size(13))
                    .on_press(UiMessage::KickUser { context_id: ctx_id, user_id: uid.clone(), is_server })
                    .style(iced::theme::Button::Custom(Box::new(DestructiveFlatButton)))
                    .padding([6, 12]),
                Space::with_width(8),
                button(text("Ban").size(13))
                    .on_press(UiMessage::BanUser { context_id: ctx_id, user_id: uid2.clone(), is_server })
                    .style(iced::theme::Button::Custom(Box::new(DangerButton)))
                    .padding([6, 12]),
                Space::with_width(8),
                button(text("Mute").size(13))
                    .on_press(UiMessage::MuteUser { context_id: ctx_id, user_id: uid2, is_server })
                    .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                    .padding([6, 12]),
            ],
            Space::with_height(12),
            button(text("Close").size(14))
                .on_press(UiMessage::CloseModal)
                .style(iced::theme::Button::Custom(Box::new(GhostButton)))
                .padding([8, 16]),
        ]
        .spacing(4)
        .into()
    }

    fn modal_error_label(&self) -> Element<'_, UiMessage> {
        if let Some(e) = &self.modal_err {
            text(e.as_str()).size(13).style(iced::theme::Text::Color(RED)).into()
        } else {
            Space::with_height(0).into()
        }
    }
}

// ─── Helper type ─────────────────────────────────────────────────────────────

struct ChatMsg {
    from_id: String,
    from_name: String,
    body: String,
    timestamp: u64,
    outgoing: bool,
    edited: bool,
}

// ─── Helper free functions ────────────────────────────────────────────────────

fn pass_trivial(p: &str) -> bool {
    let has_upper = p.chars().any(|c| c.is_uppercase());
    let has_lower = p.chars().any(|c| c.is_lowercase());
    let has_digit = p.chars().any(|c| c.is_ascii_digit());
    !(has_upper && has_lower && has_digit)
}

// ─── BgDarkest, BgDark, BgMain, BgModal ──────────────────────────────────────

pub struct BgDarkest;
impl container::StyleSheet for BgDarkest {
    type Style = iced::Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_DARKEST)),
            text_color: Some(TEXT_NORMAL),
            ..Default::default()
        }
    }
}

pub struct BgDark;
impl container::StyleSheet for BgDark {
    type Style = iced::Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_DARK)),
            text_color: Some(TEXT_NORMAL),
            ..Default::default()
        }
    }
}

pub struct BgMain;
impl container::StyleSheet for BgMain {
    type Style = iced::Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(BG_MAIN)),
            text_color: Some(TEXT_NORMAL),
            ..Default::default()
        }
    }
}

pub struct BgModal;
impl container::StyleSheet for BgModal {
    type Style = iced::Theme;
    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 })),
            ..Default::default()
        }
    }
}
