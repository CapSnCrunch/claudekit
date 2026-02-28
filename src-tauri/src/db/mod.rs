pub mod queries;
pub mod schema;

use rusqlite::Connection;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Migration error: {0}")]
    Migration(String),
}

pub fn open(path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    schema::run_migrations(&conn)?;
    Ok(conn)
}
