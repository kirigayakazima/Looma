use chrono::Utc;
use looma_core::{LoomaError, LoomaResult};
use rusqlite::Connection;
use tracing::info;

struct Migration {
    version: i32,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_initial",
        sql: include_str!("migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "002_audit_and_references",
        sql: include_str!("migrations/002_audit_and_references.sql"),
    },
];

pub fn run_migrations(conn: &mut Connection) -> LoomaResult<()> {
    // Ensure migrations table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| LoomaError::Database(format!("Failed to create migration table: {e}")))?;

    for m in MIGRATIONS {
        let applied: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _schema_migrations WHERE version = ?1)",
                [m.version],
                |row| row.get(0),
            )
            .map_err(|e| LoomaError::Database(format!("Failed to check migration version: {e}")))?;

        if !applied {
            info!("Applying migration {} ({})", m.version, m.name);
            let tx = conn
                .transaction()
                .map_err(|e| LoomaError::Database(format!("Failed to start migration transaction: {e}")))?;

            tx.execute_batch(m.sql)
                .map_err(|e| LoomaError::Database(format!("Migration {} ({}) failed: {e}", m.version, m.name)))?;

            tx.execute(
                "INSERT INTO _schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![m.version, m.name, Utc::now().to_rfc3339()],
            )
            .map_err(|e| LoomaError::Database(format!("Failed to record migration: {e}")))?;

            tx.commit()
                .map_err(|e| LoomaError::Database(format!("Failed to commit migration: {e}")))?;
            info!("Successfully applied migration {} ({})", m.version, m.name);
        }
    }

    Ok(())
}
