use rusqlite::Connection;
use crate::db::DbError;

const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
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
];

pub fn run_migrations(conn: &Connection) -> Result<(), DbError> {
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
