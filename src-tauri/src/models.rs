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
    Call,
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

/// Maps a process name to the cleanup style used while it is focused.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCleanupStyle {
    /// Executable name, matched case-insensitively, e.g. "code.exe".
    pub process_name: String,
    pub style: String,
}

impl AppCleanupStyle {
    /// Seeds the mapping table with the applications VoiceFlow previously
    /// hardcoded, so the behaviour is now visible and editable.
    pub fn defaults() -> Vec<Self> {
        [
            ("code.exe", "developer"),
            ("cursor.exe", "developer"),
            ("robloxstudiobeta.exe", "developer"),
            ("windowsterminal.exe", "code_literal"),
            ("powershell.exe", "code_literal"),
            ("pwsh.exe", "code_literal"),
            ("cmd.exe", "code_literal"),
            ("discord.exe", "casual"),
        ]
        .into_iter()
        .map(|(process_name, style)| Self {
            process_name: process_name.into(),
            style: style.into(),
        })
        .collect()
    }
}

pub const CLEANUP_STYLES: [&str; 4] = ["balanced", "casual", "developer", "code_literal"];

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
    pub call_mode_application: String,
    pub call_mode_output_device_name: Option<String>,
    pub theme: String,
    pub buddy_stroll_enabled: bool,
    pub buddy_speak_responses: bool,
    pub assistant_endpoint: String,
    pub assistant_model: String,
    /// Hide to the tray instead of exiting when the main window is closed.
    pub shortcuts: crate::shortcuts::ShortcutBindings,
    /// Mode used by the Home and tray "start dictation" affordances.
    pub default_mode: DictationMode,
    pub auto_detect_developer_apps: bool,
    /// Per-process cleanup style overrides, consulted before the built-in list.
    pub app_cleanup_styles: Vec<AppCleanupStyle>,
    pub remove_filler_words: bool,
    pub remove_false_starts: bool,
    pub backtracking_enabled: bool,
    pub spoken_formatting_enabled: bool,
    pub voice_actions_enabled: bool,
    pub show_overlay: bool,
    pub show_waveform: bool,
    pub play_tones: bool,
    pub overlay_position: String,
    /// Overlay opacity as a percentage, 40–100.
    pub overlay_opacity: u32,
    pub buddy_enabled: bool,
    pub buddy_show_at_startup: bool,
    pub buddy_size: String,
    pub buddy_always_on_top: bool,
    pub assistant_allow_screen_context: bool,
    pub assistant_voice: Option<String>,
    pub store_raw_transcript: bool,
    pub store_normalized_transcript: bool,
    pub store_cleaned_transcript: bool,
    pub include_transcript_in_logs: bool,
    /// Days of history to keep. `0` means keep everything.
    pub history_retention_days: u32,
    /// Maximum stored transcripts. `0` means unlimited.
    pub max_history_entries: u32,
    pub confirm_paste_again: bool,
    pub debug_logging: bool,
    pub close_to_tray: bool,
    /// Hide from the taskbar when the main window is minimised.
    pub minimize_to_tray: bool,
    pub show_notifications: bool,
    /// Navigation route restored on the next launch.
    pub last_page: String,
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
            call_mode_application: "Discord".into(),
            call_mode_output_device_name: None,
            theme: "system".into(),
            buddy_stroll_enabled: false,
            buddy_speak_responses: false,
            assistant_endpoint: "https://api.openai.com/v1".into(),
            assistant_model: "gpt-4.1-mini".into(),
            shortcuts: crate::shortcuts::ShortcutBindings::default(),
            default_mode: DictationMode::PushToTalk,
            auto_detect_developer_apps: true,
            app_cleanup_styles: AppCleanupStyle::defaults(),
            remove_filler_words: true,
            remove_false_starts: true,
            backtracking_enabled: true,
            spoken_formatting_enabled: true,
            voice_actions_enabled: true,
            show_overlay: true,
            show_waveform: true,
            play_tones: true,
            overlay_position: "bottom_center".into(),
            overlay_opacity: 90,
            buddy_enabled: true,
            buddy_show_at_startup: true,
            buddy_size: "medium".into(),
            buddy_always_on_top: true,
            assistant_allow_screen_context: true,
            assistant_voice: None,
            store_raw_transcript: true,
            store_normalized_transcript: true,
            store_cleaned_transcript: true,
            include_transcript_in_logs: false,
            history_retention_days: 0,
            max_history_entries: 0,
            confirm_paste_again: true,
            debug_logging: false,
            close_to_tray: true,
            minimize_to_tray: false,
            show_notifications: true,
            last_page: "/".into(),
        }
    }
}

