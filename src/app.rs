// ─── NullChat Backend Server ──────────────────────────────────────────────────
// Communicates with the Python GUI via JSON-lines over a local TCP socket.
// Protocol:
//   Python → Rust : {"cmd":"...", ...fields...}
//   Rust → Python : {"event":"...", ...fields...}

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::crypto::identity::LocalIdentity;
use crate::model::{
    now_unix, generate_server_code, short_id,
    Channel, ChannelMessage, ChannelType,
    DirectMessage, Friend, GroupChat, GroupMember, GroupMessage, GroupRole,
    ModerationAction as ModAction, Notification, NotifKind,
    Server, ServerMember, ServerRole, UserStatus,
    WireKind, WireMessage,
};
use crate::network::p2p::P2PStatus;
use crate::storage::vault::EncryptedVault;

// ─── Fixed NullSec server UUIDs ──────────────────────────────────────────────
const NULLSEC_SERVER_ID: Uuid  = Uuid::from_bytes([0xa1,0xb2,0xc3,0xd4,0xe5,0xf6,0x78,0x90,0xab,0xcd,0xef,0x12,0x34,0x56,0x78,0x90]);
const NULLSEC_CH_ANN: Uuid     = Uuid::from_bytes([0xaa,0xbb,0xcc,0xdd,0x11,0x22,0x33,0x44,0x55,0x66,0x77,0x88,0x99,0x00,0x11,0x22]);
const NULLSEC_CH_CHAT: Uuid    = Uuid::from_bytes([0xbb,0xcc,0xdd,0xee,0x22,0x33,0x44,0x55,0x66,0x77,0x88,0x99,0x00,0x11,0x22,0x33]);
const NULLSEC_CH_HACK: Uuid    = Uuid::from_bytes([0xcc,0xdd,0xee,0xff,0x33,0x44,0x55,0x66,0x77,0x88,0x99,0x00,0x11,0x22,0x33,0x44]);
pub const NULLSEC_CODE: &str   = "NULLSEC0";

