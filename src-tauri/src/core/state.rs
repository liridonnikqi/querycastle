use std::collections::HashMap;
use std::sync::Arc;

use crate::core::error::DbError;
use crate::core::pool::Pool;
use crate::core::tunnel::SshTunnel;
use crate::core::types::{ConnectionInput, ConnectionStatus, DatabaseType};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub id: String,
    pub input: ConnectionInput,
    pub server_version: Option<String>,
    pub pool: Pool,
    pub tunnel: Option<Arc<SshTunnel>>,
}

pub struct SessionState {
    pub sessions: HashMap<String, Arc<ActiveConnection>>,
    pub active_id: Option<String>,
}

pub struct AppState {
    pub inner: RwLock<SessionState>,
    pub running_queries: RwLock<HashMap<String, CancellationToken>>,
}

impl AppState {
    pub async fn get_active(&self) -> Option<Arc<ActiveConnection>> {
        let guard = self.inner.read().await;
        let id = guard.active_id.as_ref()?;
        guard.sessions.get(id).cloned()
    }

    pub async fn require_session(&self, session_id: &str) -> Result<Arc<ActiveConnection>, DbError> {
        let id = session_id.trim();
        if id.is_empty() {
            return Err(DbError::validation("Session id is required"));
        }
        let guard = self.inner.read().await;
        guard
            .sessions
            .get(id)
            .cloned()
            .ok_or_else(|| DbError::NotFound("Connection session not found".to_string()))
    }

    pub async fn register_query(&self, query_id: String, token: CancellationToken) {
        let id = query_id.trim();
        if id.is_empty() {
            return;
        }
        self.running_queries.write().await.insert(id.to_string(), token);
    }

    pub async fn cancel_query(&self, query_id: &str) -> Result<(), DbError> {
        let id = query_id.trim();
        if id.is_empty() {
            return Err(DbError::validation("Query id is required"));
        }
        let guard = self.running_queries.read().await;
        let Some(token) = guard.get(id) else {
            return Err(DbError::NotFound("No running query with that id".to_string()));
        };
        token.cancel();
        Ok(())
    }

    pub async fn unregister_query(&self, query_id: &str) {
        let id = query_id.trim();
        if id.is_empty() {
            return;
        }
        self.running_queries.write().await.remove(id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            inner: RwLock::new(SessionState {
                sessions: HashMap::new(),
                active_id: None,
            }),
            running_queries: RwLock::new(HashMap::new()),
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
        read_only: input.read_only,
        ssh_tunnel: input.ssh_enabled,
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
        read_only: false,
        ssh_tunnel: false,
    }
}
