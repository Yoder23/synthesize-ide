use rusqlite::{Connection, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../schema.sql"))
}

pub fn append_event(conn: &Connection, session_id: &str, kind: &str, payload_json: &str) -> Result<String> {
    let id = new_id("evt");
    conn.execute(
        "INSERT INTO audit_events (id, session_id, timestamp, kind, payload_json) VALUES (?1, ?2, datetime('now'), ?3, ?4)",
        (&id, session_id, kind, payload_json),
    )?;
    Ok(id)
}

pub fn new_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}-{}", prefix, std::process::id(), nanos)
}
