mod commands;
mod db;
mod indexer;
mod parser;

use std::sync::Mutex;

use commands::{
    dashboard::{get_dashboard_stats, get_heatmap_data},
    projects::list_projects,
    sessions::{get_session_messages, list_sessions},
};
use rusqlite::Connection;

pub struct AppState {
    pub db: Mutex<Connection>,
}

#[tauri::command]
fn run_index(state: tauri::State<'_, AppState>) -> Result<indexer::IndexStats, String> {
    let claude_dir = indexer::claude_dir();
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    indexer::run_full_index(&conn, &claude_dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Determine DB path: ~/.claudekit/db.sqlite
    let db_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".claudekit");
    std::fs::create_dir_all(&db_dir).expect("Failed to create ~/.claudekit");
    let db_path = db_dir.join("db.sqlite");

    let conn = db::open(&db_path).expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            db: Mutex::new(conn),
        })
        .invoke_handler(tauri::generate_handler![
            run_index,
            list_projects,
            list_sessions,
            get_session_messages,
            get_dashboard_stats,
            get_heatmap_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
