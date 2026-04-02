use crate::core::types::{ConnectionInput, ConnectionStatus, DatabaseType};
use tokio::sync::Mutex;

pub struct AppState {
    pub connection: Mutex<Option<ConnectionInput>>,
    pub server_version: Mutex<Option<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection: Mutex::new(None),
            server_version: Mutex::new(None),
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
