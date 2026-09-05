use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    #[default]
    Postgres,
    Mysql,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInput {
    #[serde(default)]
    pub database_type: DatabaseType,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub ssl: bool,
    #[serde(default)]
    pub ssl_insecure: bool,
    pub use_connection_string: Option<bool>,
    pub connection_string: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub connected: bool,
    pub database_type: DatabaseType,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub server_version: Option<String>,
    #[serde(default)]
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResultPayload {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, Value>>,
    pub row_count: usize,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResponse {
    pub ok: bool,
    pub message: String,
    pub server_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTableChangesResponse {
    pub ok: bool,
    pub updated: usize,
    pub deleted: usize,
    pub inserted: usize,
    pub updated_rows: Vec<UpdatedRowCtid>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedRowCtid {
    pub old_ctid: String,
    pub new_ctid: String,
    pub values: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseColumn {
    pub name: String,
    pub data_type: String,
    pub not_null: bool,
    pub is_primary: bool,
    #[serde(default)]
    pub has_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseForeignKey {
    pub column: String,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_column: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseIndex {
    pub name: String,
    pub columns: String,
    pub unique: bool,
    pub is_primary: bool,
    pub definition: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseTrigger {
    pub name: String,
    pub definition: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseRoutine {
    pub schema: String,
    pub name: String,
    pub kind: String,
    pub identity_args: String,
    pub language: Option<String>,
    pub return_type: Option<String>,
    pub object_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSequence {
    pub schema: String,
    pub name: String,
    pub data_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseTable {
    pub schema: String,
    pub name: String,
    pub kind: String,
    pub columns: Vec<DatabaseColumn>,
    pub foreign_keys: Vec<DatabaseForeignKey>,
    #[serde(default)]
    pub indexes: Vec<DatabaseIndex>,
    #[serde(default)]
    pub triggers: Vec<DatabaseTrigger>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseSchema {
    pub name: String,
    pub tables: Vec<DatabaseTable>,
    #[serde(default)]
    pub routines: Vec<DatabaseRoutine>,
    #[serde(default)]
    pub sequences: Vec<DatabaseSequence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseExplorer {
    pub database: String,
    pub schemas: Vec<DatabaseSchema>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectDefinitionParams {
    pub kind: String,
    pub schema: String,
    pub name: String,
    #[serde(default)]
    pub object_id: Option<String>,
    #[serde(default)]
    pub identity_args: Option<String>,
    #[serde(default)]
    pub table: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectDefinition {
    pub title: String,
    pub sql: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssl_insecure_defaults_false() {
        let json = r#"{
            "name":"n","host":"h","port":1,"user":"u","password":"p","database":"d","ssl":true
        }"#;
        let input: ConnectionInput = serde_json::from_str(json).unwrap();
        assert!(!input.ssl_insecure);
        assert!(input.ssl);
    }
}

impl DatabaseSchema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tables: Vec::new(),
            routines: Vec::new(),
            sequences: Vec::new(),
        }
    }
}

impl DatabaseTable {
    pub fn new(schema: String, name: String, kind: String) -> Self {
        Self {
            schema,
            name,
            kind,
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            triggers: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    pub sql: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectDatabaseParams {
    pub database: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyTableChangesParams {
    pub schema: String,
    pub table: String,
    pub changes: TableChangesPayload,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableChangesPayload {
    pub updates: Vec<TableUpdatePayload>,
    pub deletes: Vec<String>,
    pub inserts: Vec<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableUpdatePayload {
    pub ctid: String,
    pub values: HashMap<String, Value>,
}
