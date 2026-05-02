use std::fs;

use rusqlite::Connection;

pub mod directories;
pub mod document;
pub mod revision;

/// Open the database connection in the data directory for the application
pub fn open_connection_file() -> Option<Connection> {
    // Make sure directory exists, skipping if already exists
    fs::create_dir_all(directories::data_dir()).ok()?;
    // Open database
    let conn = Connection::open(directories::data_dir().join("db.sqlite3")).ok()?;
    // Setup database if necessary
    setup_connection(&conn).ok()?;
    Some(conn)
}

/// Create in memory connection. Only for internal testing
#[cfg(test)]
fn open_connection_memory() -> Option<Connection> {
    // Open connection
    let conn = Connection::open_in_memory().ok()?;
    // Setup database if necessary
    setup_connection(&conn).ok()?;
    Some(conn)
}

/// Setup connection, creating tables and enforcing foreign keys
fn setup_connection(conn: &Connection) -> rusqlite::Result<()> {
    // Enforce foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    // Create tables
    document::Document::ensure_table_exists(&conn)?;
    revision::Revision::ensure_table_exists(&conn)?;

    Ok(())
}
