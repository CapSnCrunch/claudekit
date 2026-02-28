use rusqlite::Connection;
use crate::db::DbError;

const MIGRATIONS: &[&str] = &[
    // v1 — initial schema (see below),
    // v2 — human prompt tracking
    "
    CREATE TABLE IF NOT EXISTS schema_migrations (
        version    INTEGER PRIMARY KEY,
        applied_at TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS projects (
        id            TEXT PRIMARY KEY,
        decoded_path  TEXT NOT NULL,
        display_name  TEXT NOT NULL,
        session_count INTEGER NOT NULL DEFAULT 0,
        last_active   TEXT,
        created_at    TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS sessions (
        id                   TEXT PRIMARY KEY,
        project_id           TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        title                TEXT,
        message_count        INTEGER NOT NULL DEFAULT 0,
        total_input_tokens   INTEGER NOT NULL DEFAULT 0,
        total_output_tokens  INTEGER NOT NULL DEFAULT 0,
        total_cost_usd       REAL,
        created_at           TEXT NOT NULL,
        updated_at           TEXT NOT NULL,
        indexed_at           TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON sessions(project_id);
    CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at);

    CREATE TABLE IF NOT EXISTS messages (
        id            TEXT PRIMARY KEY,
        session_id    TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
        parent_id     TEXT,
        role          TEXT NOT NULL,
        is_summary    INTEGER NOT NULL DEFAULT 0,
        content_json  TEXT NOT NULL,
        input_tokens  INTEGER,
        output_tokens INTEGER,
        cost_usd      REAL,
        model         TEXT,
        timestamp     TEXT NOT NULL,
        ordinal       INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
    CREATE INDEX IF NOT EXISTS idx_messages_timestamp  ON messages(timestamp);
    ",

    // v2 — add is_human_prompt to messages, user_message_count to sessions,
    //       reset indexed_at to force full re-index so new columns are populated
    "
    ALTER TABLE messages ADD COLUMN is_human_prompt INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE sessions ADD COLUMN user_message_count INTEGER NOT NULL DEFAULT 0;
    UPDATE sessions SET indexed_at = '1970-01-01 00:00:00';
    ",

    // v3 — force re-index again: previous v2 re-index ran with the old binary
    //       before is_human_prompt detection was compiled in, so all sessions
    //       got indexed with is_human_prompt = 0. Reset indexed_at so the next
    //       launch re-indexes everything with the correct code.
    "UPDATE sessions SET indexed_at = '1970-01-01 00:00:00';",
];

pub fn run_migrations(conn: &Connection) -> Result<(), DbError> {
    // NOTE: tests live at the bottom of this file.
    // Ensure the migrations table exists before querying it
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for (i, migration) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current_version {
            conn.execute_batch(migration)
                .map_err(|e| DbError::Migration(format!("v{version}: {e}")))?;
            conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                [version],
            )?;
        }
    }

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn migrations_run_on_fresh_db() {
        let conn = fresh();
        run_migrations(&conn).unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = fresh();
        run_migrations(&conn).unwrap();
        // Running again must not fail (no duplicate inserts, no re-applied ALTER TABLE)
        run_migrations(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64, "each version recorded exactly once");
    }

    #[test]
    fn schema_has_expected_tables() {
        let conn = fresh();
        run_migrations(&conn).unwrap();
        for table in &["projects", "sessions", "messages", "schema_migrations"] {
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "table '{table}' must exist after migrations");
        }
    }

    #[test]
    fn sessions_table_has_user_message_count_column() {
        let conn = fresh();
        run_migrations(&conn).unwrap();
        // If the column doesn't exist this INSERT would fail
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('p','/','')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, project_id, title, message_count, user_message_count, created_at, updated_at)
             VALUES ('s','p',NULL,0,5,'2025-01-01','2025-01-01')",
            [],
        ).unwrap();
        let v: i64 = conn
            .query_row("SELECT user_message_count FROM sessions WHERE id='s'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 5);
    }

    #[test]
    fn messages_table_has_is_human_prompt_column() {
        let conn = fresh();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO projects (id, decoded_path, display_name) VALUES ('p','/','')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, project_id, title, message_count, created_at, updated_at) VALUES ('s','p',NULL,0,'2025-01-01','2025-01-01')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO messages (id, session_id, role, is_summary, is_human_prompt, content_json, timestamp, ordinal)
             VALUES ('m1','s','user',0,1,'\"hello\"','2025-01-01T00:00:00Z',0)",
            [],
        ).unwrap();
        let v: i64 = conn
            .query_row("SELECT is_human_prompt FROM messages WHERE id='m1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }
}
