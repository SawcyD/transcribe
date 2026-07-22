use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension, Row};

use crate::{
    errors::AppError,
    models::{DashboardStats, DictationMode, InsertionStatus, PostPasteAction, TranscriptRecord},
};

use super::Database;

impl Database {
    pub fn insert_transcript(&self, record: &TranscriptRecord) -> Result<(), AppError> {
        self.with_connection(|connection| {
            connection.execute(
                r#"INSERT INTO transcripts(
                    id, created_at, started_at, duration_ms, processing_ms, application_name, process_name,
                    window_title, mode, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript,
                    transform_id, provider, model, confidence, insertion_status, post_paste_action, audio_path, is_favorite
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)"#,
                params![
                    record.id, record.created_at.to_rfc3339(), record.started_at.to_rfc3339(), record.duration_ms,
                    record.processing_ms, record.application_name, record.process_name, record.window_title,
                    mode_str(record.mode), record.raw_transcript, record.normalized_transcript, record.cleaned_transcript,
                    record.final_transcript, record.transform_id, record.provider, record.model, record.confidence,
                    insertion_str(record.insertion_status), action_str(record.post_paste_action), record.audio_path,
                    i64::from(record.is_favorite),
                ],
            )?;
            let word_count = record.final_transcript.split_whitespace().count() as i64;
            connection.execute(
                "INSERT INTO usage_stats(day, word_count, session_count, audio_duration_ms)
                 VALUES (date('now', 'localtime'), ?1, 1, ?2)
                 ON CONFLICT(day) DO UPDATE SET word_count = word_count + excluded.word_count,
                   session_count = session_count + 1, audio_duration_ms = audio_duration_ms + excluded.audio_duration_ms",
                params![word_count, record.duration_ms],
            )?;
            Ok(())
        })
    }

    pub fn list_transcripts(&self, query: &str) -> Result<Vec<TranscriptRecord>, AppError> {
        self.with_connection(|connection| {
            let pattern = format!("%{}%", query.trim());
            let mut statement = connection.prepare(
                r#"SELECT id, created_at, started_at, duration_ms, processing_ms, application_name, process_name,
                   window_title, mode, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript,
                   transform_id, provider, model, confidence, insertion_status, post_paste_action, audio_path, is_favorite
                   FROM transcripts
                   WHERE ?1 = '' OR raw_transcript LIKE ?2 OR normalized_transcript LIKE ?2 OR cleaned_transcript LIKE ?2
                     OR final_transcript LIKE ?2 OR application_name LIKE ?2 OR window_title LIKE ?2
                   ORDER BY created_at DESC LIMIT 500"#,
            )?;
            let rows = statement.query_map(params![query.trim(), pattern], transcript_from_row)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
        })
    }

    pub fn clear_transcripts(&self) -> Result<usize, AppError> {
        self.with_connection(|connection| {
            connection
                .execute("DELETE FROM transcripts", [])
                .map_err(AppError::from)
        })
    }

    /// Enforces the retention settings. `0` disables either limit.
    pub fn prune_transcripts(
        &self,
        retention_days: u32,
        max_entries: u32,
    ) -> Result<usize, AppError> {
        if retention_days == 0 && max_entries == 0 {
            return Ok(0);
        }
        self.with_connection(|connection| {
            let mut removed = 0usize;
            if retention_days > 0 {
                let cutoff = Utc::now() - Duration::days(i64::from(retention_days));
                removed += connection.execute(
                    "DELETE FROM transcripts WHERE created_at < ?1",
                    params![cutoff.to_rfc3339()],
                )?;
            }
            if max_entries > 0 {
                removed += connection.execute(
                    r#"DELETE FROM transcripts WHERE id NOT IN (
                           SELECT id FROM transcripts ORDER BY created_at DESC LIMIT ?1
                       )"#,
                    params![max_entries],
                )?;
            }
            Ok(removed)
        })
    }

    pub fn get_transcript(&self, id: &str) -> Result<TranscriptRecord, AppError> {
        self.with_connection(|connection| {
            connection.query_row(
                r#"SELECT id, created_at, started_at, duration_ms, processing_ms, application_name, process_name,
                   window_title, mode, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript,
                   transform_id, provider, model, confidence, insertion_status, post_paste_action, audio_path, is_favorite
                   FROM transcripts WHERE id = ?1"#,
                params![id],
                transcript_from_row,
            ).optional()?.ok_or_else(|| AppError::Database("transcript was not found".into()))
        })
    }

    pub fn delete_transcript(&self, id: &str) -> Result<(), AppError> {
        self.with_connection(|connection| {
            connection.execute("DELETE FROM transcripts WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn dashboard_stats(&self) -> Result<DashboardStats, AppError> {
        self.with_connection(|connection| {
            let values = connection.query_row(
                "SELECT word_count, session_count FROM usage_stats WHERE day = date('now', 'localtime')",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            ).optional()?.unwrap_or((0, 0));
            Ok(DashboardStats {
                daily_words: values.0.max(0) as u64,
                daily_sessions: values.1.max(0) as u64,
                estimated_minutes_saved: values.0.max(0) as f64 / 40.0,
            })
        })
    }
}

fn transcript_from_row(row: &Row<'_>) -> rusqlite::Result<TranscriptRecord> {
    let created: String = row.get(1)?;
    let started: String = row.get(2)?;
    Ok(TranscriptRecord {
        id: row.get(0)?,
        created_at: parse_datetime(created, 1)?,
        started_at: parse_datetime(started, 2)?,
        duration_ms: row.get(3)?,
        processing_ms: row.get(4)?,
        application_name: row.get(5)?,
        process_name: row.get(6)?,
        window_title: row.get(7)?,
        mode: parse_mode(&row.get::<_, String>(8)?),
        raw_transcript: row.get(9)?,
        normalized_transcript: row.get(10)?,
        cleaned_transcript: row.get(11)?,
        final_transcript: row.get(12)?,
        transform_id: row.get(13)?,
        provider: row.get(14)?,
        model: row.get(15)?,
        confidence: row.get(16)?,
        insertion_status: parse_insertion(&row.get::<_, String>(17)?),
        post_paste_action: parse_action(&row.get::<_, String>(18)?),
        audio_path: row.get(19)?,
        is_favorite: row.get::<_, i64>(20)? != 0,
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

fn mode_str(value: DictationMode) -> &'static str {
    match value {
        DictationMode::PushToTalk => "push_to_talk",
        DictationMode::HandsFree => "hands_free",
        DictationMode::Call => "call",
        DictationMode::Command => "command",
    }
}
fn insertion_str(value: InsertionStatus) -> &'static str {
    match value {
        InsertionStatus::Inserted => "inserted",
        InsertionStatus::Copied => "copied",
        InsertionStatus::Failed => "failed",
        InsertionStatus::Cancelled => "cancelled",
    }
}
fn action_str(value: PostPasteAction) -> &'static str {
    match value {
        PostPasteAction::None => "none",
        PostPasteAction::Enter => "enter",
        PostPasteAction::Tab => "tab",
        PostPasteAction::Newline => "newline",
    }
}
fn parse_mode(value: &str) -> DictationMode {
    match value {
        "hands_free" => DictationMode::HandsFree,
        "call" => DictationMode::Call,
        "command" => DictationMode::Command,
        _ => DictationMode::PushToTalk,
    }
}
fn parse_insertion(value: &str) -> InsertionStatus {
    match value {
        "inserted" => InsertionStatus::Inserted,
        "copied" => InsertionStatus::Copied,
        "cancelled" => InsertionStatus::Cancelled,
        _ => InsertionStatus::Failed,
    }
}
fn parse_action(value: &str) -> PostPasteAction {
    match value {
        "enter" => PostPasteAction::Enter,
        "tab" => PostPasteAction::Tab,
        "newline" => PostPasteAction::Newline,
        _ => PostPasteAction::None,
    }
}
