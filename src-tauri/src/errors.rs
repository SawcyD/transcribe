use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::models::AppErrorPayload;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("A dictation is already active")]
    SessionAlreadyActive,
    #[error("There is no active dictation")]
    NoActiveSession,
    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),
    #[error("Microphone error: {0}")]
    Microphone(String),
    #[error("Transcription error: {0}")]
    Transcription(String),
    #[error("Cleanup error: {0}")]
    Cleanup(String),
    #[error("Insertion error: {0}")]
    Insertion(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Credential error: {0}")]
    Credential(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Windows integration error: {0}")]
    Windows(String),
    #[error("The transcription provider returned no speech")]
    EmptyTranscript,
}

impl AppError {
    pub fn payload(&self) -> AppErrorPayload {
        let (category, recoverable) = match self {
            Self::SessionAlreadyActive | Self::InvalidTransition(_) | Self::NoActiveSession => {
                ("state", true)
            }
            Self::Microphone(_) => ("microphone", true),
            Self::Transcription(_) | Self::EmptyTranscript => ("transcription", true),
            Self::Cleanup(_) => ("cleanup", true),
            Self::Insertion(_) => ("insertion", true),
            Self::Database(_) => ("database", true),
            Self::Credential(_) => ("credential", true),
            Self::Configuration(_) => ("configuration", true),
            Self::Windows(_) => ("windows", true),
        };
        AppErrorPayload {
            category: category.into(),
            message: self.to_string(),
            recoverable,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.payload().serialize(serializer)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Database(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Configuration(value.to_string())
    }
}
