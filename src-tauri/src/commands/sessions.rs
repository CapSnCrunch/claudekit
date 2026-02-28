use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub message_count: i64,
    pub user_message_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub is_summary: bool,
    pub content_json: String,
    pub timestamp: String,
    pub ordinal: i64,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
    project_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<SessionSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(200);
    let offset = offset.unwrap_or(0);

    let mut stmt = conn
        .prepare(
            "SELECT id, project_id, title, message_count, user_message_count, created_at, updated_at
             FROM sessions
             WHERE project_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
        )
        .map_err(|e| e.to_string())?;

    let sessions = stmt
        .query_map(params![project_id, limit, offset], |row| {
            Ok(SessionSummary {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                message_count: row.get(3)?,
                user_message_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

#[tauri::command]
pub fn get_session_messages(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<MessageRecord>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, is_summary, content_json,
                    timestamp, ordinal, model, input_tokens, output_tokens
             FROM messages
             WHERE session_id = ?1
             ORDER BY ordinal ASC",
        )
        .map_err(|e| e.to_string())?;

    let messages = stmt
        .query_map(params![session_id], |row| {
            let is_summary: i64 = row.get(3)?;
            Ok(MessageRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                is_summary: is_summary != 0,
                content_json: row.get(4)?,
                timestamp: row.get(5)?,
                ordinal: row.get(6)?,
                model: row.get(7)?,
                input_tokens: row.get(8)?,
                output_tokens: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub title: Option<String>,
    pub project_id: String,
    pub project_decoded_path: String,
}

#[tauri::command]
pub fn get_session_info(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<SessionInfo, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT s.id, s.title, s.project_id, p.decoded_path
         FROM sessions s
         JOIN projects p ON p.id = s.project_id
         WHERE s.id = ?1",
        rusqlite::params![session_id],
        |row| {
            Ok(SessionInfo {
                session_id: row.get(0)?,
                title: row.get(1)?,
                project_id: row.get(2)?,
                project_decoded_path: row.get(3)?,
            })
        },
    )
    .map_err(|e| e.to_string())
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

    fn seed(conn: &Connection) {
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('p1', '/p', 'P')",
            [],
        ).unwrap();
        for (i, (sid, ts, umc)) in [
            ("s1", "2025-03-01", 5i64),
            ("s2", "2025-04-01", 2i64),
            ("s3", "2025-05-01", 0i64),
        ].iter().enumerate() {
            conn.execute(
                "INSERT INTO sessions (id, project_id, title, message_count, user_message_count, created_at, updated_at)
                 VALUES (?1,'p1','Title',10,?2,?3,?3)",
                params![sid, umc, ts],
            ).unwrap();
            // Insert two messages per session
            for m in 0..2u64 {
                conn.execute(
                    "INSERT INTO messages (id, session_id, role, is_summary, is_human_prompt, content_json, timestamp, ordinal)
                     VALUES (?1,?2,'user',0,1,'\"hi\"','2025-01-01T00:00:00Z',?3)",
                    params![format!("m{i}{m}"), sid, m as i64],
                ).unwrap();
            }
        }
    }

    // ── list_sessions SQL ─────────────────────────────────────────────────────

    #[test]
    fn list_sessions_returns_sessions_for_project() {
        let conn = db();
        seed(&conn);
        let mut stmt = conn.prepare(
            "SELECT id FROM sessions WHERE project_id = 'p1' ORDER BY created_at DESC LIMIT 200 OFFSET 0"
        ).unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert_eq!(ids, vec!["s3", "s2", "s1"], "should be newest-first");
    }

    #[test]
    fn list_sessions_includes_user_message_count() {
        let conn = db();
        seed(&conn);
        let umc: i64 = conn
            .query_row("SELECT user_message_count FROM sessions WHERE id = 's1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(umc, 5);
    }

    #[test]
    fn list_sessions_pagination_limit_and_offset() {
        let conn = db();
        seed(&conn);
        let mut stmt = conn.prepare(
            "SELECT id FROM sessions WHERE project_id = 'p1' ORDER BY created_at DESC LIMIT 1 OFFSET 1"
        ).unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert_eq!(ids, vec!["s2"], "offset=1, limit=1 should return second-newest");
    }

    #[test]
    fn list_sessions_returns_empty_for_unknown_project() {
        let conn = db();
        seed(&conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions WHERE project_id = 'nope'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    // ── get_session_messages SQL ──────────────────────────────────────────────

    #[test]
    fn get_session_messages_returns_all_messages_ordered_by_ordinal() {
        let conn = db();
        seed(&conn);
        let mut stmt = conn.prepare(
            "SELECT id FROM messages WHERE session_id = 's1' ORDER BY ordinal ASC"
        ).unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert_eq!(ids, vec!["m00", "m01"]);
    }

    #[test]
    fn get_session_messages_empty_for_unknown_session() {
        let conn = db();
        seed(&conn);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages WHERE session_id = 'nope'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
