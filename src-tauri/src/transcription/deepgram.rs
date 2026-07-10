use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};

use crate::{
    errors::AppError,
    models::{TranscriptionConfig, TranscriptionResult, WordTiming},
};

use super::provider::{ProviderEvent, SessionHandle, TranscriptionProvider};

pub(crate) enum DeepgramInput {
    Audio(Vec<u8>),
    Finish,
}

pub struct DeepgramProvider {
    api_key: String,
    event_sender: mpsc::UnboundedSender<ProviderEvent>,
}

impl DeepgramProvider {
    pub fn new(api_key: String, event_sender: mpsc::UnboundedSender<ProviderEvent>) -> Self {
        Self {
            api_key,
            event_sender,
        }
    }
}

#[async_trait]
impl TranscriptionProvider for DeepgramProvider {
    async fn start_session(&self, config: TranscriptionConfig) -> Result<SessionHandle, AppError> {
        let mut url = url::Url::parse("wss://api.deepgram.com/v1/listen")
            .expect("Deepgram endpoint is static and valid");
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("model", &config.model)
                .append_pair("language", &config.language)
                .append_pair("encoding", &config.encoding)
                .append_pair("sample_rate", &config.sample_rate.to_string())
                .append_pair("channels", "1")
                .append_pair("interim_results", bool_str(config.interim_results))
                .append_pair("punctuate", bool_str(config.punctuation))
                .append_pair("smart_format", bool_str(config.smart_formatting))
                .append_pair("endpointing", &config.endpointing_ms.to_string());
            for keyterm in config.dictionary_keyterms.iter().take(100) {
                query.append_pair("keyterm", keyterm);
            }
            if config.developer_mode {
                query
                    .append_pair("keyterm", "camelCase")
                    .append_pair("keyterm", "PascalCase");
            }
            if let Some(application) = config.active_application.as_deref() {
                query.append_pair("tag", application);
            }
        }
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|error| AppError::Transcription(error.to_string()))?;
        let authorization = format!("Token {}", self.api_key).parse().map_err(|_| {
            AppError::Credential("Deepgram credential contains invalid header characters".into())
        })?;
        request.headers_mut().insert("Authorization", authorization);
        let (socket, _) = connect_async(request).await.map_err(map_websocket_error)?;
        let (audio_sender, mut audio_receiver) = mpsc::channel::<DeepgramInput>(256);
        let event_sender = self.event_sender.clone();
        let worker = tokio::spawn(async move {
            let started = Instant::now();
            let mut socket = socket;
            let mut accumulator = Accumulator::default();
            let mut finishing = false;
            loop {
                if finishing {
                    match tokio::time::timeout(Duration::from_secs(10), socket.next()).await {
                        Ok(Some(message)) => {
                            if handle_message(
                                message.map_err(map_websocket_error)?,
                                &mut accumulator,
                                &event_sender,
                            )? {
                                break;
                            }
                        }
                        Ok(None) => break,
                        Err(_) => {
                            return Err(AppError::Transcription(
                                "Deepgram did not finalize within 10 seconds".into(),
                            ))
                        }
                    }
                    continue;
                }
                tokio::select! {
                    input = audio_receiver.recv() => match input {
                        Some(DeepgramInput::Audio(bytes)) => socket.send(Message::Binary(bytes.into())).await.map_err(map_websocket_error)?,
                        Some(DeepgramInput::Finish) | None => {
                            socket.send(Message::Text(r#"{"type":"CloseStream"}"#.into())).await.map_err(map_websocket_error)?;
                            finishing = true;
                        }
                    },
                    message = socket.next() => match message {
                        Some(message) => if handle_message(message.map_err(map_websocket_error)?, &mut accumulator, &event_sender)? { break; },
                        None => break,
                    }
                }
            }
            Ok(accumulator.finish(started.elapsed()))
        });
        Ok(SessionHandle {
            audio_sender,
            worker,
        })
    }

    async fn send_audio(&self, session: &SessionHandle, audio: &[u8]) -> Result<(), AppError> {
        session
            .audio_sender
            .send(DeepgramInput::Audio(audio.to_vec()))
            .await
            .map_err(|_| {
                AppError::Transcription(
                    "Deepgram streaming session closed before audio was sent".into(),
                )
            })
    }

    async fn finish_session(
        &self,
        session: SessionHandle,
    ) -> Result<TranscriptionResult, AppError> {
        let _ = session.audio_sender.send(DeepgramInput::Finish).await;
        session
            .worker
            .await
            .map_err(|error| AppError::Transcription(error.to_string()))?
    }
}

