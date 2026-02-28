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
            (id, project_id, title, message_count, total_input_tokens, total_output_tokens,
             total_cost_usd, created_at, updated_at, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET
             title               = excluded.title,
             message_count       = excluded.message_count,
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

        conn.execute(
            "INSERT OR REPLACE INTO messages
                (id, session_id, parent_id, role, is_summary, content_json,
                 input_tokens, output_tokens, cost_usd, model, timestamp, ordinal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.uuid,
                session_id,
                entry.parent_uuid,
                role,
                is_summary,
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
