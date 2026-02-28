use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

use crate::parser::{jsonl, project as proj};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub projects_indexed: usize,
    pub sessions_indexed: usize,
    pub messages_indexed: usize,
    pub duration_ms: u64,
}

/// Walk `~/.claude/projects/` and upsert everything into SQLite.
pub fn run_full_index(conn: &Connection, claude_dir: &Path) -> Result<IndexStats, String> {
    let start = Instant::now();
    let projects_dir = claude_dir.join("projects");

    if !projects_dir.exists() {
        return Ok(IndexStats {
            projects_indexed: 0,
            sessions_indexed: 0,
            messages_indexed: 0,
            duration_ms: 0,
        });
    }

    let mut projects_indexed = 0;
    let mut sessions_indexed = 0;
    let mut messages_indexed = 0;

    // Each immediate subdirectory of projects/ is a project
    for entry in WalkDir::new(&projects_dir).min_depth(1).max_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Error reading projects dir entry: {e}");
                continue;
            }
        };
        if !entry.file_type().is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let decoded_path = proj::decode_project_path(&dir_name);
        let display = proj::display_name(&decoded_path);

        upsert_project(conn, &dir_name, &decoded_path, &display)
            .map_err(|e| format!("Failed to upsert project {dir_name}: {e}"))?;
        projects_indexed += 1;

        // Each .jsonl file in the project dir is a session
        let (s, m) = index_project_sessions(conn, &dir_name, entry.path())
            .map_err(|e| format!("Failed to index sessions in {dir_name}: {e}"))?;
        sessions_indexed += s;
        messages_indexed += m;

        // Update session_count and last_active on the project row
        conn.execute(
            "UPDATE projects SET
                session_count = (SELECT COUNT(*) FROM sessions WHERE project_id = ?1),
                last_active   = (SELECT MAX(updated_at) FROM sessions WHERE project_id = ?1)
             WHERE id = ?1",
            params![dir_name],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(IndexStats {
        projects_indexed,
        sessions_indexed,
        messages_indexed,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

fn upsert_project(
    conn: &Connection,
    id: &str,
    decoded_path: &str,
    display_name: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO projects (id, decoded_path, display_name)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
             decoded_path = excluded.decoded_path,
             display_name = excluded.display_name",
        params![id, decoded_path, display_name],
    )?;
    Ok(())
}

fn index_project_sessions(
    conn: &Connection,
    project_id: &str,
    project_path: &Path,
) -> Result<(usize, usize), String> {
    let mut sessions_indexed = 0;
    let mut messages_indexed = 0;

    for entry in WalkDir::new(project_path).min_depth(1).max_depth(1) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        if session_id.is_empty() {
            continue;
        }

        // Skip if file hasn't changed since last index
        if !needs_reindex(conn, &session_id, path) {
            continue;
        }

        let entries = match jsonl::parse_file(path) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Failed to parse {}: {}", path.display(), e);
                continue;
            }
        };

        if entries.is_empty() {
            continue;
        }

        let m = upsert_session(conn, &session_id, project_id, &entries)
            .map_err(|e| format!("upsert_session {session_id}: {e}"))?;

        sessions_indexed += 1;
        messages_indexed += m;
    }

    Ok((sessions_indexed, messages_indexed))
}

