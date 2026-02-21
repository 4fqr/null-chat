use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_socks::tcp::Socks5Stream;

use crate::model::WireMessage;

/// Port our hidden service listens on (and forwards to locally)
pub const P2P_PORT: u16 = 17777;
/// The system Tor SOCKS5 proxy (standard install)
pub const TOR_SOCKS_SYSTEM: &str = "127.0.0.1:9050";
/// Our bundled Tor SOCKS5 proxy (custom instance)
pub const TOR_SOCKS_LOCAL: &str = "127.0.0.1:9150";

#[derive(Debug, Clone, PartialEq)]
pub enum P2PStatus {
    Offline,
    TorConnecting,
    TorReady { onion: String },
    DirectMode, // No Tor — local testing only
    Error(String),
}

impl P2PStatus {
    pub fn label(&self) -> &'static str {
        match self {
            P2PStatus::Offline => "Offline",
            P2PStatus::TorConnecting => "Connecting to Tor...",
            P2PStatus::TorReady { .. } => "Tor Connected",
            P2PStatus::DirectMode => "Direct (no Tor)",
            P2PStatus::Error(_) => "Tor Error",
        }
    }
    pub fn is_ready(&self) -> bool {
        matches!(self, P2PStatus::TorReady { .. } | P2PStatus::DirectMode)
    }
}

/// Check if system Tor SOCKS5 is reachable
pub async fn probe_system_tor() -> bool {
    TcpStream::connect(TOR_SOCKS_SYSTEM)
        .await
        .is_ok()
}

/// Spawn a Tor process with hidden service configured.
/// Returns (process_handle, onion_address).
/// Falls back gracefully if `tor` is not installed.
pub async fn start_hidden_service(
    data_dir: &PathBuf,
) -> anyhow::Result<(tokio::process::Child, String)> {
    use tokio::fs;

    let tor_dir = data_dir.join("tor");
    let hs_dir = tor_dir.join("hs");
    fs::create_dir_all(&hs_dir).await?;

    // Set proper permissions on hidden service dir (Tor requires 0700)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hs_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let torrc = format!(
        concat!(
            "DataDirectory {}\n",
            "HiddenServiceDir {}\n",
            "HiddenServicePort {} 127.0.0.1:{}\n",
            "SocksPort 9150\n",
            "ControlPort 9151\n",
            "Log notice stderr\n",
            "ExitPolicy reject *:*\n",
        ),
        tor_dir.display(),
        hs_dir.display(),
        P2P_PORT,
        P2P_PORT,
    );

    fs::write(tor_dir.join("torrc"), torrc.as_bytes()).await?;

    let child = tokio::process::Command::new("tor")
        .arg("-f")
        .arg(tor_dir.join("torrc"))
        .kill_on_drop(true)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn tor: {}. Is tor installed?", e))?;

    // Wait up to 60 seconds for the hidden service hostname file
    let hostname_path = hs_dir.join("hostname");
    for _ in 0..120 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if hostname_path.exists() {
            let onion = fs::read_to_string(&hostname_path)
                .await?
                .trim()
                .to_string();
            tracing::info!("Hidden service ready at {}", onion);
            return Ok((child, onion));
        }
    }

    Err(anyhow::anyhow!(
        "Tor hidden service startup timed out after 60s"
    ))
}

/// Start a TCP listener for incoming P2P messages.
/// Spawns a background Tokio task.
pub async fn start_listener(
    incoming: Arc<Mutex<Vec<WireMessage>>>,
) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{}", P2P_PORT);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Cannot bind P2P listener {}: {}", addr, e))?;

    tracing::info!("P2P listener bound on {}", addr);

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, peer)) => {
                    tracing::debug!("Incoming P2P connection from {}", peer);
                    let queue = incoming.clone();
                    tokio::spawn(async move {
                        let mut buf = Vec::with_capacity(8192);
                        let mut tmp = [0u8; 4096];
                        let deadline = tokio::time::Instant::now()
                            + tokio::time::Duration::from_secs(30);

                        loop {
                            let remaining = deadline.saturating_duration_since(
                                tokio::time::Instant::now(),
                            );
                            if remaining.is_zero() {
                                break;
                            }
                            match tokio::time::timeout(remaining, stream.read(&mut tmp))
                                .await
                            {
                                Ok(Ok(0)) | Ok(Err(_)) | Err(_) => break,
                                Ok(Ok(n)) => {
                                    buf.extend_from_slice(&tmp[..n]);
                                    // Messages are newline-delimited
                                    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                                        if let Ok(msg) = serde_json::from_slice::<WireMessage>(
                                            &buf[..pos],
                                        ) {
                                            tracing::info!(
                                                "Received P2P message from {}",
                                                msg.from_id
                                            );
                                            queue.lock().await.push(msg);
                                        }
                                        break;
                                    }
                                    if buf.len() > 1_048_576 {
                                        // 1 MiB max — drop oversized
                                        break;
                                    }
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("P2P accept error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    });

    Ok(())
}

/// Send a message to a peer via Tor SOCKS5 or direct TCP.
pub async fn send_to_peer(
    peer_id: &str,
    msg: &WireMessage,
    tor_socks: Option<&str>,
) -> anyhow::Result<()> {
    let target = format!("{}:{}", peer_id, P2P_PORT);
    let mut payload = serde_json::to_vec(msg)?;
    payload.push(b'\n');

    match tor_socks {
        Some(socks_addr) => {
            let mut stream =
                Socks5Stream::connect(socks_addr, target.as_str())
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Tor SOCKS5 connect to {} failed: {}", target, e)
                    })?;
            stream.write_all(&payload).await?;
        }
        None => {
            // Direct TCP fallback (local testing / same LAN)
            let mut stream = TcpStream::connect(&target)
                .await
                .map_err(|e| anyhow::anyhow!("Direct TCP to {} failed: {}", target, e))?;
            stream.write_all(&payload).await?;
        }
    }

    tracing::info!("Sent P2P message to {}", peer_id);
    Ok(())
}

/// Initialise P2P: try to start our own hidden service, fall back to
/// system Tor, fall back to direct mode.
pub async fn init_p2p(
    data_dir: PathBuf,
    incoming: Arc<Mutex<Vec<WireMessage>>>,
) -> (P2PStatus, Option<String> /* socks_addr */) {
    // Try to start our bundled Tor instance with hidden service
    match start_hidden_service(&data_dir).await {
        Ok((_child, onion)) => {
            if start_listener(incoming.clone()).await.is_ok() {
                return (
                    P2PStatus::TorReady { onion: onion.clone() },
                    Some(TOR_SOCKS_LOCAL.to_string()),
                );
            }
        }
        Err(e) => {
            tracing::warn!("Failed to start own Tor instance: {}. Trying system Tor.", e);
        }
    }

    // Try system Tor (already running)
    if probe_system_tor().await {
        tracing::info!("Using system Tor at {}", TOR_SOCKS_SYSTEM);
        if start_listener(incoming.clone()).await.is_ok() {
            return (
                P2PStatus::TorReady {
                    onion: String::from("(system Tor — no hidden service)"),
                },
                Some(TOR_SOCKS_SYSTEM.to_string()),
            );
        }
    }

    // Fall back to direct mode (no Tor — still encrypted messages, just no anonymity)
    tracing::warn!("Tor unavailable. Operating in direct mode (no anonymity).");
    if start_listener(incoming).await.is_ok() {
        return (P2PStatus::DirectMode, None);
    }

    (
        P2PStatus::Error("P2P listener failed to bind".to_string()),
        None,
    )
}
