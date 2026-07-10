use std::str::FromStr;

use chrono::{DateTime, Utc};
use rusqlite::{params, Row};

use crate::{
    errors::AppError,
    models::{DictionaryCategory, DictionaryEntry},
};

use super::Database;

const DEVELOPER_TERMS: &[&str] = &[
    "Tauri",
    "Rust",
    "Cargo",
    "TypeScript",
    "JavaScript",
    "React",
    "Next.js",
    "GitHub",
    "Vercel",
    "Supabase",
    "PostgreSQL",
    "API",
    "JSON",
    "WebSocket",
    "Roblox",
    "Roblox Studio",
    "roblox-ts",
    "Luau",
    "Rojo",
    "RemoteEvent",
    "RemoteFunction",
    "DataStore",
    "DataStoreService",
    "MarketplaceService",
    "ReplicatedStorage",
    "ServerScriptService",
    "StarterGui",
    "ScrollingFrame",
    "UIListLayout",
    "AutomaticCanvasSize",
    "CFrame",
    "Vector3",
    "UDim2",
    "ViewportFrame",
];

impl Database {
    pub fn upsert_dictionary_entry(
        &self,
        id: Option<&str>,
        input: &crate::models::DictionaryEntryInput,
    ) -> Result<DictionaryEntry, AppError> {
        let id = id
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("entry:{}", uuid::Uuid::new_v4()));
        let display_term = input.display_term.trim();
        if display_term.is_empty()
            || input
                .spoken_forms
                .iter()
                .all(|value| value.trim().is_empty())
        {
            return Err(AppError::Configuration(
                "dictionary entries need a written term and a spoken form".into(),
            ));
        }
        let now = Utc::now();
        let spoken_json = serde_json::to_string(&input.spoken_forms)
            .map_err(|error| AppError::Database(error.to_string()))?;
        self.with_connection(|connection| {
            connection.execute(
                "INSERT INTO dictionary_entries(id, display_term, normalized_term, spoken_forms_json, replacement, category, priority, case_sensitive, whole_word_only, enabled, usage_count, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, COALESCE((SELECT created_at FROM dictionary_entries WHERE id = ?1), ?11), ?11)
                 ON CONFLICT(id) DO UPDATE SET display_term=excluded.display_term, normalized_term=excluded.normalized_term, spoken_forms_json=excluded.spoken_forms_json, replacement=excluded.replacement, category=excluded.category, priority=excluded.priority, case_sensitive=excluded.case_sensitive, whole_word_only=excluded.whole_word_only, enabled=excluded.enabled, updated_at=excluded.updated_at",
                rusqlite::params![
                    id,
                    display_term,
                    display_term.to_lowercase(),
                    spoken_json,
                    input.replacement,
                    category_str(input.category),
                    input.priority,
                    i64::from(input.case_sensitive),
                    i64::from(input.whole_word_only),
                    i64::from(input.enabled),
                    now.to_rfc3339(),
                ],
            )?;
            connection.query_row(
                "SELECT id, display_term, spoken_forms_json, replacement, category, priority, case_sensitive, whole_word_only, enabled, usage_count, created_at, updated_at FROM dictionary_entries WHERE id = ?1",
                rusqlite::params![id],
                dictionary_from_row,
            ).map_err(AppError::from)
        })
    }

    pub fn delete_dictionary_entry(&self, id: &str) -> Result<(), AppError> {
        self.with_connection(|connection| {
            connection.execute(
                "DELETE FROM dictionary_entries WHERE id = ?1 AND id NOT LIKE 'builtin:%'",
                rusqlite::params![id],
            )?;
            Ok(())
        })
    }

    pub fn seed_developer_dictionary(&self) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        self.with_connection(|connection| {
            connection.execute(
                "INSERT OR IGNORE INTO dictionary_profiles(id, name, description, built_in, created_at, updated_at)
                 VALUES ('builtin:developer', 'Developer', 'Built-in technical vocabulary', 1, ?1, ?1)",
                params![now],
            )?;
            for (priority, term) in DEVELOPER_TERMS.iter().enumerate() {
                let id = format!("builtin:{}", term.to_lowercase().replace([' ', '.'], "-"));
                let spoken = serde_json::to_string(&default_spoken_forms(term)).map_err(|error| AppError::Database(error.to_string()))?;
                connection.execute(
                    "INSERT OR IGNORE INTO dictionary_entries(
                      id, display_term, normalized_term, spoken_forms_json, replacement, category, priority,
                      case_sensitive, whole_word_only, enabled, usage_count, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, NULL, 'protected_identifier', ?5, 1, 1, 1, 0, ?6, ?6)",
                    params![id, term, term.to_lowercase(), spoken, 1000 - priority as i32, now],
                )?;
                connection.execute(
                    "INSERT OR IGNORE INTO dictionary_profile_entries(profile_id, entry_id) VALUES ('builtin:developer', ?1)",
                    params![id],
                )?;
            }
            insert_replacement(connection, "builtin:tauri-v2", "Tauri v2", &["terry version two", "tauri version two"], &now)?;
            insert_replacement(connection, "builtin:roblox-ts", "roblox-ts", &["roblox ts", "roblox type script"], &now)?;
            Ok(())
        })
    }

    pub fn list_dictionary_entries(&self) -> Result<Vec<DictionaryEntry>, AppError> {
        self.with_connection(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, display_term, spoken_forms_json, replacement, category, priority, case_sensitive,
                 whole_word_only, enabled, usage_count, created_at, updated_at
                 FROM dictionary_entries ORDER BY category, priority DESC, display_term COLLATE NOCASE",
            )?;
            let rows = statement.query_map([], dictionary_from_row)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn enabled_dictionary_entries(&self) -> Result<Vec<DictionaryEntry>, AppError> {
        Ok(self
            .list_dictionary_entries()?
            .into_iter()
            .filter(|entry| entry.enabled)
            .collect())
    }

    pub fn dictionary_keyterms(&self) -> Result<Vec<String>, AppError> {
        Ok(self
            .enabled_dictionary_entries()?
            .into_iter()
            .map(|entry| entry.replacement.unwrap_or(entry.display_term))
            .collect())
    }
}