/// Returns true if the file's mtime is newer than the session's indexed_at timestamp.
fn needs_reindex(conn: &Connection, session_id: &str, path: &Path) -> bool {
    let indexed_at: Option<String> = conn
        .query_row(
            "SELECT indexed_at FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .ok();

    let Some(indexed_at) = indexed_at else {
        return true; // not yet indexed
    };

    let Ok(mtime) = path.metadata().and_then(|m| m.modified()) else {
        return true;
    };

    let Ok(mtime_dt) = mtime.duration_since(std::time::UNIX_EPOCH) else {
        return true;
    };

    // Parse indexed_at as a naive datetime and compare
    // If file mtime (seconds since epoch) > indexed_at, re-index
    let indexed_secs = chrono::NaiveDateTime::parse_from_str(&indexed_at, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|dt| dt.and_utc().timestamp() as u64)
        .unwrap_or(0);

    mtime_dt.as_secs() > indexed_secs
}

/// Upserts a session and all its messages. Returns the number of messages written.
fn upsert_session(
    conn: &Connection,
    session_id: &str,
    project_id: &str,
    entries: &[jsonl::JournalEntry],
) -> Result<usize, rusqlite::Error> {
    let non_summary: Vec<&jsonl::JournalEntry> =
        entries.iter().filter(|e| e.entry_type != "summary").collect();

    let created_at = entries.first().map(|e| e.timestamp.as_str()).unwrap_or("");
    let updated_at = entries.last().map(|e| e.timestamp.as_str()).unwrap_or("");
    let title = jsonl::extract_title(entries);
    let message_count = non_summary.len() as i64;

    // Count only entries where the human actually typed a prompt:
    // role = "user" and content contains at least one text block
    // (excludes tool_result-only messages that come back from tools)
    let user_message_count = entries.iter().filter(|e| {
        let role = e.message.as_ref()
            .and_then(|m| m.role.as_deref())
            .unwrap_or(&e.entry_type);
        if role != "user" || e.entry_type == "summary" { return false; }
        if let Some(content) = e.message.as_ref().and_then(|m| m.content.as_ref()) {
            if content.is_string() { return true; }  // plain string = human text
            if let Some(arr) = content.as_array() {
                return arr.iter().any(|block| {
                    block.get("type").and_then(|t| t.as_str()) == Some("text")
                });
            }
        }
        false
    }).count() as i64;

    let total_input: i64 = entries
        .iter()
        .filter_map(|e| e.usage.as_ref()?.input_tokens)
        .sum();
    let total_output: i64 = entries
        .iter()
        .filter_map(|e| e.usage.as_ref()?.output_tokens)
        .sum();
    let total_cost: f64 = entries.iter().filter_map(|e| e.cost_usd).sum();

    conn.execute(
        "INSERT INTO sessions
            (id, project_id, title, message_count, user_message_count,
             total_input_tokens, total_output_tokens,
             total_cost_usd, created_at, updated_at, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             title               = excluded.title,
             message_count       = excluded.message_count,
             user_message_count  = excluded.user_message_count,
             total_input_tokens  = excluded.total_input_tokens,
             total_output_tokens = excluded.total_output_tokens,
             total_cost_usd      = excluded.total_cost_usd,
             updated_at          = excluded.updated_at,
             indexed_at          = datetime('now')",
        params![
            session_id,
            project_id,
            title,
            message_count,
            user_message_count,
            total_input,
            total_output,
            total_cost,
            created_at,
            updated_at
        ],
    )?;

    // Delete existing messages for this session and re-insert
    // (simpler than diffing; sessions are append-mostly so this is fine)
    conn.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id])?;

    let mut count = 0;
    for (ordinal, entry) in entries.iter().enumerate() {
        let role = entry
            .message
            .as_ref()
            .and_then(|m| m.role.as_deref())
            .unwrap_or(&entry.entry_type);

        let content_json = entry
            .message
            .as_ref()
            .and_then(|m| m.content.as_ref())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "[]".to_string());

        let model = entry.message.as_ref().and_then(|m| m.model.as_deref());
        let input_tokens = entry.usage.as_ref().and_then(|u| u.input_tokens);
        let output_tokens = entry.usage.as_ref().and_then(|u| u.output_tokens);
        let is_summary = if entry.entry_type == "summary" { 1 } else { 0 };

        // is_human_prompt: user-role entry with at least one text content block
        let is_human_prompt = if role == "user" && is_summary == 0 {
            if let Some(msg) = &entry.message {
                if let Some(content) = &msg.content {
                    if content.is_string() {
                        1
                    } else if let Some(arr) = content.as_array() {
                        if arr.iter().any(|b| {
                            b.get("type").and_then(|t| t.as_str()) == Some("text")
                        }) { 1 } else { 0 }
                    } else { 0 }
                } else { 0 }
            } else { 0 }
        } else { 0 };

        conn.execute(
            "INSERT OR REPLACE INTO messages
                (id, session_id, parent_id, role, is_summary, is_human_prompt, content_json,
                 input_tokens, output_tokens, cost_usd, model, timestamp, ordinal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                entry.uuid,
                session_id,
                entry.parent_uuid,
                role,
                is_summary,
                is_human_prompt,
                content_json,
                input_tokens,
                output_tokens,
                entry.cost_usd,
                model,
                entry.timestamp,
                ordinal as i64,
            ],
        )?;
        count += 1;
    }

    Ok(count)
}