fn bool_str(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn map_websocket_error(error: tokio_tungstenite::tungstenite::Error) -> AppError {
    AppError::Transcription(match error {
        tokio_tungstenite::tungstenite::Error::Http(response)
            if response.status().as_u16() == 401 =>
        {
            "Deepgram rejected the API key".into()
        }
        tokio_tungstenite::tungstenite::Error::Http(response)
            if response.status().as_u16() == 429 =>
        {
            "Deepgram rate limit reached".into()
        }
        other => other.to_string(),
    })
}

fn handle_message(
    message: Message,
    accumulator: &mut Accumulator,
    event_sender: &mpsc::UnboundedSender<ProviderEvent>,
) -> Result<bool, AppError> {
    match message {
        Message::Text(text) => {
            let response: DeepgramResponse = serde_json::from_str(&text).map_err(|error| {
                AppError::Transcription(format!("Deepgram returned invalid JSON: {error}"))
            })?;
            match response.message_type.as_str() {
                "Results" => {
                    if let Some(channel) = response.channel {
                        if let Some(alternative) = channel.alternatives.into_iter().next() {
                            let transcript = alternative.transcript.trim().to_string();
                            if response.is_final.unwrap_or(false) {
                                if !transcript.is_empty() {
                                    accumulator.final_segments.push(transcript);
                                }
                                accumulator
                                    .words
                                    .extend(alternative.words.into_iter().map(Into::into));
                                accumulator.confidences.push(alternative.confidence);
                                accumulator.detected_language = alternative
                                    .languages
                                    .into_iter()
                                    .next()
                                    .or(accumulator.detected_language.take());
                                accumulator.audio_duration = accumulator.audio_duration.max(
                                    response.start.unwrap_or(0.0)
                                        + response.duration.unwrap_or(0.0),
                                );
                            } else if !transcript.is_empty() {
                                accumulator.interim_segments.push(transcript.clone());
                                let _ = event_sender.send(ProviderEvent::Interim(transcript));
                            }
                        }
                    }
                }
                "Metadata" => return Ok(true),
                "Error" => {
                    return Err(AppError::Transcription(
                        response
                            .description
                            .unwrap_or_else(|| "Deepgram reported an unknown error".into()),
                    ))
                }
                _ => {}
            }
        }
        Message::Close(_) => return Ok(true),
        _ => {}
    }
    Ok(false)
}

#[derive(Default)]
struct Accumulator {
    final_segments: Vec<String>,
    interim_segments: Vec<String>,
    confidences: Vec<f32>,
    detected_language: Option<String>,
    audio_duration: f32,
    words: Vec<WordTiming>,
}

impl Accumulator {
    fn finish(self, latency: Duration) -> TranscriptionResult {
        let confidence = if self.confidences.is_empty() {
            None
        } else {
            Some(self.confidences.iter().sum::<f32>() / self.confidences.len() as f32)
        };
        TranscriptionResult {
            raw_text: self.final_segments.join(" ").trim().to_string(),
            final_segments: self.final_segments,
            interim_segments: self.interim_segments,
            confidence,
            detected_language: self.detected_language,
            duration_ms: (self.audio_duration * 1000.0) as i64,
            provider_latency_ms: latency.as_millis() as i64,
            words: self.words,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    #[serde(rename = "type", default)]
    message_type: String,
    channel: Option<DeepgramChannel>,
    is_final: Option<bool>,
    start: Option<f32>,
    duration: Option<f32>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepgramChannel {
    #[serde(default)]
    alternatives: Vec<DeepgramAlternative>,
}

#[derive(Debug, Deserialize)]
struct DeepgramAlternative {
    #[serde(default)]
    transcript: String,
    #[serde(default)]
    confidence: f32,
    #[serde(default)]
    words: Vec<DeepgramWord>,
    #[serde(default)]
    languages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DeepgramWord {
    word: String,
    start: f32,
    end: f32,
    #[serde(default)]
    confidence: f32,
}

impl From<DeepgramWord> for WordTiming {
    fn from(value: DeepgramWord) -> Self {
        Self {
            word: value.word,
            start: value.start,
            end: value.end,
            confidence: value.confidence,
        }
    }
}