fn category_str(value: DictionaryCategory) -> &'static str {
    match value {
        DictionaryCategory::Vocabulary => "vocabulary",
        DictionaryCategory::Replacement => "replacement",
        DictionaryCategory::ProtectedIdentifier => "protected_identifier",
    }
}

fn insert_replacement(
    connection: &rusqlite::Connection,
    id: &str,
    written: &str,
    spoken: &[&str],
    now: &str,
) -> Result<(), AppError> {
    let spoken_json =
        serde_json::to_string(spoken).map_err(|error| AppError::Database(error.to_string()))?;
    connection.execute(
        "INSERT OR IGNORE INTO dictionary_entries(
          id, display_term, normalized_term, spoken_forms_json, replacement, category, priority,
          case_sensitive, whole_word_only, enabled, usage_count, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?2, 'replacement', 2000, 0, 1, 1, 0, ?5, ?5)",
        params![id, written, written.to_lowercase(), spoken_json, now],
    )?;
    Ok(())
}

fn default_spoken_forms(term: &str) -> Vec<String> {
    let mut forms = vec![term.to_lowercase()];
    match term {
        "TypeScript" => forms.push("type script".into()),
        "PostgreSQL" => forms.extend(["post gres q l".into(), "postgres".into()]),
        "UIListLayout" => forms.push("u i list layout".into()),
        "UDim2" => forms.push("u dim two".into()),
        "CFrame" => forms.push("c frame".into()),
        _ => {}
    }
    forms
}

fn dictionary_from_row(row: &Row<'_>) -> rusqlite::Result<DictionaryEntry> {
    let spoken_json: String = row.get(2)?;
    let created: String = row.get(10)?;
    let updated: String = row.get(11)?;
    Ok(DictionaryEntry {
        id: row.get(0)?,
        display_term: row.get(1)?,
        spoken_forms: serde_json::from_str(&spoken_json).unwrap_or_default(),
        replacement: row.get(3)?,
        category: match row.get::<_, String>(4)?.as_str() {
            "replacement" => DictionaryCategory::Replacement,
            "protected_identifier" => DictionaryCategory::ProtectedIdentifier,
            _ => DictionaryCategory::Vocabulary,
        },
        priority: row.get(5)?,
        case_sensitive: row.get::<_, i64>(6)? != 0,
        whole_word_only: row.get::<_, i64>(7)? != 0,
        enabled: row.get::<_, i64>(8)? != 0,
        usage_count: row.get(9)?,
        created_at: parse_datetime(created, 10)?,
        updated_at: parse_datetime(updated, 11)?,
    })
}

fn parse_datetime(value: String, column: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::<Utc>::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
