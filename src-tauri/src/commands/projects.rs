use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub decoded_path: String,
    pub display_name: String,
    pub session_count: i64,
    pub last_active: Option<String>,
}

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, decoded_path, display_name, session_count, last_active
             FROM projects
             ORDER BY last_active DESC NULLS LAST",
        )
        .map_err(|e| e.to_string())?;

    let projects = stmt
        .query_map([], |row| {
            Ok(ProjectSummary {
                id: row.get(0)?,
                decoded_path: row.get(1)?,
                display_name: row.get(2)?,
                session_count: row.get(3)?,
                last_active: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(projects)
}
