use std::collections::HashMap;

use crate::core::error::DbError;
use crate::core::pool::Pool;
use crate::core::types::{ConnectionInput, ConnectionStatus, DatabaseType};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub id: String,
    pub input: ConnectionInput,
    pub server_version: Option<String>,
    pub pool: Pool,
}

pub struct SessionState {
    pub sessions: HashMap<String, ActiveConnection>,
    pub active_id: Option<String>,
}

pub struct AppState {
    pub inner: RwLock<SessionState>,
}

impl AppState {
    pub async fn get_active(&self) -> Option<ActiveConnection> {
        let guard = self.inner.read().await;
        let id = guard.active_id.as_ref()?;
        guard.sessions.get(id).cloned()
    }

    pub async fn require_active(&self) -> Result<ActiveConnection, DbError> {
        self.get_active()
            .await
            .ok_or_else(|| DbError::connection("No active database connection"))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: RwLock::new(SessionState {
                sessions: HashMap::new(),
                active_id: None,
            }),
        }
    }
}

pub fn new_session_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn status_from_input(
    input: &ConnectionInput,
    version: Option<String>,
    session_id: String,
) -> ConnectionStatus {
    ConnectionStatus {
        connected: true,
        database_type: input.database_type,
        name: input.name.clone(),
        host: input.host.clone(),
        port: input.port,
        database: input.database.clone(),
        user: input.user.clone(),
        server_version: version,
        session_id,
    }
}

pub fn status_from_active(active: &ActiveConnection) -> ConnectionStatus {
    status_from_input(&active.input, active.server_version.clone(), active.id.clone())
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
        session_id: String::new(),
    }
}
