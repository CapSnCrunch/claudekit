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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rusqlite::{Connection, params};
    use crate::db::schema::run_migrations;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    /// Seed a minimal DB: 1 project, 2 sessions each with some messages.
    fn seed(conn: &Connection) {
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name, session_count)
             VALUES ('p1','/p','MyProject',2)",
            [],
        ).unwrap();

        for sid in &["s1", "s2"] {
            conn.execute(
                "INSERT INTO sessions (id, project_id, title, message_count, user_message_count, created_at, updated_at)
                 VALUES (?1,'p1','T',4,2,'2025-03-10','2025-03-10')",
                [sid],
            ).unwrap();
        }

        // s1: 3 messages on 2025-03-10, 2 human prompts
        let msgs_s1 = [
            ("m1", "s1", "user",      1i64, "2025-03-10T09:00:00Z"),
            ("m2", "s1", "assistant", 0,    "2025-03-10T09:00:01Z"),
            ("m3", "s1", "user",      1,    "2025-03-10T09:00:02Z"),
        ];
        // s2: 1 human message on a different day
        let msgs_s2 = [
            ("m4", "s2", "user", 1i64, "2025-03-11T10:00:00Z"),
            ("m5", "s2", "user", 0,    "2025-03-11T10:00:01Z"), // tool result
        ];

        for (id, sid, role, is_hp, ts) in msgs_s1.iter().chain(msgs_s2.iter()) {
            conn.execute(
                "INSERT INTO messages (id, session_id, role, is_summary, is_human_prompt, content_json, timestamp, ordinal)
                 VALUES (?1,?2,?3,0,?4,'\"x\"',?5,0)",
                params![id, sid, role, is_hp, ts],
            ).unwrap();
        }
    }

    // ── get_heatmap_data SQL ──────────────────────────────────────────────────

    #[test]
    fn heatmap_counts_only_human_prompts_per_day() {
        let conn = db();
        seed(&conn);

        let mut stmt = conn.prepare(
            "SELECT DATE(timestamp) as day, COUNT(*) as count
             FROM messages
             WHERE is_human_prompt = 1
               AND DATE(timestamp) BETWEEN '2025-01-01' AND '2025-12-31'
             GROUP BY day ORDER BY day ASC"
        ).unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), 2, "two distinct days with activity");
        assert_eq!(rows[0], ("2025-03-10".to_string(), 2), "2 human msgs on 2025-03-10");
        assert_eq!(rows[1], ("2025-03-11".to_string(), 1), "1 human msg on 2025-03-11");
    }

    #[test]
    fn heatmap_excludes_days_outside_year_range() {
        let conn = db();
        seed(&conn);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages
                 WHERE is_human_prompt = 1
                   AND DATE(timestamp) BETWEEN '2024-01-01' AND '2024-12-31'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "no messages in 2024");
    }

    // ── get_day_detail SQL ────────────────────────────────────────────────────

    #[test]
    fn day_detail_total_messages_counts_human_only() {
        let conn = db();
        seed(&conn);

        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE is_human_prompt = 1 AND DATE(timestamp) = '2025-03-10'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total, 2);
    }

    #[test]
    fn day_detail_sessions_grouped_by_session() {
        let conn = db();
        seed(&conn);

        let mut stmt = conn.prepare(
            "SELECT s.id, p.display_name, s.title, COUNT(m.id) as msg_count
             FROM messages m
             JOIN sessions s ON m.session_id = s.id
             JOIN projects p ON s.project_id = p.id
             WHERE m.is_human_prompt = 1 AND DATE(m.timestamp) = '2025-03-10'
             GROUP BY s.id ORDER BY msg_count DESC"
        ).unwrap();
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(3)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), 1, "only s1 has messages on 2025-03-10");
        assert_eq!(rows[0], ("s1".to_string(), 2));
    }

    #[test]
    fn day_detail_returns_empty_for_date_with_no_activity() {
        let conn = db();
        seed(&conn);

        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE is_human_prompt = 1 AND DATE(timestamp) = '2025-01-01'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(total, 0);
    }

    // ── get_dashboard_stats SQL ───────────────────────────────────────────────

    #[test]
    fn dashboard_stats_counts_sessions_and_projects() {
        let conn = db();
        seed(&conn);

        let sessions: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
            .unwrap();
        let projects: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sessions, 2);
        assert_eq!(projects, 1);
    }

    #[test]
    fn dashboard_stats_this_week_query() {
        let conn = db();
        // Insert a session created today
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('px','/x','X')", []
        ).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, project_id, title, message_count, created_at, updated_at)
             VALUES ('sx','px','T',0,datetime('now'),datetime('now'))",
            [],
        ).unwrap();

        let this_week: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE created_at >= datetime('now', '-7 days')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(this_week >= 1, "the just-inserted session must count as this week");
    }

    #[test]
    fn dashboard_stats_most_active_project_by_session_count() {
        let conn = db();
        seed(&conn);
        // p1 has session_count=2 in the seed; add a second project with 0
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name, session_count) VALUES ('p2','/q','Q',0)",
            [],
        ).unwrap();

        let top_id: String = conn
            .query_row(
                "SELECT id FROM projects ORDER BY session_count DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(top_id, "p1");
    }
}