// ─── IPC types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Cmd {
    Setup   { name: String, pass: String },
    Unlock  { pass: String },
    SendDm  { friend_id: String, body: String },
    SendChannel { server_id: String, channel_id: String, body: String },
    SendGroup   { group_id: String, body: String },
    AddFriend   { user_id: String, name: String },
    CreateGroup { name: String, #[serde(default)] desc: String },
    JoinServer  { code: String },
    SaveProfile { name: String, #[serde(default)] nick: String, #[serde(default)] bio: String },
    SetStatus   { status: String },
    Kick        { context_id: String, user_id: String, is_server: bool },
    Ban         { context_id: String, user_id: String, is_server: bool },
    Unban       { context_id: String, user_id: String },
    Mute        { context_id: String, user_id: String, is_server: bool },
    Unmute      { context_id: String, user_id: String, is_server: bool },
    SetRole     { context_id: String, user_id: String, role: String, is_server: bool },
    AddGroupMember { group_id: String, user_id: String, #[serde(default)] name: String, #[serde(default)] role: String },
    GetState,
}

#[derive(Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Event {
    Phase  { phase: String },
    State  { data: StateSnapshot },
    Tor    { status: String, #[serde(skip_serializing_if = "Option::is_none")] onion: Option<String> },
    Notif  { kind: String, msg: String },
    Error  { msg: String },
}

// ─── Serialisable state snapshot sent to Python on every change ──────────────

#[derive(Debug, Serialize)]
struct StateSnapshot {
    my_name:    String,
    my_id:      String,
    my_nick:    Option<String>,
    my_status:  String,
    my_bio:     Option<String>,
    friends:    Vec<FriendSnap>,
    groups:     Vec<GroupSnap>,
    servers:    Vec<ServerSnap>,
    conversations: HashMap<String, Vec<MsgSnap>>,
    tor_status: String,
    unread:     HashMap<String, u32>,
}

#[derive(Debug, Serialize)]
struct FriendSnap {
    id: String,
    display_name: String,
    user_id: String,
}

#[derive(Debug, Serialize)]
struct GroupSnap {
    id: String,
    name: String,
    owner_id: String,
    members: Vec<MemberSnap>,
    messages: Vec<MsgSnap>,
}

#[derive(Debug, Serialize)]
struct ServerSnap {
    id: String,
    name: String,
    server_code: String,
    owner_id: String,
    description: Option<String>,
    channels: Vec<ChannelSnap>,
    members: Vec<MemberSnap>,
    banned_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChannelSnap {
    id: String,
    name: String,
    topic: Option<String>,
    ch_type: String,
    messages: Vec<MsgSnap>,
}

#[derive(Debug, Serialize)]
struct MemberSnap {
    user_id: String,
    display_name: String,
    role: String,
    muted: bool,
    banned: bool,
}

#[derive(Debug, Serialize)]
struct MsgSnap {
    id: String,
    from_id: String,
    from_name: String,
    body: String,
    ts: u64,
    outgoing: bool,
    edited: bool,
}

// ─── App state ───────────────────────────────────────────────────────────────

enum Phase { Setup, Unlock, Main }

struct AppState {
    phase: Phase,
    vault_path: PathBuf,
    my_name: String,
    my_user_id: String,
    my_nick: Option<String>,
    my_status: UserStatus,
    my_bio: Option<String>,
    friends: Vec<Friend>,
    groups: Vec<GroupChat>,
    servers: Vec<Server>,
    conversations: HashMap<Uuid, Vec<DirectMessage>>,
    tor_status: P2PStatus,
    incoming_queue: Arc<Mutex<Vec<WireMessage>>>,
    tor_socks: Option<String>,
    unread: HashMap<String, u32>,
}

impl AppState {
    fn new() -> Self {
        let vault_path = EncryptedVault::default_path();
        let is_first = EncryptedVault::is_first_run(&vault_path);
        let fp = LocalIdentity::generate().fingerprint_hex();
        let phase = if is_first { Phase::Setup } else { Phase::Unlock };

        let servers = Self::build_nullsec_server(&fp);

        AppState {
            phase,
            vault_path,
            my_name: String::from("Anonymous"),
            my_user_id: fp.clone(),
            my_nick: None,
            my_status: UserStatus::Online,
            my_bio: None,
            friends: Vec::new(),
            groups: Vec::new(),
            servers,
            conversations: HashMap::new(),
            tor_status: P2PStatus::Offline,
            incoming_queue: Arc::new(Mutex::new(Vec::new())),
            tor_socks: None,
            unread: HashMap::new(),
        }
    }

    fn build_nullsec_server(owner_id: &str) -> Vec<Server> {
        vec![Server {
            id: NULLSEC_SERVER_ID,
            name: String::from("NullSec Hacking Ground"),
            description: Some(String::from("Sovereign. Encrypted. Underground.")),
            server_code: String::from(NULLSEC_CODE),
            owner_id: owner_id.to_string(),
            channels: vec![
                Channel {
                    id: NULLSEC_CH_ANN,
                    name: String::from("announcements"),
                    topic: Some(String::from("Official announcements from the owner")),
                    channel_type: ChannelType::Announcement,
                    messages: Vec::new(),
                    position: 0,
                },
                Channel {
                    id: NULLSEC_CH_CHAT,
                    name: String::from("chat"),
                    topic: Some(String::from("General discussion — all members")),
                    channel_type: ChannelType::Public,
                    messages: Vec::new(),
                    position: 1,
                },
                Channel {
                    id: NULLSEC_CH_HACK,
                    name: String::from("hacking"),
                    topic: Some(String::from("Hacking drops and techniques")),
                    channel_type: ChannelType::Announcement,
                    messages: Vec::new(),
                    position: 2,
                },
            ],
            members: vec![ServerMember {
                user_id: owner_id.to_string(),
                display_name: String::from("Anonymous"),
                role: ServerRole::Owner,
                muted: false,
                banned: false,
                joined_at: now_unix(),
            }],
            banned_ids: Vec::new(),
            created_at: now_unix(),
            is_owned: true,
        }]
    }

    fn display_name(&self) -> &str {
        self.my_nick.as_deref().unwrap_or(&self.my_name)
    }

    fn snapshot(&self) -> StateSnapshot {
        let mut conversations: HashMap<String, Vec<MsgSnap>> = HashMap::new();
        for (fid, msgs) in &self.conversations {
            let name = self.friends.iter().find(|f| f.id == *fid)
                .map(|f| f.display_name.clone())
                .unwrap_or_else(|| String::from("Unknown"));
            conversations.insert(fid.to_string(), msgs.iter().map(|m| MsgSnap {
                id: m.id.to_string(),
                from_id: m.from_id.clone(),
                from_name: if m.outgoing { self.display_name().to_string() } else { name.clone() },
                body: m.body.clone(),
                ts: m.timestamp,
                outgoing: m.outgoing,
                edited: m.edited,
            }).collect());
        }

        StateSnapshot {
            my_name:   self.my_name.clone(),
            my_id:     self.my_user_id.clone(),
            my_nick:   self.my_nick.clone(),
            my_status: self.my_status.label().to_string(),
            my_bio:    self.my_bio.clone(),
            tor_status: self.tor_status.label().to_string(),
            unread:    self.unread.clone(),
            friends:   self.friends.iter().map(|f| FriendSnap {
                id: f.id.to_string(),
                display_name: f.display_name.clone(),
                user_id: f.user_id.clone(),
            }).collect(),
            groups: self.groups.iter().map(|g| GroupSnap {
                id: g.id.to_string(),
                name: g.name.clone(),
                owner_id: g.owner_id.clone(),
                members: g.members.iter().map(|m| MemberSnap {
                    user_id: m.user_id.clone(),
                    display_name: m.display_name.clone(),
                    role: m.role.label().to_string(),
                    muted: m.muted,
                    banned: m.banned,
                }).collect(),
                messages: g.messages.iter().map(|m| MsgSnap {
                    id: m.id.to_string(),
                    from_id: m.from_id.clone(),
                    from_name: m.from_name.clone(),
                    body: m.body.clone(),
                    ts: m.timestamp,
                    outgoing: m.from_id == self.my_user_id,
                    edited: m.edited,
                }).collect(),
            }).collect(),
            servers: self.servers.iter().map(|s| ServerSnap {
                id: s.id.to_string(),
                name: s.name.clone(),
                server_code: s.server_code.clone(),
                owner_id: s.owner_id.clone(),
                description: s.description.clone(),
                banned_ids: s.banned_ids.clone(),
                channels: s.channels.iter().map(|c| ChannelSnap {
                    id: c.id.to_string(),
                    name: c.name.clone(),
                    topic: c.topic.clone(),
                    ch_type: c.channel_type.label().to_string(),
                    messages: c.messages.iter().map(|m| MsgSnap {
                        id: m.id.to_string(),
                        from_id: m.from_id.clone(),
                        from_name: m.from_name.clone(),
                        body: m.body.clone(),
                        ts: m.timestamp,
                        outgoing: m.from_id == self.my_user_id,
                        edited: m.edited,
                    }).collect(),
                }).collect(),
                members: s.members.iter().map(|m| MemberSnap {
                    user_id: m.user_id.clone(),
                    display_name: m.display_name.clone(),
                    role: m.role.label().to_string(),
                    muted: m.muted,
                    banned: m.banned,
                }).collect(),
            }).collect(),
            conversations,
        }
    }

    fn my_role_in_server(&self, sid: Uuid) -> ServerRole {
        self.servers.iter()
            .find(|s| s.id == sid)
            .map(|s| s.my_role(&self.my_user_id))
            .unwrap_or(ServerRole::Member)
    }

    fn my_role_in_group(&self, gid: Uuid) -> GroupRole {
        self.groups.iter()
            .find(|g| g.id == gid)
            .and_then(|g| g.members.iter().find(|m| m.user_id == self.my_user_id))
            .map(|m| m.role.clone())
            .unwrap_or(GroupRole::Member)
    }

    fn am_muted_server(&self, sid: Uuid) -> bool {
        self.servers.iter()
            .find(|s| s.id == sid)
            .and_then(|s| s.members.iter().find(|m| m.user_id == self.my_user_id))
            .map(|m| m.muted)
            .unwrap_or(false)
    }

    fn handle_incoming(&mut self, wire: WireMessage) {
        match &wire.kind.clone() {
            WireKind::DirectMessage => {
                let fid = match self.friends.iter().find(|f| f.user_id == wire.from_id) {
                    Some(f) => f.id,
                    None => {
                        let fid = Uuid::new_v4();
                        self.friends.push(Friend {
                            id: fid,
                            display_name: wire.from_name.clone(),
                            user_id: wire.from_id.clone(),
                            note: None,
                            added_at: now_unix(),
                            last_seen: None,
                        });
                        fid
                    }
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
                *self.unread.entry(fid.to_string()).or_insert(0) += 1;
            }
            WireKind::GroupMessage { group_id } => {
                if let Ok(gid) = group_id.parse::<Uuid>() {
                    if let Some(g) = self.groups.iter_mut().find(|g| g.id == gid) {
                        g.messages.push(GroupMessage {
                            id: Uuid::new_v4(),
                            from_id: wire.from_id.clone(),
                            from_name: wire.from_name.clone(),
                            body: wire.body.clone(),
                            timestamp: wire.timestamp,
                            edited: false,
                            reactions: Vec::new(),
                        });
                        *self.unread.entry(gid.to_string()).or_insert(0) += 1;
                    }
                }
            }
            WireKind::ChannelMessage { server_id, channel_id } => {
                if let (Ok(sid), Ok(cid)) = (server_id.parse::<Uuid>(), channel_id.parse::<Uuid>()) {
                    if let Some(s) = self.servers.iter_mut().find(|s| s.id == sid) {
                        if let Some(ch) = s.channels.iter_mut().find(|c| c.id == cid) {
                            ch.messages.push(ChannelMessage {
                                id: Uuid::new_v4(),
                                from_id: wire.from_id.clone(),
                                from_name: wire.from_name.clone(),
                                body: wire.body.clone(),
                                timestamp: wire.timestamp,
                                edited: false,
                                reactions: Vec::new(),
                            });
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
                }
            }
            _ => {}
        }
    }
}

// ─── Backend entry point ─────────────────────────────────────────────────────

pub async fn run(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).await
        .expect("Failed to bind backend IPC port");
    tracing::info!("Backend IPC listening on {}", addr);

    // Accept connections in a loop so Python can reconnect if needed
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                tracing::info!("GUI connected from {}", peer);
                handle_connection(stream).await;
                tracing::info!("GUI disconnected");
            }
            Err(e) => {
                tracing::error!("Accept error: {}", e);
                break;
            }
        }
    }
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    let mut state = AppState::new();
    let incoming = state.incoming_queue.clone();

    // Send initial phase
    let initial_phase = match &state.phase {
        Phase::Setup  => "setup",
        Phase::Unlock => "unlock",
        Phase::Main   => "main",
    };
    send_event(&mut writer, &Event::Phase { phase: initial_phase.to_string() }).await;

    // Background task: drain incoming P2P queue every 250ms
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WireMessage>(256);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
            let msgs: Vec<WireMessage> = incoming.lock().await.drain(..).collect();
            for m in msgs {
                if tx.send(m).await.is_err() { break; }
            }
        }
    });

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(raw)) => {
                        let raw = raw.trim().to_string();
                        if raw.is_empty() { continue; }
                        match serde_json::from_str::<Cmd>(&raw) {
                            Ok(cmd) => {
                                let evts = process_cmd(&mut state, cmd).await;
                                for evt in evts {
                                    send_event(&mut writer, &evt).await;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Bad command: {} — {}", raw, e);
                                send_event(&mut writer, &Event::Error { msg: format!("Parse error: {}", e) }).await;
                            }
                        }
                    }
                    Ok(None) => break, // connection closed
                    Err(e)   => { tracing::warn!("Read error: {}", e); break; }
                }
            }
            Some(wire) = rx.recv() => {
                state.handle_incoming(wire);
                send_event(&mut writer, &Event::State { data: state.snapshot() }).await;
            }
        }
    }
}

