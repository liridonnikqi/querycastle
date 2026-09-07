use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::core::error::{DbError, StructuredDbError};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSqlFilePayload {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSqliteFilePayload {
    pub path: String,
}

fn find_launch_path_by_extensions(extensions: &[&str]) -> Option<PathBuf> {
    std::env::args_os().skip(1).find_map(|arg| {
        let path = PathBuf::from(arg);
        let is_sql = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| extensions.iter().any(|candidate| ext.eq_ignore_ascii_case(candidate)))
            .unwrap_or(false);
        if is_sql && path.exists() {
            Some(path)
        } else {
            None
        }
    })
}

static LAUNCH_SQL_CONSUMED: AtomicBool = AtomicBool::new(false);
static LAUNCH_SQLITE_CONSUMED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn get_launch_sql_file() -> Result<Option<LaunchSqlFilePayload>, StructuredDbError> {
    if LAUNCH_SQL_CONSUMED.swap(true, Ordering::SeqCst) {
        return Ok(None);
    }

    let Some(path) = find_launch_path_by_extensions(&["sql"]) else {
        return Ok(None);
    };

    let content = std::fs::read_to_string(&path).map_err(|error| {
        DbError::internal(format!(
            "Failed to read SQL file '{}': {error}",
            path.to_string_lossy()
        ))
    })?;

    Ok(Some(LaunchSqlFilePayload {
        path: path.to_string_lossy().to_string(),
        content,
    }))
}

#[tauri::command]
pub fn get_launch_sqlite_file() -> Result<Option<LaunchSqliteFilePayload>, StructuredDbError> {
    if LAUNCH_SQLITE_CONSUMED.swap(true, Ordering::SeqCst) {
        return Ok(None);
    }

    let Some(path) = find_launch_path_by_extensions(&["db", "sqlite", "sqlite3"]) else {
        return Ok(None);
    };

    Ok(Some(LaunchSqliteFilePayload {
        path: path.to_string_lossy().to_string(),
    }))
}
