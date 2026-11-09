use super::migrations;
use crate::error::AppError;
use rusqlite::Connection;
use std::{path::Path, time::Duration};

pub fn open(path: &Path) -> Result<Connection, AppError> {
    let initialize = || -> Result<Connection, Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        migrations::apply(&mut connection)?;
        Ok(connection)
    };
    initialize().map_err(|error| {
        eprintln!("Database initialization failed: {error}");
        AppError::DatabaseInitialization
    })
}
