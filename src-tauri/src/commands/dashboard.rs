use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::projects::ProjectSummary;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_sessions: i64,
    pub total_projects: i64,
    pub sessions_this_week: i64,
    pub sessions_last_week: i64,
    pub most_active_project: Option<ProjectSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDay {
    pub date: String, // "YYYY-MM-DD"
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySession {
    pub session_id: String,
    pub project_name: String,
    pub title: Option<String>,
    pub user_message_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayDetail {
    pub date: String,
    pub total_messages: i64,
    pub sessions: Vec<DaySession>,
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<'_, AppState>) -> Result<DashboardStats, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let total_sessions: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
        .unwrap_or(0);

    let total_projects: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
        .unwrap_or(0);

    let sessions_this_week: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sessions
             WHERE created_at >= datetime('now', '-7 days')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let sessions_last_week: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sessions
             WHERE created_at >= datetime('now', '-14 days')
               AND created_at <  datetime('now', '-7 days')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let most_active_project = conn
        .query_row(
            "SELECT id, decoded_path, display_name, session_count, last_active
             FROM projects
             ORDER BY session_count DESC
             LIMIT 1",
            [],
            |row| {
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    decoded_path: row.get(1)?,
                    display_name: row.get(2)?,
                    session_count: row.get(3)?,
                    last_active: row.get(4)?,
                })
            },
        )
        .ok();

    Ok(DashboardStats {
        total_sessions,
        total_projects,
        sessions_this_week,
        sessions_last_week,
        most_active_project,
    })
}

#[tauri::command]
pub fn get_heatmap_data(
    state: State<'_, AppState>,
    year: Option<i32>,
) -> Result<Vec<HeatmapDay>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let target_year = year.unwrap_or_else(|| {
        chrono::Local::now().format("%Y").to_string().parse().unwrap_or(2025)
    });

    let start = format!("{}-01-01", target_year);
    let end = format!("{}-12-31", target_year);

    let mut stmt = conn
        .prepare(
            "SELECT DATE(timestamp) as day, COUNT(*) as count
             FROM messages
             WHERE is_human_prompt = 1
               AND DATE(timestamp) BETWEEN ?1 AND ?2
             GROUP BY day
             ORDER BY day ASC",
        )
        .map_err(|e| e.to_string())?;

    let days = stmt
        .query_map(params![start, end], |row| {
            Ok(HeatmapDay {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(days)
}

#[tauri::command]
pub fn get_day_detail(
    state: State<'_, AppState>,
    date: String,
) -> Result<DayDetail, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let total_messages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages
             WHERE is_human_prompt = 1 AND DATE(timestamp) = ?1",
            params![date],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut stmt = conn
        .prepare(
            "SELECT s.id, p.display_name, s.title, COUNT(m.id) as msg_count
             FROM messages m
             JOIN sessions s ON m.session_id = s.id
             JOIN projects p ON s.project_id = p.id
             WHERE m.is_human_prompt = 1
               AND DATE(m.timestamp) = ?1
             GROUP BY s.id
             ORDER BY msg_count DESC",
        )
        .map_err(|e| e.to_string())?;

    let sessions = stmt
        .query_map(params![date], |row| {
            Ok(DaySession {
                session_id: row.get(0)?,
                project_name: row.get(1)?,
                title: row.get(2)?,
                user_message_count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(DayDetail {
        date,
        total_messages,
        sessions,
    })
}
