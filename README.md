# Null Chat

**Sovereign post-quantum secure messenger routed over Tor.**

Null Chat is a fully offline-capable, end-to-end encrypted chat application
that routes all traffic through the Tor network. It features a Discord-style
interface with servers, channels, group chats, direct messages, and a complete
staff moderation hierarchy — all without any central server.

---

## Features

### Security
- **Post-quantum key exchange** — ML-KEM (Kyber) + X25519 hybrid KEM
- **Double Ratchet** session encryption for forward secrecy
- **AES-256-GCM** for all vault storage
- **Argon2id** for passphrase key derivation
- **Tor hidden services** — your identity is your `.onion` address
- **Encrypted vault** — all data stored locally, encrypted at rest
- **Panic engine** — `Ctrl+Alt+Shift+X` destroys all local data instantly
- **Secure delete** — multiple-pass overwrite on sensitive files

### Communication
- **Direct Messages** — peer-to-peer over Tor, no relay server
- **Group Chats** — multi-party messaging with owner/admin/moderator/member roles
- **Servers** — Discord-like servers with named channels
- **Real Tor P2P** — messages sent via SOCKS5 proxy directly to `.onion` peers

### User Features
- **Global nickname** — set an optional display alias separate from your vault name
- **Status** — Online, Away, Do Not Disturb, Invisible
- **Profile / Bio** — optional description visible to peers
- **Friend list** — add friends by `.onion` User ID

### Server & Group Management
- **Role hierarchy** — Owner → Co-Owner → Admin → Moderator → Member
- **Channel types:**
  - `Public` — everyone can read and write
  - `Read-Only` — everyone can read, only staff (Mod+) can write
  - `Staff Only` — only staff can see and use
  - `Announcement` — only Owner/Admin can post
- **Moderation actions** — Kick, Ban, Mute, Unmute per user
- **Ban list management** — view and unban server members
- **Role assignment** — promote/demote members in servers and groups
- **Server editing** — rename server, update description
- **Channel creation** — create channels with custom types
- **Invite codes** — 8-character server codes for inviting members

### UI
- **Discord-inspired layout** — server rail, sidebar, main chat area
- **Premium dark theme** — Discord colour palette with smooth interactions
- **Unread badges** — per-DM, group, and channel
- **Notification toasts** — dismissable info/success/warn/error notifications
- **Avatar initials** — colour-coded per user ID
- **Role badges** — colour-coded role labels in member lists

---

## Architecture

```
null-chat/
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # iced Application wrapper + P2P subscription
│   ├── model.rs             # All data models + wire protocol
│   ├── panic_engine.rs      # Emergency data destruction
│   ├── crypto/
│   │   ├── identity.rs      # LocalIdentity keypair + fingerprint
│   │   ├── kdf.rs           # Chain/Message/Root key derivation
│   │   ├── kem.rs           # ML-KEM + X25519 hybrid KEM
│   │   └── ratchet.rs       # Double Ratchet session
│   ├── network/
│   │   ├── p2p.rs           # Tor hidden service + TCP listener + SOCKS5 send
│   │   ├── tor_manager.rs   # Tor process management
│   │   └── traffic_morph.rs # Traffic analysis resistance
│   ├── protocol/
│   │   ├── ncp.rs           # Null Chat Protocol encapsulation
│   │   ├── discord.rs       # Discord protocol adapter (stub)
│   │   └── matrix.rs        # Matrix protocol adapter (stub)
│   ├── storage/
│   │   ├── vault.rs         # AES-256-GCM + Argon2id encrypted LMDB vault
│   │   └── secure_delete.rs # Multi-pass file wiping
│   └── ui/
│       ├── command_center.rs # Full UI controller — all views, modals, update loop
│       ├── theme.rs          # Premium Discord-style iced StyleSheet implementations
│       ├── message_view.rs   # (stub)
│       └── sidebar.rs        # (stub)
```

---

## Wire Protocol

All messages are sent as JSON over TCP through a Tor SOCKS5 proxy:

```json
{
  "kind": {"DirectMessage": null},
  "from_id": "abc123...onion",
  "from_name": "Alice",
  "target_id": "xyz789...onion",
  "body": "Hello!",
  "timestamp": 1708000000
}
```

`WireKind` variants: `DirectMessage`, `GroupMessage`, `ChannelMessage`, `FriendRequest`, `GroupInvite`, `ModerationAction`, `NicknameUpdate`, `RoleAssignment`, `Ping`

---

## Building

### Prerequisites
- Rust stable (1.75+)
- Tor installed (`apt install tor` / `brew install tor`)

```bash
git clone https://github.com/4fqr/null-chat
cd null-chat
cargo build --release
./target/release/null-chat
```

### First Run
On first launch you'll be prompted to:
1. Choose a display name
2. Create a vault passphrase (>=12 chars, mixed case + digit required)

Your passphrase is **never stored** — it derives the AES-256 key via Argon2id.

---

## Privacy Model

| Threat | Mitigation |
|--------|-----------|
| Server interception | Tor hidden services — no IP exposed |
| Metadata analysis | Traffic morphing layer |
| Local data exposure | Argon2id + AES-256-GCM encrypted vault |
| Long-term compromise | Double Ratchet forward secrecy |
| Quantum adversary | ML-KEM (Kyber) post-quantum KEM |
| Emergency | Panic engine: Ctrl+Alt+Shift+X wipes vault |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Ctrl+Alt+Shift+X` | **PANIC** — destroy all local data |

---

*Null Chat is experimental software. It has not been audited. Use at your own risk.*
