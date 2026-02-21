use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── User Identity ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub user_id: String, // .onion address when Tor is active, else fingerprint hex
    pub created_at: u64,
}

// ─── Friends ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub id: Uuid,
    pub display_name: String,
    pub user_id: String, // their .onion address / public fingerprint
    pub added_at: u64,
}

// ─── Direct Messages ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub id: Uuid,
    pub from_id: String,
    pub body: String,
    pub timestamp: u64,
    pub outgoing: bool,
}

// ─── Group Chats ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub user_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupChat {
    pub id: Uuid,
    pub name: String,
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
}

// ─── Servers ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub server_code: String, // 8-char invite code, shown only to owner
    pub owner_id: String,
    pub channels: Vec<Channel>,
    pub member_ids: Vec<String>,
    pub created_at: u64,
    pub is_owned: bool, // true = we created this server
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub messages: Vec<ChannelMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub id: Uuid,
    pub from_id: String,
    pub from_name: String,
    pub body: String,
    pub timestamp: u64,
}

// ─── Wire Protocol ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub kind: WireKind,
    pub from_id: String,
    pub from_name: String,
    pub target_id: String, // friend user_id, group uuid, channel uuid
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
    Ping,
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

pub fn user_color_for(user_id: &str) -> iced::Color {
    let hash: u32 = user_id
        .bytes()
        .take(16)
        .fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32));
    let palette = [
        iced::Color { r: 88.0 / 255.0, g: 101.0 / 255.0, b: 242.0 / 255.0, a: 1.0 },
        iced::Color { r: 59.0 / 255.0, g: 165.0 / 255.0, b: 93.0 / 255.0, a: 1.0 },
        iced::Color { r: 250.0 / 255.0, g: 168.0 / 255.0, b: 26.0 / 255.0, a: 1.0 },
        iced::Color { r: 235.0 / 255.0, g: 69.0 / 255.0, b: 158.0 / 255.0, a: 1.0 },
        iced::Color { r: 26.0 / 255.0, g: 188.0 / 255.0, b: 156.0 / 255.0, a: 1.0 },
        iced::Color { r: 149.0 / 255.0, g: 55.0 / 255.0, b: 255.0 / 255.0, a: 1.0 },
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
    if id.len() > 12 {
        format!("{}...", &id[..10])
    } else {
        id.to_string()
    }
}
