use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MatrixError {
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("homeserver unreachable: {0}")]
    Unreachable(String),
    #[error("room not found: {0}")]
    RoomNotFound(String),
    #[error("encryption error: {0}")]
    Encryption(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixCredential {
    pub homeserver: String,
    pub user_id: String,
    pub access_token: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixRoom {
    pub id: String,
    pub display_name: String,
    pub canonical_alias: Option<String>,
    pub encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixEvent {
    pub event_id: String,
    pub room_id: String,
    pub sender: String,
    pub body: String,
    pub origin_server_ts: u64,
}

pub struct MatrixClient {
    credential: MatrixCredential,
    http_client: reqwest_stub::StubClient,
}

impl MatrixClient {
    pub fn new(credential: MatrixCredential) -> Self {
        Self {
            credential,
            http_client: reqwest_stub::StubClient,
        }
    }

    pub async fn login(&self) -> Result<(), MatrixError> {
        Ok(())
    }

    pub async fn sync(&self) -> Result<Vec<MatrixEvent>, MatrixError> {
        Ok(vec![])
    }

    pub async fn joined_rooms(&self) -> Result<Vec<MatrixRoom>, MatrixError> {
        Ok(vec![])
    }

    pub async fn send_text_event(
        &self,
        room_id: &str,
        body: &str,
    ) -> Result<String, MatrixError> {
        Ok(uuid::Uuid::new_v4().to_string())
    }
}

mod reqwest_stub {
    pub struct StubClient;
}
