use rusqlite::Connection;

use crate::errors::AppError;

pub fn run(connection: &Connection) -> Result<(), AppError> {
    connection.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS transcripts (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            started_at TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            processing_ms INTEGER NOT NULL,
            application_name TEXT,
            process_name TEXT,
            window_title TEXT,
            mode TEXT NOT NULL CHECK (mode IN ('push_to_talk', 'hands_free', 'command')),
            raw_transcript TEXT NOT NULL,
            normalized_transcript TEXT NOT NULL,
            cleaned_transcript TEXT NOT NULL,
            final_transcript TEXT NOT NULL,
            transform_id TEXT,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            confidence REAL,
            insertion_status TEXT NOT NULL CHECK (insertion_status IN ('inserted', 'copied', 'failed', 'cancelled')),
            post_paste_action TEXT NOT NULL CHECK (post_paste_action IN ('none', 'enter', 'tab', 'newline')),
            audio_path TEXT,
            is_favorite INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_transcripts_created_at ON transcripts(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_transcripts_application ON transcripts(process_name, application_name);
        CREATE INDEX IF NOT EXISTS idx_transcripts_mode_status ON transcripts(mode, insertion_status);

        CREATE VIRTUAL TABLE IF NOT EXISTS transcript_search USING fts5(
            raw_transcript, normalized_transcript, cleaned_transcript, final_transcript,
            content='transcripts', content_rowid='rowid', tokenize='unicode61'
        );
        CREATE TRIGGER IF NOT EXISTS transcripts_ai AFTER INSERT ON transcripts BEGIN
          INSERT INTO transcript_search(rowid, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript)
          VALUES (new.rowid, new.raw_transcript, new.normalized_transcript, new.cleaned_transcript, new.final_transcript);
        END;
        CREATE TRIGGER IF NOT EXISTS transcripts_ad AFTER DELETE ON transcripts BEGIN
          INSERT INTO transcript_search(transcript_search, rowid, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript)
          VALUES ('delete', old.rowid, old.raw_transcript, old.normalized_transcript, old.cleaned_transcript, old.final_transcript);
        END;
        CREATE TRIGGER IF NOT EXISTS transcripts_au AFTER UPDATE ON transcripts BEGIN
          INSERT INTO transcript_search(transcript_search, rowid, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript)
          VALUES ('delete', old.rowid, old.raw_transcript, old.normalized_transcript, old.cleaned_transcript, old.final_transcript);
          INSERT INTO transcript_search(rowid, raw_transcript, normalized_transcript, cleaned_transcript, final_transcript)
          VALUES (new.rowid, new.raw_transcript, new.normalized_transcript, new.cleaned_transcript, new.final_transcript);
        END;

        CREATE TABLE IF NOT EXISTS dictionary_entries (
            id TEXT PRIMARY KEY,
            display_term TEXT NOT NULL,
            normalized_term TEXT NOT NULL,
            spoken_forms_json TEXT NOT NULL,
            replacement TEXT,
            category TEXT NOT NULL CHECK (category IN ('vocabulary', 'replacement', 'protected_identifier')),
            priority INTEGER NOT NULL DEFAULT 0,
            case_sensitive INTEGER NOT NULL DEFAULT 0,
            whole_word_only INTEGER NOT NULL DEFAULT 1,
            enabled INTEGER NOT NULL DEFAULT 1,
            usage_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_dictionary_normalized ON dictionary_entries(normalized_term, enabled, priority DESC);

        CREATE TABLE IF NOT EXISTS dictionary_profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            built_in INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS dictionary_profile_entries (
            profile_id TEXT NOT NULL REFERENCES dictionary_profiles(id) ON DELETE CASCADE,
            entry_id TEXT NOT NULL REFERENCES dictionary_entries(id) ON DELETE CASCADE,
            PRIMARY KEY(profile_id, entry_id)
        );

        CREATE TABLE IF NOT EXISTS transform_presets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            icon TEXT NOT NULL,
            system_prompt TEXT NOT NULL,
            output_mode TEXT NOT NULL CHECK (output_mode IN ('replace', 'preview', 'copy')),
            shortcut TEXT,
            auto_apply_eligible INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1,
            sort_order INTEGER NOT NULL DEFAULT 0,
            built_in INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS application_profiles (
            id TEXT PRIMARY KEY,
            application_name TEXT NOT NULL,
            process_executable TEXT NOT NULL,
            default_dictation_mode TEXT NOT NULL DEFAULT 'push_to_talk',
            default_transform_id TEXT,
            paste_shortcut TEXT NOT NULL DEFAULT 'ctrl_v',
            developer_mode INTEGER NOT NULL DEFAULT 0,
            automatic_submit_permission INTEGER NOT NULL DEFAULT 0,
            vocabulary_profile_id TEXT,
            cleanup_style TEXT NOT NULL DEFAULT 'balanced',
            context_access_permission INTEGER NOT NULL DEFAULT 0,
            priority INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS usage_stats (
            day TEXT PRIMARY KEY,
            word_count INTEGER NOT NULL DEFAULT 0,
            session_count INTEGER NOT NULL DEFAULT 0,
            audio_duration_ms INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS recent_identifiers (
            id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL,
            process_name TEXT,
            usage_count INTEGER NOT NULL DEFAULT 1,
            last_used_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS provider_metrics (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            stage TEXT NOT NULL,
            latency_ms INTEGER NOT NULL,
            success INTEGER NOT NULL,
            error_category TEXT,
            created_at TEXT NOT NULL
        );

        INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (1, datetime('now'));
    "#)?;
    Ok(())
}