async fn send_event(writer: &mut tokio::net::tcp::OwnedWriteHalf, evt: &Event) {
    if let Ok(mut json) = serde_json::to_string(evt) {
        json.push('\n');
        let _ = writer.write_all(json.as_bytes()).await;
    }
}

// ─── Command processor ───────────────────────────────────────────────────────

async fn process_cmd(s: &mut AppState, cmd: Cmd) -> Vec<Event> {
    match cmd {
        Cmd::Setup { name, pass } => {
            let n = name.trim().to_string();
            let p = pass.trim().to_string();
            if n.is_empty() {
                return vec![Event::Error { msg: "Enter a display name.".into() }];
            }
            if p.len() < 12 {
                return vec![Event::Error { msg: "Passphrase must be ≥12 characters.".into() }];
            }
            let mut vault = EncryptedVault::new();
            match vault.open(&s.vault_path.clone(), &p) {
                Ok(()) => {
                    s.my_name = n.clone();
                    // Update nullsec server member display name
                    if let Some(srv) = s.servers.iter_mut().find(|x| x.id == NULLSEC_SERVER_ID) {
                        if let Some(m) = srv.members.iter_mut().find(|m| m.user_id == s.my_user_id) {
                            m.display_name = n;
                        }
                    }
                    s.phase = Phase::Main;
                    let tor_events = start_tor(s).await;
                    let mut evts = vec![
                        Event::Phase { phase: "main".into() },
                        Event::State { data: s.snapshot() },
                    ];
                    evts.extend(tor_events);
                    evts
                }
                Err(e) => vec![Event::Error { msg: format!("Vault error: {}", e) }],
            }
        }

        Cmd::Unlock { pass } => {
            let p = pass.trim().to_string();
            if p.is_empty() {
                return vec![Event::Error { msg: "Enter your passphrase.".into() }];
            }
            let mut vault = EncryptedVault::new();
            match vault.open(&s.vault_path.clone(), &p) {
                Ok(()) => {
                    s.phase = Phase::Main;
                    let tor_events = start_tor(s).await;
                    let mut evts = vec![
                        Event::Phase { phase: "main".into() },
                        Event::State { data: s.snapshot() },
                    ];
                    evts.extend(tor_events);
                    evts
                }
                Err(crate::storage::vault::VaultError::Decryption) =>
                    vec![Event::Error { msg: "Incorrect passphrase.".into() }],
                Err(e) => vec![Event::Error { msg: format!("Vault error: {}", e) }],
            }
        }

        Cmd::GetState => vec![Event::State { data: s.snapshot() }],

        Cmd::SendDm { friend_id, body } => {
            let body = body.trim().to_string();
            if body.is_empty() { return vec![]; }
            if let Ok(fid) = friend_id.parse::<Uuid>() {
                let from_id   = s.my_user_id.clone();
                let from_name = s.display_name().to_string();
                let ts = now_unix();
                s.conversations.entry(fid).or_default().push(DirectMessage {
                    id: Uuid::new_v4(),
                    from_id: from_id.clone(),
                    body: body.clone(),
                    timestamp: ts,
                    outgoing: true,
                    edited: false,
                    reactions: Vec::new(),
                });
                // Fire-and-forget P2P send
                if let Some(friend) = s.friends.iter().find(|f| f.id == fid) {
                    let peer = friend.user_id.clone();
                    let socks = s.tor_socks.clone();
                    let wire = WireMessage {
                        kind: WireKind::DirectMessage,
                        from_id, from_name,
                        target_id: peer.clone(),
                        body, timestamp: ts,
                    };
                    tokio::spawn(async move {
                        let _ = crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await;
                    });
                }
            }
            vec![Event::State { data: s.snapshot() }]
        }

        Cmd::SendChannel { server_id, channel_id, body } => {
            let body = body.trim().to_string();
            if body.is_empty() { return vec![]; }
            let (sid, cid) = match (server_id.parse::<Uuid>(), channel_id.parse::<Uuid>()) {
                (Ok(a), Ok(b)) => (a, b),
                _ => return vec![Event::Error { msg: "Invalid IDs.".into() }],
            };
            let my_role = s.my_role_in_server(sid);
            let can_send = s.servers.iter()
                .find(|s| s.id == sid)
                .and_then(|srv| srv.channels.iter().find(|c| c.id == cid))
                .map(|ch| s.servers.iter().find(|s| s.id == sid).unwrap().can_send_in(ch, &my_role))
                .unwrap_or(false);
            if !can_send || s.am_muted_server(sid) {
                return vec![Event::Error { msg: "You cannot send messages in this channel.".into() }];
            }
            let from_id = s.my_user_id.clone();
            let from_name = s.display_name().to_string();
            let ts = now_unix();
            if let Some(srv) = s.servers.iter_mut().find(|s| s.id == sid) {
                if let Some(ch) = srv.channels.iter_mut().find(|c| c.id == cid) {
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
                let members: Vec<String> = srv.members.iter()
                    .map(|m| m.user_id.clone())
                    .filter(|uid| uid != &from_id)
                    .collect();
                let socks = s.tor_socks.clone();
                let wire = WireMessage {
                    kind: WireKind::ChannelMessage { server_id: sid.to_string(), channel_id: cid.to_string() },
                    from_id, from_name, target_id: cid.to_string(), body, timestamp: ts,
                };
                tokio::spawn(async move {
                    for peer in members {
                        let _ = crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await;
                    }
                });
            }
            vec![Event::State { data: s.snapshot() }]
        }

        Cmd::SendGroup { group_id, body } => {
            let body = body.trim().to_string();
            if body.is_empty() { return vec![]; }
            if let Ok(gid) = group_id.parse::<Uuid>() {
                let muted = s.groups.iter()
                    .find(|g| g.id == gid)
                    .and_then(|g| g.members.iter().find(|m| m.user_id == s.my_user_id))
                    .map(|m| m.muted)
                    .unwrap_or(false);
                if muted { return vec![Event::Error { msg: "You are muted in this group.".into() }]; }
                let from_id = s.my_user_id.clone();
                let from_name = s.display_name().to_string();
                let ts = now_unix();
                if let Some(g) = s.groups.iter_mut().find(|x| x.id == gid) {
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
                    let socks = s.tor_socks.clone();
                    let wire = WireMessage {
                        kind: WireKind::GroupMessage { group_id: gid.to_string() },
                        from_id, from_name, target_id: gid.to_string(), body, timestamp: ts,
                    };
                    tokio::spawn(async move {
                        for peer in members {
                            let _ = crate::network::p2p::send_to_peer(&peer, &wire, socks.as_deref()).await;
                        }
                    });
                }
            }
            vec![Event::State { data: s.snapshot() }]
        }

        Cmd::AddFriend { user_id, name } => {
            let uid  = user_id.trim().to_string();
            let name = name.trim().to_string();
            if uid.is_empty()  { return vec![Event::Error { msg: "Enter a User ID.".into() }]; }
            if name.is_empty() { return vec![Event::Error { msg: "Enter a display name.".into() }]; }
            if s.friends.iter().any(|f| f.user_id == uid) {
                return vec![Event::Error { msg: "Already in your friends list.".into() }];
            }
            s.friends.push(Friend {
                id: Uuid::new_v4(), display_name: name,
                user_id: uid, note: None, added_at: now_unix(), last_seen: None,
            });
            vec![
                Event::Notif { kind: "success".into(), msg: "Friend added!".into() },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::CreateGroup { name, desc } => {
            let name = name.trim().to_string();
            if name.is_empty() { return vec![Event::Error { msg: "Enter a group name.".into() }]; }
            let gid = Uuid::new_v4();
            s.groups.push(GroupChat {
                id: gid,
                name,
                description: if desc.is_empty() { None } else { Some(desc) },
                owner_id: s.my_user_id.clone(),
                members: vec![GroupMember {
                    user_id: s.my_user_id.clone(),
                    display_name: s.display_name().to_string(),
                    role: GroupRole::Owner, muted: false, banned: false, joined_at: now_unix(),
                }],
                messages: Vec::new(),
                created_at: now_unix(),
            });
            vec![
                Event::Notif { kind: "success".into(), msg: "Group created!".into() },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::JoinServer { code } => {
            let code = code.trim().to_uppercase();
            if code.len() != 8 {
                return vec![Event::Error { msg: "Server code is 8 characters.".into() }];
            }
            let my_id = s.my_user_id.clone();
            let my_name = s.display_name().to_string();
            let found = s.servers.iter().position(|srv| srv.server_code == code);
            match found {
                None => vec![Event::Error { msg: "Server not found. Check the code.".into() }],
                Some(idx) => {
                    if s.servers[idx].banned_ids.contains(&my_id) {
                        return vec![Event::Error { msg: "You are banned from this server.".into() }];
                    }
                    if !s.servers[idx].members.iter().any(|m| m.user_id == my_id) {
                        s.servers[idx].members.push(ServerMember {
                            user_id: my_id, display_name: my_name,
                            role: ServerRole::Member, muted: false, banned: false, joined_at: now_unix(),
                        });
                    }
                    vec![
                        Event::Notif { kind: "success".into(), msg: "Joined server!".into() },
                        Event::State { data: s.snapshot() },
                    ]
                }
            }
        }

        Cmd::SaveProfile { name, nick, bio } => {
            let n = name.trim().to_string();
            if !n.is_empty() {
                s.my_name = n.clone();
                if let Some(srv) = s.servers.iter_mut().find(|x| x.id == NULLSEC_SERVER_ID) {
                    if let Some(m) = srv.members.iter_mut().find(|m| m.user_id == s.my_user_id) {
                        m.display_name = n;
                    }
                }
            }
            s.my_nick = if nick.trim().is_empty() { None } else { Some(nick.trim().to_string()) };
            s.my_bio  = if bio.trim().is_empty()  { None } else { Some(bio.trim().to_string()) };
            vec![
                Event::Notif { kind: "success".into(), msg: "Profile saved!".into() },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::SetStatus { status } => {
            s.my_status = match status.as_str() {
                "Away"           => UserStatus::Away,
                "DoNotDisturb"   => UserStatus::DoNotDisturb,
                "Invisible"      => UserStatus::Invisible,
                _                => UserStatus::Online,
            };
            vec![Event::State { data: s.snapshot() }]
        }

        Cmd::Kick { context_id, user_id, is_server } => {
            if let Ok(cid) = context_id.parse::<Uuid>() {
                if is_server {
                    if s.my_role_in_server(cid).can_moderate() {
                        if let Some(srv) = s.servers.iter_mut().find(|s| s.id == cid) {
                            srv.members.retain(|m| m.user_id != user_id);
                        }
                    }
                } else {
                    if s.my_role_in_group(cid).can_moderate() {
                        if let Some(g) = s.groups.iter_mut().find(|g| g.id == cid) {
                            g.members.retain(|m| m.user_id != user_id);
                        }
                    }
                }
            }
            vec![
                Event::Notif { kind: "warn".into(), msg: format!("Kicked {}.", short_id(&user_id)) },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::Ban { context_id, user_id, is_server } => {
            if let Ok(cid) = context_id.parse::<Uuid>() {
                if is_server && s.my_role_in_server(cid).can_moderate() {
                    if let Some(srv) = s.servers.iter_mut().find(|s| s.id == cid) {
                        srv.members.retain(|m| m.user_id != user_id);
                        if !srv.banned_ids.contains(&user_id) {
                            srv.banned_ids.push(user_id.clone());
                        }
                    }
                }
            }
            vec![
                Event::Notif { kind: "error".into(), msg: format!("Banned {}.", short_id(&user_id)) },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::Unban { context_id, user_id } => {
            if let Ok(cid) = context_id.parse::<Uuid>() {
                if let Some(srv) = s.servers.iter_mut().find(|s| s.id == cid) {
                    srv.banned_ids.retain(|id| *id != user_id);
                }
            }
            vec![
                Event::Notif { kind: "info".into(), msg: format!("Unbanned {}.", short_id(&user_id)) },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::Mute { context_id, user_id, is_server } => {
            if let Ok(cid) = context_id.parse::<Uuid>() {
                if is_server {
                    if let Some(srv) = s.servers.iter_mut().find(|s| s.id == cid) {
                        if let Some(m) = srv.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = true;
                        }
                    }
                } else {
                    if let Some(g) = s.groups.iter_mut().find(|g| g.id == cid) {
                        if let Some(m) = g.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = true;
                        }
                    }
                }
            }
            vec![
                Event::Notif { kind: "warn".into(), msg: format!("Muted {}.", short_id(&user_id)) },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::Unmute { context_id, user_id, is_server } => {
            if let Ok(cid) = context_id.parse::<Uuid>() {
                if is_server {
                    if let Some(srv) = s.servers.iter_mut().find(|s| s.id == cid) {
                        if let Some(m) = srv.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = false;
                        }
                    }
                } else {
                    if let Some(g) = s.groups.iter_mut().find(|g| g.id == cid) {
                        if let Some(m) = g.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.muted = false;
                        }
                    }
                }
            }
            vec![
                Event::Notif { kind: "info".into(), msg: format!("Unmuted {}.", short_id(&user_id)) },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::SetRole { context_id, user_id, role, is_server } => {
            if let Ok(cid) = context_id.parse::<Uuid>() {
                if is_server && s.my_role_in_server(cid).can_manage_roles() {
                    if let Some(srv) = s.servers.iter_mut().find(|s| s.id == cid) {
                        if let Some(m) = srv.members.iter_mut().find(|m| m.user_id == user_id) {
                            m.role = match role.as_str() {
                                "Co-Owner"   => ServerRole::CoOwner,
                                "Admin"      => ServerRole::Admin,
                                "Moderator"  => ServerRole::Moderator,
                                _            => ServerRole::Member,
                            };
                        }
                    }
                }
            }
            vec![
                Event::Notif { kind: "success".into(), msg: format!("Role updated to {}.", role) },
                Event::State { data: s.snapshot() },
            ]
        }

        Cmd::AddGroupMember { group_id, user_id, name, role } => {
            let uid = user_id.trim().to_string();
            if uid.is_empty() { return vec![Event::Error { msg: "Enter a User ID.".into() }]; }
            if let Ok(gid) = group_id.parse::<Uuid>() {
                if let Some(g) = s.groups.iter_mut().find(|g| g.id == gid) {
                    if !g.members.iter().any(|m| m.user_id == uid) {
                        let nm = if name.trim().is_empty() { short_id(&uid) } else { name.trim().to_string() };
                        g.members.push(GroupMember {
                            user_id: uid,
                            display_name: nm,
                            role: match role.as_str() {
                                "Admin"     => GroupRole::Admin,
                                "Moderator" => GroupRole::Moderator,
                                _           => GroupRole::Member,
                            },
                            muted: false, banned: false, joined_at: now_unix(),
                        });
                    }
                }
            }
            vec![
                Event::Notif { kind: "success".into(), msg: "Member added!".into() },
                Event::State { data: s.snapshot() },
            ]
        }
    }
}

// ─── Tor startup (non-blocking) ──────────────────────────────────────────────

async fn start_tor(s: &mut AppState) -> Vec<Event> {
    let data_dir = s.vault_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let queue = s.incoming_queue.clone();

    // Start P2P listener first
    let _ = crate::network::p2p::start_listener(queue).await;

    // Probe existing system Tor
    if crate::network::p2p::probe_system_tor().await {
        s.tor_status = P2PStatus::TorReady { onion: format!("{}@tor", short_id(&s.my_user_id)) };
        s.tor_socks = Some(crate::network::p2p::TOR_SOCKS_SYSTEM.to_string());
        return vec![Event::Tor {
            status: "Tor Connected".into(),
            onion: Some(s.my_user_id.clone()),
        }];
    }

    // Try to start bundled Tor hidden service
    match crate::network::p2p::start_hidden_service(&data_dir).await {
        Ok((_child, onion)) => {
            let old = s.my_user_id.clone();
            let dn  = s.display_name().to_string();
            s.my_user_id = onion.clone();
            s.tor_socks  = Some(crate::network::p2p::TOR_SOCKS_LOCAL.to_string());
            s.tor_status = P2PStatus::TorReady { onion: onion.clone() };
            // Sync NullSec identity
            if let Some(srv) = s.servers.iter_mut().find(|x| x.id == NULLSEC_SERVER_ID) {
                if srv.owner_id == old { srv.owner_id = onion.clone(); }
                for m in srv.members.iter_mut() {
                    if m.user_id == old { m.user_id = onion.clone(); m.display_name = dn; break; }
                }
            }
            vec![Event::Tor { status: "Tor Ready".into(), onion: Some(onion) }]
        }
        Err(e) => {
            tracing::warn!("Tor hidden service failed: {}", e);
            s.tor_status = P2PStatus::Error(e.to_string());
            vec![Event::Tor { status: format!("Tor unavailable: {}", e), onion: None }]
        }
    }
}

