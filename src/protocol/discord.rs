use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_socks::tcp::Socks5Stream;
use tokio::net::TcpStream;

#[derive(Error, Debug)]
pub enum DiscordError {
    #[error("authentication failed")]
    AuthFailed,
    #[error("gateway connection failed: {0}")]
    GatewayFailed(String),
    #[error("rate limit exceeded")]
    RateLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordCredential {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannel {
    pub id: String,
    pub name: String,
    pub guild_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    pub author_id: String,
    pub content: String,
    pub timestamp: String,
}

pub struct DiscordGateway {
    token: secrecy::SecretString,
    stream: Option<Socks5Stream<TcpStream>>,
}

impl DiscordGateway {
    pub fn new(token: String) -> Self {
        Self {
            token: secrecy::SecretString::new(token),
            stream: None,
        }
    }

    pub async fn connect_via_tor(&mut self, tor_socks: &str) -> Result<(), DiscordError> {
        let stream = Socks5Stream::connect(tor_socks, "gateway.discord.gg:443")
            .await
            .map_err(|e| DiscordError::GatewayFailed(e.to_string()))?;
        self.stream = Some(stream);
        tracing::info!("Discord gateway connected via Tor");
        Ok(())
    }

    pub async fn fetch_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannel>, DiscordError> {
        Ok(vec![])
    }

    pub async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<DiscordMessage, DiscordError> {
        let msg = DiscordMessage {
            id: uuid::Uuid::new_v4().to_string(),
            channel_id: channel_id.to_string(),
            author_id: String::from("local"),
            content: content.to_string(),
            timestamp: chrono_stub(),
        };
        Ok(msg)
    }
}

fn chrono_stub() -> String {
    String::from("1970-01-01T00:00:00Z")
}
