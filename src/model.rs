use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Server Roles ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerRole {
    Owner,
    CoOwner,
    Admin,
    Moderator,
    Member,
}

impl ServerRole {
    pub fn label(&self) -> &'static str {
        match self {
            ServerRole::Owner => "Owner",
            ServerRole::CoOwner => "Co-Owner",
            ServerRole::Admin => "Admin",
            ServerRole::Moderator => "Moderator",
            ServerRole::Member => "Member",
        }
    }
    pub fn badge_color(&self) -> iced::Color {
        match self {
            ServerRole::Owner => iced::Color { r: 0.98, g: 0.66, b: 0.10, a: 1.0 },
            ServerRole::CoOwner => iced::Color { r: 0.98, g: 0.66, b: 0.10, a: 0.75 },
            ServerRole::Admin => iced::Color { r: 0.93, g: 0.26, b: 0.27, a: 1.0 },
            ServerRole::Moderator => iced::Color { r: 0.34, g: 0.40, b: 0.95, a: 1.0 },
            ServerRole::Member => iced::Color { r: 0.55, g: 0.57, b: 0.59, a: 1.0 },
        }
    }
    pub fn can_moderate(&self) -> bool {
        matches!(self, ServerRole::Owner | ServerRole::CoOwner | ServerRole::Admin | ServerRole::Moderator)
    }
    pub fn can_manage_roles(&self) -> bool {
        matches!(self, ServerRole::Owner | ServerRole::CoOwner | ServerRole::Admin)
    }
    pub fn can_manage_channels(&self) -> bool {
        matches!(self, ServerRole::Owner | ServerRole::CoOwner | ServerRole::Admin)
    }
}

// ─── Group Roles ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupRole {
    Owner,
    Admin,
    Moderator,
    Member,
}

impl GroupRole {
    pub fn label(&self) -> &'static str {
        match self {
            GroupRole::Owner => "Owner",
            GroupRole::Admin => "Admin",
            GroupRole::Moderator => "Moderator",
            GroupRole::Member => "Member",
        }
    }
    pub fn can_moderate(&self) -> bool {
        matches!(self, GroupRole::Owner | GroupRole::Admin | GroupRole::Moderator)
    }
}

// ─── Channel Type ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    /// Everyone can read and write
    Public,
    /// Everyone can read, only staff (Mod+) can write
    ReadOnly,
    /// Only staff (Mod+) can see and use
    StaffOnly,
    /// Only Owner/Co-Owner/Admin can post (pinboard style)
    Announcement,
}

impl ChannelType {
    pub fn label(&self) -> &'static str {
        match self {
            ChannelType::Public => "Public",
            ChannelType::ReadOnly => "Read-Only",
            ChannelType::StaffOnly => "Staff Only",
            ChannelType::Announcement => "Announcement",
        }
    }
    pub fn icon(&self) -> &'static str {
        match self {
            ChannelType::Public => "#",
            ChannelType::ReadOnly => "👁",
            ChannelType::StaffOnly => "🔒",
            ChannelType::Announcement => "📢",
        }
    }
}

// ─── User Profile ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub user_id: String,
    pub nick: Option<String>, // global nickname override
    pub status: UserStatus,
    pub bio: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Online,
    Away,
    DoNotDisturb,
    Invisible,
}

impl UserStatus {
    pub fn color(&self) -> iced::Color {
        match self {
            UserStatus::Online => iced::Color { r: 0.23, g: 0.65, b: 0.37, a: 1.0 },
            UserStatus::Away => iced::Color { r: 0.98, g: 0.66, b: 0.10, a: 1.0 },
            UserStatus::DoNotDisturb => iced::Color { r: 0.93, g: 0.26, b: 0.27, a: 1.0 },
            UserStatus::Invisible => iced::Color { r: 0.55, g: 0.57, b: 0.59, a: 1.0 },
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            UserStatus::Online => "Online",
            UserStatus::Away => "Away",
            UserStatus::DoNotDisturb => "Do Not Disturb",
            UserStatus::Invisible => "Invisible",
        }
    }
}

// ─── Friends ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub id: Uuid,
    pub display_name: String,
    pub user_id: String,
    pub note: Option<String>,
    pub added_at: u64,
    pub last_seen: Option<u64>,
}

// ─── Direct Messages ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub id: Uuid,
    pub from_id: String,
    pub body: String,
    pub timestamp: u64,
    pub outgoing: bool,
    pub edited: bool,
    pub reactions: Vec<Reaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub count: u32,
    pub reacted_by_me: bool,
}

// ─── Group Chats ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub user_id: String,
    pub display_name: String,
    pub role: GroupRole,
    pub muted: bool,
    pub banned: bool,
    pub joined_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupChat {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub members: Vec<GroupMember>,
    pub messages: Vec<GroupMessage>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessage {
    pub id: Uuid,
    pub from_id: String,
    pub from_name: String,
    pub body: String,
    pub timestamp: u64,
    pub edited: bool,
    pub reactions: Vec<Reaction>,
}

// ─── Servers ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMember {
    pub user_id: String,
    pub display_name: String,
    pub role: ServerRole,
    pub muted: bool,
    pub banned: bool,
    pub joined_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub server_code: String,
    pub owner_id: String,
    pub channels: Vec<Channel>,
    pub members: Vec<ServerMember>,
    pub banned_ids: Vec<String>,
    pub created_at: u64,
    pub is_owned: bool,
}

