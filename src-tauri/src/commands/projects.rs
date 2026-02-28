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

    fn insert_project(conn: &Connection, id: &str, display: &str, sessions: i64, last_active: Option<&str>) {
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name, session_count, last_active)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, format!("/{id}"), display, sessions, last_active],
        ).unwrap();
    }

    #[test]
    fn list_projects_returns_all_rows() {
        let conn = db();
        insert_project(&conn, "p1", "Alpha", 3, Some("2025-06-01"));
        insert_project(&conn, "p2", "Beta",  1, Some("2025-05-01"));

        let mut stmt = conn.prepare(
            "SELECT id, decoded_path, display_name, session_count, last_active
             FROM projects ORDER BY last_active DESC NULLS LAST"
        ).unwrap();
        let projects: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0)).unwrap()
            .filter_map(|r| r.ok()).collect();

        assert_eq!(projects, vec!["p1", "p2"], "should be ordered by last_active desc");
    }

    #[test]
    fn list_projects_empty_table_returns_empty_vec() {
        let conn = db();
        let mut stmt = conn.prepare(
            "SELECT id FROM projects ORDER BY last_active DESC NULLS LAST"
        ).unwrap();
        let projects: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert!(projects.is_empty());
    }

    #[test]
    fn list_projects_null_last_active_sorts_last() {
        let conn = db();
        insert_project(&conn, "p1", "HasDate",  0, Some("2025-01-01"));
        insert_project(&conn, "p2", "NoDate",   0, None);

        let mut stmt = conn.prepare(
            "SELECT id FROM projects ORDER BY last_active DESC NULLS LAST"
        ).unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert_eq!(ids[0], "p1", "project with date should come first");
        assert_eq!(ids[1], "p2", "null last_active should sort last");
    }
}
