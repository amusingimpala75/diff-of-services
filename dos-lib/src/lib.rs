use std::fs;

use anyhow::{Context, Result};
use rusqlite::Connection;
use time::{OffsetDateTime, macros::format_description};

pub mod directories;
pub mod document;
pub mod revision;

/// Open the database connection in the data directory for the application
pub fn open_connection_file() -> Result<Connection> {
    // Make sure directory exists, skipping if already exists
    fs::create_dir_all(directories::data_dir()).context("Creating data directory")?;
    // Open database
    let conn = Connection::open(directories::data_dir().join("db.sqlite3"))
        .context("Opening database file")?;
    // Setup database if necessary
    setup_connection(&conn).context("Setting up database")?;
    Ok(conn)
}

/// Create in memory connection. Only for internal testing
#[cfg(test)]
fn open_connection_memory() -> Result<Connection> {
    // Open connection
    let conn = Connection::open_in_memory()?;
    // Setup database if necessary
    setup_connection(&conn)?;
    Ok(conn)
}

/// Setup connection, creating tables and enforcing foreign keys
fn setup_connection(conn: &Connection) -> Result<()> {
    // Enforce foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    // Create tables
    document::Document::ensure_table_exists(conn)?;
    revision::Revision::ensure_table_exists(conn)?;

    Ok(())
}

/// Formats the given time as a string. Shows date/time down to the minute
pub fn format_local_time(time: OffsetDateTime) -> String {
    let formatter = format_description!("[year]-[month]-[day] at [hour]:[minute]");
    time.format(formatter).unwrap()
}
