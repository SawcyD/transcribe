use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::{errors::AppError, models::AppSettings};

use super::Database;

impl Database {
    pub fn load_settings(&self) -> Result<AppSettings, AppError> {
        self.with_connection(|connection| {
            let stored: Option<String> = connection
                .query_row(
                    "SELECT value_json FROM settings WHERE key = 'app_settings'",
                    [],
                    |row| row.get(0),
                )
                .optional()?;
            match stored {
                Some(value) => serde_json::from_str(&value).map_err(|error| {
                    AppError::Database(format!("settings JSON is invalid: {error}"))
                }),
                None => Ok(AppSettings::default()),
            }
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), AppError> {
        settings.validate().map_err(AppError::Configuration)?;
        let value = serde_json::to_string(settings)
            .map_err(|error| AppError::Database(error.to_string()))?;
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO settings(key, value_json, updated_at) VALUES ('app_settings', ?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
                params![value, Utc::now().to_rfc3339()],
            )?;
            Ok(())
        })
    }
}
