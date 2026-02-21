# Null Chat

A sovereign, post-quantum secure messenger for Linux. Null Chat operates exclusively over Tor Onion Services, stores all data in a locally-encrypted vault, and serves as a hardened protocol gateway to legacy networks including Discord and Matrix.

---

## Architecture

The application is organized into five independent subsystems with strict module boundaries:

| Module | Responsibility |
|---|---|
| `crypto/` | Key generation, hybrid KEM, Double Ratchet, KDF |
| `network/` | Tor SOCKS5 management, traffic morphing |
| `protocol/` | NCP session framing, Discord gateway, Matrix client |
| `storage/` | AES-256-GCM encrypted vault (LMDB), secure deletion |
| `ui/` | Iced-based command center, vault unlock, status display |

There is no shared mutable global state. Each subsystem communicates through typed message passing. The UI is a pure function of application state — it holds no secrets.

---

## Null Cryptographic Protocol (NCP)

### Key Exchange

Session establishment uses a hybrid KEM combining X25519 and Kyber-1024. A compromised classical channel does not compromise the post-quantum security of the Kyber component, and vice versa. The shared secret is the SHA-3 hash of the concatenated X25519 and Kyber shared secrets, domain-separated with the label `NCP-HYBRID-KEM-v1`.

```
shared_secret = SHA3-256("NCP-HYBRID-KEM-v1" || X25519_ss || Kyber1024_ss)
```

### Double Ratchet

Forward secrecy is enforced by the Double Ratchet Algorithm. Every message derives a unique symmetric key that is discarded after use. The root KDF chain uses HKDF-SHA3-256 with the domain label `NCP-RATCHET-v1` and outputs 96 bytes split into: new root key (32), new chain key (32), header key (32).

Message keys are derived from the chain key via HMAC-SHA3-256:
- `message_key = HMAC-SHA3-256(chain_key, 0x01)`
- `next_chain_key = HMAC-SHA3-256(chain_key, 0x02)`

Per-message AEAD keys are derived from the message key via HKDF-SHA3-256 with the labels `NCP-ENC-KEY` (32 bytes) and `NCP-NONCE` (12 bytes).

### Symmetric Encryption

ChaCha20-Poly1305 with per-message key and nonce derived from the Double Ratchet. Associated data includes the ratchet header fields (DH public key, previous chain length, message number) to prevent header tampering without requiring header encryption in this revision.

### Identity

Users are identified by the SHA-3 (Keccak-256) hash of their Ed25519 verifying key. There are no usernames, handles, or server-assigned identifiers. Contact verification requires manual comparison of full 256-bit safety numbers out-of-band.

---

## Storage

### Encrypted Vault

All persistent data — messages, sessions, credentials, identity keys — resides in a single LMDB database directory encrypted with AES-256-GCM. The encryption key is derived from a passphrase using Argon2id with the following parameters:

| Parameter | Value |
|---|---|
| Memory | 64 MiB |
| Iterations | 3 |
| Parallelism | 4 |
| Output | 256-bit key |

A random 256-bit salt is stored alongside the database. If a TPM is present, the salt should be sealed to it; the current implementation stores the salt in a plaintext file adjacent to the vault. TPM sealing is planned for a future revision.

Each individual record is encrypted independently before writing to LMDB. The per-record nonce is randomly generated and prepended to the ciphertext.

### Secure Deletion

`SecureDelete::wipe_file` performs a 7-pass overwrite before unlinking the file: three DoD 5220.22-M passes (0x00, 0xFF, 0x00) followed by three random passes followed by a final zero pass. This is a best-effort implementation — journaling filesystems and SSDs with wear leveling may not honor sequential writes to the same logical block.

---

## Network

### Tor Integration

The `TorManager` connects to a running Tor SOCKS5 proxy at `127.0.0.1:9050`. All outbound connections — sovereign NCP and legacy gateway traffic — are routed through this proxy. Circuit renewal is available via the control port at `127.0.0.1:9051` using the `SIGNAL NEWNYM` command.

The application does not bundle a Tor binary. Tor must be installed and running on the host before Null Chat is started.

### Traffic Morphing

`TrafficMorpher` pads outbound packets to one of three uniform sizes (1 KiB, 5 KiB, 10 KiB) and introduces a randomized send delay of 50–500 ms per message. This mitigates traffic correlation attacks based on packet size or inter-arrival timing at the cost of increased latency.