impl Server {
    pub fn my_role(&self, my_id: &str) -> ServerRole {
        self.members.iter()
            .find(|m| m.user_id == my_id)
            .map(|m| m.role.clone())
            .unwrap_or(ServerRole::Member)
    }

    pub fn can_view_channel(&self, ch: &Channel, my_role: &ServerRole) -> bool {
        match ch.channel_type {
            ChannelType::StaffOnly => my_role.can_moderate(),
            _ => true,
        }
    }

    pub fn can_send_in(&self, ch: &Channel, my_role: &ServerRole) -> bool {
        match ch.channel_type {
            ChannelType::Public => true,
            ChannelType::ReadOnly => my_role.can_moderate(),
            ChannelType::Announcement => my_role.can_manage_channels(),
            ChannelType::StaffOnly => my_role.can_moderate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub topic: Option<String>,
    pub channel_type: ChannelType,
    pub messages: Vec<ChannelMessage>,
    pub position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: Uuid,
    pub from_id: String,
    pub from_name: String,
    pub body: String,
    pub timestamp: u64,
    pub edited: bool,
    pub reactions: Vec<Reaction>,
}

// ─── Notifications ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub kind: NotifKind,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifKind {
    Info,
    Success,
    Warning,
    Error,
}

impl Notification {
    pub fn info(msg: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), kind: NotifKind::Info, message: msg.into(), timestamp: now_unix() }
    }
    pub fn success(msg: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), kind: NotifKind::Success, message: msg.into(), timestamp: now_unix() }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), kind: NotifKind::Error, message: msg.into(), timestamp: now_unix() }
    }
    pub fn warn(msg: impl Into<String>) -> Self {
        Self { id: Uuid::new_v4(), kind: NotifKind::Warning, message: msg.into(), timestamp: now_unix() }
    }
}

// ─── Wire Protocol ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub kind: WireKind,
    pub from_id: String,
    pub from_name: String,
    pub target_id: String,
    pub body: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WireKind {
    DirectMessage,
    GroupMessage { group_id: String },
    ChannelMessage { server_id: String, channel_id: String },
    FriendRequest,
    GroupInvite { group_id: String, group_name: String },
    ModerationAction { target_uid: String, action: ModerationAction, context_id: String },
    NicknameUpdate { new_nick: String },
    RoleAssignment { target_uid: String, role_label: String, context_id: String },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModerationAction {
    Kick,
    Ban,
    Unban,
    Mute,
    Unmute,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn now_unix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn format_ts(ts: u64) -> String {
    let h = (ts / 3600) % 24;
    let m = (ts / 60) % 60;
    format!("{:02}:{:02}", h, m)
}

pub fn format_date_ts(ts: u64) -> String {
    let h = (ts / 3600) % 24;
    let m = (ts / 60) % 60;
    let day = ts / 86400;
    format!("Day {} {:02}:{:02}", day, h, m)
}

pub fn user_color_for(user_id: &str) -> iced::Color {
    let hash: u32 = user_id
        .bytes()
        .take(16)
        .fold(5381u32, |acc, b| acc.wrapping_mul(33).wrapping_add(b as u32));
    let palette = [
        iced::Color { r: 0.345, g: 0.396, b: 0.949, a: 1.0 },
        iced::Color { r: 0.231, g: 0.647, b: 0.365, a: 1.0 },
        iced::Color { r: 0.980, g: 0.659, b: 0.102, a: 1.0 },
        iced::Color { r: 0.922, g: 0.271, b: 0.620, a: 1.0 },
        iced::Color { r: 0.102, g: 0.737, b: 0.612, a: 1.0 },
        iced::Color { r: 0.584, g: 0.216, b: 0.996, a: 1.0 },
        iced::Color { r: 0.988, g: 0.447, b: 0.243, a: 1.0 },
    ];
    palette[(hash as usize) % palette.len()]
}

pub fn user_initials(name: &str) -> String {
    let mut parts = name.split_whitespace();
    let first = parts.next().and_then(|s| s.chars().next()).unwrap_or('?');
    let second = parts.next().and_then(|s| s.chars().next());
    if let Some(s) = second {
        format!("{}{}", first.to_uppercase(), s.to_uppercase())
    } else {
        first.to_uppercase().to_string()
    }
}

pub fn generate_server_code() -> String {
    use rand::Rng;
    let charset = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect()
}

pub fn short_id(id: &str) -> String {
    if id.len() > 14 {
        format!("{}..{}", &id[..6], &id[id.len()-4..])
    } else {
        id.to_string()
    }
}

pub fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