impl AppSettings {
    /// Resolves the cleanup style for the focused application.
    ///
    /// An explicit mapping always wins. Otherwise the configured default is
    /// used, and app detection only upgrades "balanced" so a deliberate style
    /// choice is never silently overridden.
    pub fn cleanup_style_for(&self, process: Option<&str>) -> String {
        if self.auto_detect_developer_apps {
            if let Some(process) = process {
                if let Some(mapping) = self
                    .app_cleanup_styles
                    .iter()
                    .find(|mapping| mapping.process_name.eq_ignore_ascii_case(process))
                {
                    return mapping.style.clone();
                }
            }
        }
        self.cleanup_style.clone()
    }

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
        let assistant_endpoint = url::Url::parse(&self.assistant_endpoint)
            .map_err(|_| "assistant endpoint is not a valid URL")?;
        let assistant_local = matches!(
            assistant_endpoint.host_str(),
            Some("localhost" | "127.0.0.1")
        );
        if assistant_endpoint.scheme() != "https" && !assistant_local {
            return Err("assistant endpoint must use HTTPS unless it is local".into());
        }
        if self.transcription_model.trim().is_empty()
            || self.cleanup_model.trim().is_empty()
            || self.assistant_model.trim().is_empty()
        {
            return Err("provider model names cannot be empty".into());
        }
        if !CLEANUP_STYLES.contains(&self.cleanup_style.as_str()) {
            return Err(
                "cleanup style must be balanced, casual, developer, or code_literal".into(),
            );
        }
        for mapping in &self.app_cleanup_styles {
            if mapping.process_name.trim().is_empty() {
                return Err("application cleanup mappings need a process name".into());
            }
            if !CLEANUP_STYLES.contains(&mapping.style.as_str()) {
                return Err(format!(
                    "{} is mapped to an unknown cleanup style",
                    mapping.process_name
                ));
            }
        }
        if !(40..=100).contains(&self.overlay_opacity) {
            return Err("overlay opacity must be between 40 and 100 percent".into());
        }
        if !matches!(
            self.overlay_position.as_str(),
            "bottom_center" | "bottom_right" | "top_center" | "top_right"
        ) {
            return Err("overlay position is not a supported anchor".into());
        }
        if !matches!(self.buddy_size.as_str(), "small" | "medium" | "large") {
            return Err("buddy size must be small, medium, or large".into());
        }
        if !matches!(self.theme.as_str(), "system" | "light" | "dark") {
            return Err("theme must be system, light, or dark".into());
        }
        if let Some(conflict) = self.shortcuts.conflict() {
            return Err(conflict);
        }
        if self.history_retention_days > 3_650 {
            return Err("history retention must be 3650 days or fewer".into());
        }
        // Restored on launch and handed straight to the router, so it must stay a
        // known in-app route rather than arbitrary caller-supplied text.
        if !matches!(
            self.last_page.as_str(),
            "/" | "/dictation"
                | "/transforms"
                | "/dictionary"
                | "/history"
                | "/assistant-settings"
                | "/settings"
                | "/about"
        ) {
            return Err("last page must be a known VoiceFlow route".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub deepgram: bool,
    pub cleanup: bool,
    pub assistant: bool,
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
        assert!(!settings.buddy_stroll_enabled);
        assert!(!settings.buddy_speak_responses);
        assert_eq!(settings.assistant_model, "gpt-4.1-mini");
        assert_eq!(settings.theme, "system");
    }
}