---

## Build Instructions

### Prerequisites

All platforms require a stable Rust toolchain (1.75 or later) and a running Tor daemon.

#### Fedora

```sh
sudo dnf install tor rust cargo
sudo systemctl enable --now tor
git clone https://github.com/4fqr/null-chat
cd null-chat
cargo build --release
```

#### Arch Linux

```sh
sudo pacman -S tor rust
sudo systemctl enable --now tor
git clone https://github.com/4fqr/null-chat
cd null-chat
cargo build --release
```

#### Ubuntu / Debian

```sh
sudo apt install tor build-essential curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
sudo systemctl enable --now tor
git clone https://github.com/4fqr/null-chat
cd null-chat
cargo build --release
```

The compiled binary is at `target/release/null-chat`. No installation step is required; the binary is self-contained.

---

## Configuration

On first launch, Null Chat generates a fresh Ed25519 identity keypair. The identity is stored in the encrypted vault after the passphrase is set. There is no configuration file. All user-adjustable parameters are set at vault creation time (passphrase strength, vault path).

The vault is created in `$HOME/.local/share/null-chat/vault/` by default. The KDF salt is written to `.vault_kdf_params` within that directory.

---

## Threat Model

### Protected Against

- **Passive network surveillance**: All traffic exits through Tor Onion Services. An observer watching the network link sees only uniformly-sized, randomly-timed Tor traffic.
- **Forward secrecy compromise**: A stolen session key compromises at most one message. The Double Ratchet guarantees that neither past nor future messages are recoverable from a single session key.
- **Harvest-and-decrypt (quantum)**: The hybrid X25519 + Kyber-1024 key exchange ensures that an adversary storing ciphertexts today cannot decrypt them with a future quantum computer, as long as Kyber-1024 holds.
- **Physical disk seizure**: AES-256-GCM with Argon2id key derivation protects vault contents at rest.
- **Forensic file recovery**: Secure deletion makes reasonable efforts to overwrite deleted content at the filesystem layer.

### Not Protected Against

- **Endpoint compromise**: A compromised OS or kernel module can read plaintext from process memory regardless of application-layer protections.
- **Metadata correlation at Tor exit**: NCP traffic uses Onion Services and has no exit node. However, traffic timing correlation at the Tor network level remains a theoretical attack by a global passive adversary.
- **SSD wear-leveling bypass**: Secure deletion is unreliable on flash storage. Full-disk encryption at the OS layer (LUKS) should be used in conjunction with this application.
- **Passphrase brute-force with weak passphrases**: Argon2id raises the cost of brute-force but does not make it impossible. Use a passphrase of sufficient entropy.
- **Malicious Tor binary**: The application connects to a system-provided Tor daemon. The integrity of the Tor binary is outside this application's control.

---

## Panic Button

`Ctrl+Alt+Shift+X` triggers an immediate secure shutdown:

1. `SIGKILL` is sent to all `tor` processes.
2. Two 128 KiB stack frames are filled with random bytes then zeroed via `zeroize`.
3. A 2 MiB heap region is allocated, filled with random bytes, then zeroed.
4. `std::process::exit(0)` terminates the process.

Note: `exit()` does not invoke Rust destructors. Key material held in heap-allocated `Zeroizing<>` wrappers will not be explicitly zeroed on exit. The OS reclaims all process memory immediately and Linux will zero pages before reassigning them to another process, but forensic recovery from physical memory remains theoretically possible until those pages are reused.

---

## Development

```sh
git clone https://github.com/4fqr/null-chat
cd null-chat
cargo build
cargo test
```

The codebase contains no inline comments. Variable names, type names, and function signatures are expected to be self-explanatory. If additional context is needed to understand a function, that function should be refactored rather than annotated.

`rustfmt` is the canonical formatter. Run `cargo fmt` before submitting changes.

There are no external test fixtures. Unit tests live in the same file as the code they test, guarded by `#[cfg(test)]`. Integration tests go in `tests/`.

### Adding a Protocol Gateway

1. Create `src/protocol/your_protocol.rs`.
2. Define a `YourError`, `YourCredential`, and a client struct.
3. Route all outbound connections through the `TorManager` SOCKS5 proxy.
4. Add the module to `src/protocol/mod.rs`.
5. Add a workspace entry to `CommandCenter::new()`.

---

## License

MIT License. See `LICENSE` for full terms.
