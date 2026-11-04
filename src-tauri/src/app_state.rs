use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, RwLock};

use crate::{
    audio::{CaptureSession, CapturedAudio},
    context::ScreenContext,
    database::Database,
    errors::AppError,
    models::{ActiveTarget, AppSettings, DictationMode, TranscriptionResult},
    state_machine::DictationStateMachine,
};

pub struct ActiveSession {
    pub id: String,
    pub mode: DictationMode,
    pub target: ActiveTarget,
    pub started_at: DateTime<Utc>,
    pub capture: Option<CaptureSession>,
    pub transcription: tokio::task::JoinHandle<Result<TranscriptionResult, AppError>>,
}

pub struct RecoveryAudio {
    pub session_id: String,
    pub audio: CapturedAudio,
}

#[derive(Default)]
pub struct DictationRuntime {
    pub machine: DictationStateMachine,
    pub active: Option<ActiveSession>,
    pub recovery_audio: Option<RecoveryAudio>,
    pub last_target: Option<ActiveTarget>,
}

pub struct AppState {
    pub database: Database,
    pub settings: RwLock<AppSettings>,
    pub dictation: Mutex<DictationRuntime>,
    pub assistant: Mutex<AssistantRuntime>,
    /// Pre-opened transcription socket for the next dictation (see
    /// `transcription::standby`). A plain mutex: it is only swapped in and out.
    pub transcription_standby: std::sync::Mutex<Option<crate::transcription::Standby>>,
    /// The live voice-agent conversation, when one is running.
    pub voice_agent: Mutex<Option<crate::voice_agent::AgentHandle>>,
    /// Sounds currently playing (see `sound`), tagged with a generation so a
    /// finished sound only releases its own device.
    pub speech_out: Mutex<Option<(u64, crate::audio::playback::Player)>>,
    pub tone_out: Mutex<Option<(u64, crate::audio::playback::Player)>>,
    /// Lock-free mirrors of the two window-behaviour settings. Window events are
    /// delivered synchronously on the UI thread, where awaiting `settings` could
    /// deadlock against a task already holding it.
    close_to_tray: AtomicBool,
    minimize_to_tray: AtomicBool,
}

#[derive(Default)]
pub struct AssistantRuntime {
    pub active_request_id: Option<String>,
    pub pending_screen_context: Option<ScreenContext>,
    pub pending_voice_prompt: Option<String>,
}

impl AppState {
    pub fn new(database: Database, settings: AppSettings) -> Self {
        let close_to_tray = AtomicBool::new(settings.close_to_tray);
        let minimize_to_tray = AtomicBool::new(settings.minimize_to_tray);
        Self {
            database,
            settings: RwLock::new(settings),
            dictation: Mutex::new(DictationRuntime::default()),
            assistant: Mutex::new(AssistantRuntime::default()),
            transcription_standby: std::sync::Mutex::new(None),
            voice_agent: Mutex::new(None),
            speech_out: Mutex::new(None),
            tone_out: Mutex::new(None),
            close_to_tray,
            minimize_to_tray,
        }
    }

    /// Keeps the lock-free window-behaviour mirrors in step with saved settings.
    pub fn sync_window_behaviour(&self, settings: &AppSettings) {
        self.close_to_tray
            .store(settings.close_to_tray, Ordering::Relaxed);
        self.minimize_to_tray
            .store(settings.minimize_to_tray, Ordering::Relaxed);
    }

    pub fn take_transcription_standby(&self) -> Option<crate::transcription::Standby> {
        self.transcription_standby.lock().ok().and_then(|mut slot| slot.take())
    }

    /// Replacing an older standby drops it, which closes its socket.
    pub fn set_transcription_standby(&self, standby: Option<crate::transcription::Standby>) {
        if let Ok(mut slot) = self.transcription_standby.lock() {
            *slot = standby;
        }
    }

    pub fn close_to_tray(&self) -> bool {
        self.close_to_tray.load(Ordering::Relaxed)
    }

    pub fn minimize_to_tray(&self) -> bool {
        self.minimize_to_tray.load(Ordering::Relaxed)
    }
}
