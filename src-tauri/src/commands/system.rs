use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSqlFilePayload {
    pub path: String,
    pub content: String,
}

fn find_launch_sql_path() -> Option<PathBuf> {
    std::env::args_os().skip(1).find_map(|arg| {
        let path = PathBuf::from(arg);
        let is_sql = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("sql"))
            .unwrap_or(false);
        if is_sql && path.exists() {
            Some(path)
        } else {
            None
        }
    })
}

#[tauri::command]
pub fn get_launch_sql_file() -> Result<Option<LaunchSqlFilePayload>, String> {
    let Some(path) = find_launch_sql_path() else {
        return Ok(None);
    };

    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read SQL file '{}': {error}", path.to_string_lossy()))?;

    Ok(Some(LaunchSqlFilePayload {
        path: path.to_string_lossy().to_string(),
        content,
    }))
}