pub fn claude_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use crate::parser::jsonl::JournalEntry;
    use crate::db::schema::run_migrations;

    // Build a minimal JournalEntry from a JSON string (the JSONL line).
    fn entry(json: &str) -> JournalEntry {
        serde_json::from_str(json).expect("test entry must be valid JSON")
    }

    // Convenience: run migrations on an in-memory DB and return it.
    fn in_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    // ── is_human_prompt detection ─────────────────────────────────────────────

    #[test]
    fn plain_string_content_is_human_prompt() {
        let e = entry(r#"{
            "type":"user","uuid":"u1","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z",
            "message":{"role":"user","content":"hello world"}
        }"#);
        // Replicate the detection logic exactly as it appears in upsert_session
        let is_human = is_human_prompt_for(&e);
        assert_eq!(is_human, 1, "plain string content should be a human prompt");
    }

    #[test]
    fn text_block_array_is_human_prompt() {
        let e = entry(r#"{
            "type":"user","uuid":"u2","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z",
            "message":{"role":"user","content":[{"type":"text","text":"hello"}]}
        }"#);
        assert_eq!(is_human_prompt_for(&e), 1);
    }

    #[test]
    fn tool_result_only_is_not_human_prompt() {
        let e = entry(r#"{
            "type":"user","uuid":"u3","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z",
            "message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]}
        }"#);
        assert_eq!(is_human_prompt_for(&e), 0, "tool_result-only should not be a human prompt");
    }

    #[test]
    fn mixed_tool_result_and_text_is_human_prompt() {
        // If the user also typed something alongside a tool result, count it.
        let e = entry(r#"{
            "type":"user","uuid":"u4","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z",
            "message":{"role":"user","content":[
                {"type":"tool_result","tool_use_id":"t1","content":"ok"},
                {"type":"text","text":"and also this"}
            ]}
        }"#);
        assert_eq!(is_human_prompt_for(&e), 1);
    }

    #[test]
    fn assistant_role_is_not_human_prompt() {
        let e = entry(r#"{
            "type":"assistant","uuid":"u5","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z",
            "message":{"role":"assistant","content":"I will help"}
        }"#);
        assert_eq!(is_human_prompt_for(&e), 0);
    }

    #[test]
    fn summary_entry_is_not_human_prompt() {
        let e = entry(r#"{
            "type":"summary","uuid":"u6","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z",
            "message":{"role":"user","content":"summary text"}
        }"#);
        assert_eq!(is_human_prompt_for(&e), 0);
    }

    // ── Full upsert_session integration ──────────────────────────────────────

    #[test]
    fn upsert_session_counts_user_messages_correctly() {
        let conn = in_memory_db();
        // Insert a dummy project row first (FK constraint)
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('p1','/','')",
            [],
        ).unwrap();

        let entries: Vec<JournalEntry> = vec![
            entry(r#"{"type":"user","uuid":"a","sessionId":"s","timestamp":"2025-01-01T00:00:00Z","message":{"role":"user","content":"first human message"}}"#),
            entry(r#"{"type":"user","uuid":"b","sessionId":"s","timestamp":"2025-01-01T00:00:01Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"done"}]}}"#),
            entry(r#"{"type":"assistant","uuid":"c","sessionId":"s","timestamp":"2025-01-01T00:00:02Z","message":{"role":"assistant","content":"response"}}"#),
            entry(r#"{"type":"user","uuid":"d","sessionId":"s","timestamp":"2025-01-01T00:00:03Z","message":{"role":"user","content":"second human message"}}"#),
        ];

        let count = super::upsert_session(&conn, "s", "p1", &entries).unwrap();
        assert_eq!(count, 4, "all 4 entries should be inserted");

        let user_msg_count: i64 = conn
            .query_row("SELECT user_message_count FROM sessions WHERE id = 's'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(user_msg_count, 2, "only 2 human-typed messages");

        let human_flags: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT is_human_prompt FROM messages ORDER BY ordinal"
            ).unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap()
                .filter_map(|r| r.ok()).collect()
        };
        assert_eq!(human_flags, vec![1, 0, 0, 1]);
    }

    // ── needs_reindex ─────────────────────────────────────────────────────────

    #[test]
    fn needs_reindex_true_when_session_not_in_db() {
        let conn = in_memory_db();
        let f = tempfile::NamedTempFile::new().unwrap();
        // Session "missing" does not exist in the DB → must re-index
        assert!(super::needs_reindex(&conn, "missing", f.path()));
    }

    #[test]
    fn needs_reindex_true_when_file_newer_than_indexed_at() {
        let conn = in_memory_db();
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('p','/','')",
            [],
        ).unwrap();
        // Set indexed_at to the distant past
        conn.execute(
            "INSERT INTO sessions (id, project_id, title, message_count, created_at, updated_at, indexed_at)
             VALUES ('s','p',NULL,0,'2020-01-01','2020-01-01','1970-01-01 00:00:00')",
            [],
        ).unwrap();
        let f = tempfile::NamedTempFile::new().unwrap();
        // Any real file's mtime will be >> epoch → needs reindex
        assert!(super::needs_reindex(&conn, "s", f.path()));
    }

    #[test]
    fn needs_reindex_false_when_file_older_than_indexed_at() {
        let conn = in_memory_db();
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('p','/','')",
            [],
        ).unwrap();
        // Set indexed_at far in the future (year 9999)
        conn.execute(
            "INSERT INTO sessions (id, project_id, title, message_count, created_at, updated_at, indexed_at)
             VALUES ('s','p',NULL,0,'2020-01-01','2020-01-01','9999-12-31 23:59:59')",
            [],
        ).unwrap();
        let f = tempfile::NamedTempFile::new().unwrap();
        // File mtime will be << year 9999 → no reindex needed
        assert!(!super::needs_reindex(&conn, "s", f.path()));
    }

    // ── run_full_index end-to-end ─────────────────────────────────────────────

    /// Write JSONL lines to a temp file inside `dir/<project_name>/<session_id>.jsonl`.
    fn write_session(
        base: &std::path::Path,
        project: &str,
        session_id: &str,
        lines: &[&str],
    ) -> std::path::PathBuf {
        let proj_dir = base.join("projects").join(project);
        std::fs::create_dir_all(&proj_dir).unwrap();
        let path = proj_dir.join(format!("{session_id}.jsonl"));
        let content = lines.join("\n") + "\n";
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn run_full_index_empty_projects_dir_returns_zero_stats() {
        let conn = in_memory_db();
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("projects")).unwrap();
        let stats = super::run_full_index(&conn, tmp.path()).unwrap();
        assert_eq!(stats.projects_indexed, 0);
        assert_eq!(stats.sessions_indexed, 0);
        assert_eq!(stats.messages_indexed, 0);
    }

    #[test]
    fn run_full_index_missing_projects_dir_returns_zero_stats() {
        let conn = in_memory_db();
        let tmp = tempfile::TempDir::new().unwrap();
        // Don't create the projects dir at all
        let stats = super::run_full_index(&conn, tmp.path()).unwrap();
        assert_eq!(stats.projects_indexed, 0);
    }

    #[test]
    fn run_full_index_indexes_project_and_session() {
        let conn = in_memory_db();
        let tmp = tempfile::TempDir::new().unwrap();

        // Encode project name the same way Claude Code does: /Users/alice → -Users-alice
        let project = "-Users-alice-Code-myproject";
        write_session(tmp.path(), project, "sess-001", &[
            r#"{"type":"user","uuid":"m1","sessionId":"sess-001","timestamp":"2025-03-10T09:00:00Z","message":{"role":"user","content":"first message"}}"#,
            r#"{"type":"user","uuid":"m2","sessionId":"sess-001","timestamp":"2025-03-10T09:00:01Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]}}"#,
            r#"{"type":"assistant","uuid":"m3","sessionId":"sess-001","timestamp":"2025-03-10T09:00:02Z","message":{"role":"assistant","content":"done"}}"#,
        ]);

        let stats = super::run_full_index(&conn, tmp.path()).unwrap();
        assert_eq!(stats.projects_indexed, 1);
        assert_eq!(stats.sessions_indexed, 1);
        assert_eq!(stats.messages_indexed, 3);

        // Project row exists
        let proj_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects WHERE id = ?1", [project], |r| r.get(0))
            .unwrap();
        assert_eq!(proj_count, 1);

        // Session row has correct user_message_count (only 1 plain-text prompt)
        let umc: i64 = conn
            .query_row("SELECT user_message_count FROM sessions WHERE id = 'sess-001'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(umc, 1);

        // Messages table populated
        let msg_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages WHERE session_id = 'sess-001'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(msg_count, 3);
    }

    #[test]
    fn run_full_index_skips_unchanged_sessions_on_second_run() {
        let conn = in_memory_db();
        let tmp = tempfile::TempDir::new().unwrap();

        write_session(tmp.path(), "-p1", "s1", &[
            r#"{"type":"user","uuid":"m1","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z","message":{"role":"user","content":"hi"}}"#,
        ]);

        // First run indexes it
        let s1 = super::run_full_index(&conn, tmp.path()).unwrap();
        assert_eq!(s1.sessions_indexed, 1);

        // Second run without file changes: file mtime has NOT advanced, so skip
        let s2 = super::run_full_index(&conn, tmp.path()).unwrap();
        assert_eq!(s2.sessions_indexed, 0, "unchanged file must be skipped");
    }

    #[test]
    fn run_full_index_multiple_projects_and_sessions() {
        let conn = in_memory_db();
        let tmp = tempfile::TempDir::new().unwrap();

        let line = |uuid: &str, sid: &str| -> String {
            format!(r#"{{"type":"user","uuid":"{uuid}","sessionId":"{sid}","timestamp":"2025-05-01T00:00:00Z","message":{{"role":"user","content":"msg"}}}}"#)
        };

        write_session(tmp.path(), "-proj-a", "s-a1", &[&line("u1", "s-a1")]);
        write_session(tmp.path(), "-proj-a", "s-a2", &[&line("u2", "s-a2")]);
        write_session(tmp.path(), "-proj-b", "s-b1", &[&line("u3", "s-b1")]);

        let stats = super::run_full_index(&conn, tmp.path()).unwrap();
        assert_eq!(stats.projects_indexed, 2);
        assert_eq!(stats.sessions_indexed, 3);
        assert_eq!(stats.messages_indexed, 3);
    }

    // ── Helper: mirrors the is_human_prompt logic from upsert_session ─────────

    fn is_human_prompt_for(entry: &JournalEntry) -> i64 {
        let role = entry.message.as_ref()
            .and_then(|m| m.role.as_deref())
            .unwrap_or(&entry.entry_type);
        let is_summary = if entry.entry_type == "summary" { 1i64 } else { 0 };

        if role == "user" && is_summary == 0 {
            if let Some(msg) = &entry.message {
                if let Some(content) = &msg.content {
                    if content.is_string() {
                        return 1;
                    } else if let Some(arr) = content.as_array() {
                        if arr.iter().any(|b| {
                            b.get("type").and_then(|t| t.as_str()) == Some("text")
                        }) { return 1; }
                    }
                }
            }
        }
        0
    }
}
