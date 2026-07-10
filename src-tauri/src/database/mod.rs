mod dictionary;
mod migrations;
mod settings;
mod transcripts;

use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;

use crate::errors::AppError;

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        migrations::run(&connection)?;
        let database = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        database.seed_developer_dictionary()?;
        Ok(database)
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| AppError::Database("database lock was poisoned".into()))?;
        operation(&guard)
    }
}
