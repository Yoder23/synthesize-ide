use rusqlite::{params, Connection, OptionalExtension, Result};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS schema_migrations (
           version INTEGER PRIMARY KEY,
           name TEXT NOT NULL,
           applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );",
    )?;
    apply_migration(conn, 1, "base_audit_schema", include_str!("../schema.sql"))?;
    apply_migration(
        conn,
        2,
        "outcome_governed_studio",
        include_str!("../migrations/002_outcome_studio.sql"),
    )?;
    apply_migration(
        conn,
        3,
        "studio_hardening",
        include_str!("../migrations/003_studio_hardening.sql"),
    )?;
    apply_migration(
        conn,
        4,
        "context_operating_system",
        include_str!("../migrations/004_context_operating_system.sql"),
    )?;
    apply_migration(
        conn,
        5,
        "dream_factory_controller",
        include_str!("../migrations/005_dream_factory_controller.sql"),
    )?;
    apply_migration(
        conn,
        6,
        "autonomous_app_foundry",
        include_str!("../migrations/006_app_foundry.sql"),
    )?;
    Ok(())
}

pub fn current_schema_version(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )
}

pub fn applied_migrations(conn: &Connection) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare("SELECT version, name FROM schema_migrations ORDER BY version")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

fn apply_migration(conn: &Connection, version: i64, name: &str, sql: &str) -> Result<()> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT name FROM schema_migrations WHERE version = ?1",
            [version],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(existing_name) = existing {
        if existing_name != name {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "migration version {version} is recorded as {existing_name}, expected {name}"
            )));
        }
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    transaction.execute_batch(sql)?;
    transaction.execute(
        "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
        params![version, name],
    )?;
    transaction.commit()
}

pub fn append_event(
    conn: &Connection,
    session_id: &str,
    kind: &str,
    payload_json: &str,
) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TemporaryDatabase(PathBuf);

    impl Drop for TemporaryDatabase {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn migrations_are_ordered_and_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
        assert_eq!(current_schema_version(&conn).unwrap(), 6);
        assert_eq!(
            applied_migrations(&conn).unwrap(),
            vec![
                (1, "base_audit_schema".to_string()),
                (2, "outcome_governed_studio".to_string()),
                (3, "studio_hardening".to_string()),
                (4, "context_operating_system".to_string()),
                (5, "dream_factory_controller".to_string())
            ]
        );
    }

    #[test]
    fn studio_migration_preserves_existing_records() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../schema.sql")).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, repo_root) VALUES ('legacy', '/repo')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO audit_events (id, session_id, kind, payload_json) VALUES ('evt', 'legacy', 'legacy.event', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO context_bundles (id, session_id, token_estimate, payload_json)
             VALUES ('legacy-context','legacy',123,'{}')",
            [],
        )
        .unwrap();

        init_schema(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let studio_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='initiatives'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(studio_table, 1);
        let legacy_counting_label: String = conn
            .query_row(
                "SELECT token_count_method FROM context_bundles WHERE id='legacy-context'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(legacy_counting_label, "legacy_character_count_not_tokens");
    }

    #[test]
    fn persistent_state_survives_connection_restart() {
        let path =
            std::env::temp_dir().join(format!("synthesize-restart-{}.sqlite3", new_id("test")));
        let database = TemporaryDatabase(path.clone());
        {
            let conn = Connection::open(&database.0).unwrap();
            init_schema(&conn).unwrap();
            conn.execute(
                "INSERT INTO sessions (id, repo_root) VALUES ('restart', '/repo')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO initiatives
                 (id, session_id, repo_root, mode, title, source, status, active_spec_version)
                 VALUES ('INIT-RESTART', 'restart', '/repo', 'studio', 'Persistent', 'test', 'created', 1)",
                [],
            )
            .unwrap();
        }
        let reopened = Connection::open(&database.0).unwrap();
        init_schema(&reopened).unwrap();
        let title: String = reopened
            .query_row(
                "SELECT title FROM initiatives WHERE id='INIT-RESTART'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Persistent");
        assert_eq!(current_schema_version(&reopened).unwrap(), 6);
    }
}
