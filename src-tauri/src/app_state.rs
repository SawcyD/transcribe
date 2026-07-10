use chrono::{DateTime, Utc};
use tokio::sync::{Mutex, RwLock};

use crate::{
    audio::{CaptureSession, CapturedAudio},
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
}

impl AppState {
    pub fn new(database: Database, settings: AppSettings) -> Self {
        Self {
            database,
            settings: RwLock::new(settings),
            dictation: Mutex::new(DictationRuntime::default()),
        }
    }
}
