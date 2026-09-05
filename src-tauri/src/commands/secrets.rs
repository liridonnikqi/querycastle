use serde::Deserialize;

use crate::core::error::{DbError, StructuredDbError};

const SERVICE: &str = "querycastle";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretNameParams {
    pub connection_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretSetParams {
    pub connection_name: String,
    pub password: String,
}

fn normalize_name(name: &str) -> Result<String, DbError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        Err(DbError::validation("Connection name is required"))
    } else {
        Ok(name)
    }
}

fn map_keyring(err: keyring::Error) -> DbError {
    DbError::internal(format!("Keychain error: {err}"))
}

#[tauri::command]
pub async fn secret_set(params: SecretSetParams) -> Result<(), StructuredDbError> {
    let name = normalize_name(&params.connection_name).map_err(StructuredDbError::from)?;
    let password = params.password;
    tokio::task::spawn_blocking(move || {
        let entry = keyring::Entry::new(SERVICE, &name).map_err(map_keyring)?;
        entry.set_password(&password).map_err(map_keyring)
    })
    .await
    .map_err(|e| StructuredDbError::from(DbError::internal(e.to_string())))?
    .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn secret_get(params: SecretNameParams) -> Result<Option<String>, StructuredDbError> {
    let name = normalize_name(&params.connection_name).map_err(StructuredDbError::from)?;
    tokio::task::spawn_blocking(move || {
        let entry = keyring::Entry::new(SERVICE, &name).map_err(map_keyring)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(map_keyring(err)),
        }
    })
    .await
    .map_err(|e| StructuredDbError::from(DbError::internal(e.to_string())))?
    .map_err(StructuredDbError::from)
}

#[tauri::command]
pub async fn secret_delete(params: SecretNameParams) -> Result<(), StructuredDbError> {
    let name = normalize_name(&params.connection_name).map_err(StructuredDbError::from)?;
    tokio::task::spawn_blocking(move || {
        let entry = keyring::Entry::new(SERVICE, &name).map_err(map_keyring)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(map_keyring(err)),
        }
    })
    .await
    .map_err(|e| StructuredDbError::from(DbError::internal(e.to_string())))?
    .map_err(StructuredDbError::from)
}
