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
    // Create tables
    document::Document::ensure_table_exists(&conn).ok()?;
    revision::Revision::ensure_table_exists(&conn).ok()?;
    Some(conn)
}

pub fn open_connection_memory() -> Option<Connection> {
    let conn = Connection::open_in_memory().ok()?;
    document::Document::ensure_table_exists(&conn).ok()?;
    revision::Revision::ensure_table_exists(&conn).ok()?;
    Some(conn)
}
