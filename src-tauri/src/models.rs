use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationState {
    Idle,
    Starting,
    ListeningPushToTalk,
    ListeningHandsFree,
    FinalizingAudio,
    Transcribing,
    Cleaning,
    Inserting,
    Completed,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictationMode {
    PushToTalk,
    HandsFree,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsertionStatus {
    Inserted,
    Copied,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PostPasteAction {
    #[default]
    None,
    Enter,
    Tab,
    Newline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorPayload {
    pub category: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationSnapshot {
    pub state: DictationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<DictationMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    pub interim_transcript: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppErrorPayload>,
}

impl Default for DictationSnapshot {
    fn default() -> Self {
        Self {
            state: DictationState::Idle,
            session_id: None,
            mode: None,
            started_at: None,
            interim_transcript: String::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelPayload {
    pub session_id: String,
    pub rms: f32,
    pub peak: f32,
    pub decibels: f32,
    pub bars: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptRecord {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub processing_ms: i64,
    pub application_name: Option<String>,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub mode: DictationMode,
    pub raw_transcript: String,
    pub normalized_transcript: String,
    pub cleaned_transcript: String,
    pub final_transcript: String,
    pub transform_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub confidence: Option<f32>,
    pub insertion_status: InsertionStatus,
    pub post_paste_action: PostPasteAction,
    pub audio_path: Option<String>,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DictionaryCategory {
    Vocabulary,
    Replacement,
    ProtectedIdentifier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntry {
    pub id: String,
    pub display_term: String,
    pub spoken_forms: Vec<String>,
    pub replacement: Option<String>,
    pub category: DictionaryCategory,
    pub priority: i32,
    pub case_sensitive: bool,
    pub whole_word_only: bool,
    pub enabled: bool,
    pub usage_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryEntryInput {
    pub display_term: String,
    pub spoken_forms: Vec<String>,
    pub replacement: Option<String>,
    pub category: DictionaryCategory,
    pub priority: i32,
    pub case_sensitive: bool,
    pub whole_word_only: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub microphone_name: Option<String>,
    pub transcription_provider: String,
    pub transcription_model: String,
    pub language: String,
    pub cleanup_enabled: bool,
    pub cleanup_endpoint: String,
    pub cleanup_model: String,
    pub cleanup_style: String,
    pub auto_apply_transform: Option<String>,
    pub paste_delay_ms: u64,
    pub restore_clipboard: bool,
    pub press_enter_enabled: bool,
    pub save_history: bool,
    pub save_audio: bool,
    pub session_limit_minutes: u32,
    pub noise_floor_db: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            microphone_name: None,
            transcription_provider: crate::brand::DEFAULT_TRANSCRIPTION_PROVIDER.into(),
            transcription_model: crate::brand::DEFAULT_TRANSCRIPTION_MODEL.into(),
            language: "en-US".into(),
            cleanup_enabled: true,
            cleanup_endpoint: "https://api.openai.com/v1".into(),
            cleanup_model: "gpt-4.1-mini".into(),
            cleanup_style: "balanced".into(),
            auto_apply_transform: None,
            paste_delay_ms: 140,
            restore_clipboard: true,
            press_enter_enabled: false,
            save_history: true,
            save_audio: false,
            session_limit_minutes: 20,
            noise_floor_db: -52.0,
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), String> {
        if !(40..=2_000).contains(&self.paste_delay_ms) {
            return Err("paste delay must be between 40 and 2000 milliseconds".into());
        }
        if !(1..=120).contains(&self.session_limit_minutes) {
            return Err("session limit must be between 1 and 120 minutes".into());
        }
        if !(-90.0..=-10.0).contains(&self.noise_floor_db) {
            return Err("noise floor must be between -90 and -10 dB".into());
        }
        let endpoint = url::Url::parse(&self.cleanup_endpoint)
            .map_err(|_| "cleanup endpoint is not a valid URL")?;
        let local = matches!(endpoint.host_str(), Some("localhost" | "127.0.0.1"));
        if endpoint.scheme() != "https" && !local {
            return Err("cleanup endpoint must use HTTPS unless it is local".into());
        }
        if self.transcription_model.trim().is_empty() || self.cleanup_model.trim().is_empty() {
            return Err("provider model names cannot be empty".into());
        }
        if !matches!(
            self.cleanup_style.as_str(),
            "balanced" | "casual" | "developer" | "code_literal"
        ) {
            return Err(
                "cleanup style must be balanced, casual, developer, or code_literal".into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub deepgram: bool,
    pub cleanup: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub daily_words: u64,
    pub daily_sessions: u64,
    pub estimated_minutes_saved: f64,
}

#[derive(Debug, Clone)]
pub struct ActiveTarget {
    pub window_handle: isize,
    pub control_handle: Option<isize>,
    pub process_id: u32,
    pub application_name: Option<String>,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordTiming {
    pub word: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct TranscriptionConfig {
    pub language: String,
    pub model: String,
    pub sample_rate: u32,
    pub encoding: String,
    pub interim_results: bool,
    pub punctuation: bool,
    pub smart_formatting: bool,
    pub dictionary_keyterms: Vec<String>,
    pub active_application: Option<String>,
    pub developer_mode: bool,
    pub endpointing_ms: u32,
}

#[derive(Debug, Clone, Default)]
pub struct TranscriptionResult {
    pub raw_text: String,
    pub final_segments: Vec<String>,
    pub interim_segments: Vec<String>,
    pub confidence: Option<f32>,
    pub detected_language: Option<String>,
    pub duration_ms: i64,
    pub provider_latency_ms: i64,
    pub words: Vec<WordTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionApplied {
    #[serde(rename = "type")]
    pub correction_type: String,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub cleaned_text: String,
    #[serde(default)]
    pub corrections_applied: Vec<CorrectionApplied>,
    #[serde(default)]
    pub post_paste_action: PostPasteAction,
    #[serde(default)]
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::AppSettings;

    #[test]
    fn older_settings_pick_up_new_cleanup_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"microphoneName":null,"transcriptionProvider":"deepgram","transcriptionModel":"nova-3","language":"en-US","cleanupEnabled":true,"cleanupEndpoint":"https://api.openai.com/v1","cleanupModel":"gpt-4.1-mini","pasteDelayMs":140,"restoreClipboard":true,"pressEnterEnabled":false,"saveHistory":true,"saveAudio":false,"sessionLimitMinutes":20,"noiseFloorDb":-52.0}"#,
        )
        .unwrap();
        assert_eq!(settings.cleanup_style, "balanced");
        assert!(settings.auto_apply_transform.is_none());
    }
}
