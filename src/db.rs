use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use std::path::Path;

pub fn init(path: &str) -> anyhow::Result<Arc<Mutex<Connection>>> {
    let exists = Path::new(path).exists();
    let conn = Connection::open(path)?;

    if !exists {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS providers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                base_url TEXT NOT NULL,
                models_endpoint TEXT NOT NULL,
                requires_key INTEGER NOT NULL DEFAULT 0,
                last_check_at TEXT,
                is_healthy INTEGER DEFAULT 0,
                response_time_ms INTEGER,
                model_count INTEGER,
                error_message TEXT
            );

            CREATE TABLE IF NOT EXISTS models (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id INTEGER NOT NULL,
                model_id TEXT NOT NULL,
                created_at TEXT,
                owned_by TEXT,
                UNIQUE(provider_id, model_id),
                FOREIGN KEY (provider_id) REFERENCES providers(id)
            );

            CREATE TABLE IF NOT EXISTS checks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_name TEXT NOT NULL,
                checked_at TEXT NOT NULL DEFAULT (datetime('now')),
                is_healthy INTEGER NOT NULL,
                response_time_ms INTEGER,
                model_count INTEGER,
                error_message TEXT
            );
            ",
        )?;
        tracing::info!("Database initialized at {path}");
    }

    Ok(Arc::new(Mutex::new(conn)))
}
