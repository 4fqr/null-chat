use std::net::SocketAddr;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

const TOR_SOCKS_ADDR: &str = "127.0.0.1:9050";
const TOR_CONTROL_ADDR: &str = "127.0.0.1:9051";

#[derive(Error, Debug)]
pub enum TorError {
    #[error("Tor SOCKS proxy unreachable: {0}")]
    SocksUnreachable(String),
    #[error("circuit build timeout")]
    CircuitTimeout,
    #[error("hidden service resolution failed: {0}")]
    ResolutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TorCircuitState {
    Uninitialized,
    Building,
    Ready,
    Failed(String),
}

pub struct TorManager {
    state: TorCircuitState,
    socks_addr: String,
}

impl TorManager {
    pub fn new() -> Self {
        Self {
            state: TorCircuitState::Uninitialized,
            socks_addr: TOR_SOCKS_ADDR.to_string(),
        }
    }

    pub async fn init(&mut self) -> Result<(), TorError> {
        self.state = TorCircuitState::Building;
        match TcpStream::connect(TOR_SOCKS_ADDR).await {
            Ok(_) => {
                self.state = TorCircuitState::Ready;
                tracing::info!("Tor SOCKS5 proxy reachable at {}", TOR_SOCKS_ADDR);
                Ok(())
            }
            Err(e) => {
                let msg = format!("{}:{}", TOR_SOCKS_ADDR, e);
                self.state = TorCircuitState::Failed(msg.clone());
                Err(TorError::SocksUnreachable(msg))
            }
        }
    }

    pub async fn connect_to_onion(
        &self,
        onion_address: &str,
        port: u16,
    ) -> Result<Socks5Stream<TcpStream>, TorError> {
        let target = format!("{}:{}", onion_address, port);
        Socks5Stream::connect(TOR_SOCKS_ADDR, target.as_str())
            .await
            .map_err(|e| TorError::SocksUnreachable(e.to_string()))
    }

    pub async fn connect_to_clearnet(
        &self,
        host: &str,
        port: u16,
    ) -> Result<Socks5Stream<TcpStream>, TorError> {
        let target = format!("{}:{}", host, port);
        Socks5Stream::connect(TOR_SOCKS_ADDR, target.as_str())
            .await
            .map_err(|e| TorError::SocksUnreachable(e.to_string()))
    }

    pub fn circuit_state(&self) -> &TorCircuitState {
        &self.state
    }

    pub fn is_ready(&self) -> bool {
        self.state == TorCircuitState::Ready
    }

    pub async fn renew_circuit(&self) -> Result<(), TorError> {
        use tokio::io::AsyncWriteExt;
        let mut ctrl = tokio::net::TcpStream::connect(TOR_CONTROL_ADDR).await?;
        ctrl.write_all(b"AUTHENTICATE \"\"\r\nSIGNAL NEWNYM\r\n").await?;
        Ok(())
    }
}

impl Default for TorManager {
    fn default() -> Self {
        Self::new()
    }
}
