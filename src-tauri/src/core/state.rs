#![allow(dead_code)]
use crate::core::pool::Pool;
use crate::core::types::{ConnectionInput, ConnectionStatus, DatabaseType};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub input: ConnectionInput,
    pub server_version: Option<String>,
    pub pool: Pool,
}

pub struct AppState {
    pub inner: RwLock<Option<ActiveConnection>>,
}

impl AppState {
    pub async fn get_snapshot(&self) -> Option<(ConnectionInput, Option<String>)> {
        let guard = self.inner.read().await;
        guard.as_ref().map(|a| (a.input.clone(), a.server_version.clone()))
    }

    pub async fn get_active(&self) -> Option<ActiveConnection> {
        let guard = self.inner.read().await;
        guard.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: RwLock::new(None),
        }
    }
}

pub fn disconnected_status() -> ConnectionStatus {
    ConnectionStatus {
        connected: false,
        database_type: DatabaseType::Postgres,
        name: "Disconnected".to_string(),
        host: String::new(),
        port: 5432,
        database: String::new(),
        user: String::new(),
        server_version: None,
    }
}
