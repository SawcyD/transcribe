#[cfg(test)]
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{
    errors::AppError,
    models::{TranscriptionConfig, TranscriptionResult},
};

#[derive(Debug, Clone)]
pub enum ProviderEvent {
    Interim(String),
}

pub struct SessionHandle {
    pub(crate) audio_sender: tokio::sync::mpsc::Sender<super::deepgram::DeepgramInput>,
    pub(crate) worker: tokio::task::JoinHandle<Result<TranscriptionResult, AppError>>,
}

#[async_trait]
pub trait TranscriptionProvider: Send + Sync {
    async fn start_session(&self, config: TranscriptionConfig) -> Result<SessionHandle, AppError>;
    async fn send_audio(&self, session: &SessionHandle, audio: &[u8]) -> Result<(), AppError>;
    async fn finish_session(&self, session: SessionHandle)
        -> Result<TranscriptionResult, AppError>;
}

#[cfg(test)]
pub struct MockTranscriptionProvider {
    result: TranscriptionResult,
    bytes_received: Arc<Mutex<usize>>,
}

#[cfg(test)]
impl MockTranscriptionProvider {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            result: TranscriptionResult {
                raw_text: text.clone(),
                final_segments: vec![text],
                confidence: Some(0.99),
                ..Default::default()
            },
            bytes_received: Arc::new(Mutex::new(0)),
        }
    }

    pub fn bytes_received(&self) -> usize {
        self.bytes_received.lock().map(|value| *value).unwrap_or(0)
    }
}

#[cfg(test)]
#[async_trait]
impl TranscriptionProvider for MockTranscriptionProvider {
    async fn start_session(&self, _config: TranscriptionConfig) -> Result<SessionHandle, AppError> {
        let (audio_sender, mut receiver) = tokio::sync::mpsc::channel(16);
        let result = self.result.clone();
        let received = Arc::clone(&self.bytes_received);
        let worker = tokio::spawn(async move {
            while let Some(input) = receiver.recv().await {
                match input {
                    super::deepgram::DeepgramInput::Audio(bytes) => {
                        if let Ok(mut count) = received.lock() {
                            *count += bytes.len();
                        }
                    }
                    super::deepgram::DeepgramInput::Finish => break,
                }
            }
            Ok(result)
        });
        Ok(SessionHandle {
            audio_sender,
            worker,
        })
    }

    async fn send_audio(&self, session: &SessionHandle, audio: &[u8]) -> Result<(), AppError> {
        session
            .audio_sender
            .send(super::deepgram::DeepgramInput::Audio(audio.to_vec()))
            .await
            .map_err(|_| AppError::Transcription("mock transcription session closed".into()))
    }

    async fn finish_session(
        &self,
        session: SessionHandle,
    ) -> Result<TranscriptionResult, AppError> {
        let _ = session
            .audio_sender
            .send(super::deepgram::DeepgramInput::Finish)
            .await;
        session
            .worker
            .await
            .map_err(|error| AppError::Transcription(error.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_receives_streamed_audio() {
        let provider = MockTranscriptionProvider::new("hello world");
        let config = TranscriptionConfig {
            language: "en-US".into(),
            model: "mock".into(),
            sample_rate: 16_000,
            encoding: "linear16".into(),
            interim_results: true,
            punctuation: true,
            smart_formatting: true,
            dictionary_keyterms: vec![],
            active_application: None,
            developer_mode: false,
            endpointing_ms: 300,
        };
        let session = provider.start_session(config).await.unwrap();
        provider.send_audio(&session, &[1, 2, 3, 4]).await.unwrap();
        let result = provider.finish_session(session).await.unwrap();
        assert_eq!(result.raw_text, "hello world");
        assert_eq!(provider.bytes_received(), 4);
    }
}
