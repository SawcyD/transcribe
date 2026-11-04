//! Live spoken conversation with the assistant through Deepgram's Voice Agent
//! API: the mic streams up, Deepgram transcribes, thinks and speaks, and the
//! agent's voice streams back to the speakers. A purple mini dock shows the
//! agent and a blue one shows you (see `src/agent/AgentDock.tsx`).
//!
//! Echo: Deepgram does no echo cancellation, so with speakers the agent would
//! hear itself and reply to its own voice. Unless the user enables
//! interruptions (meant for headphones), the mic is replaced with silence
//! while the agent is talking.

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

use crate::{
    app_state::AppState,
    audio::{playback::Player, CaptureSession},
    errors::AppError,
    security::{self, CredentialKind},
};

const AGENT_URL: &str = "wss://agent.deepgram.com/v1/agent/converse";
const INPUT_RATE: u32 = 16_000;
const OUTPUT_RATE: u32 = 24_000;
/// After the agent's last audio, keep the mic muted briefly so the tail of
/// its voice (still in the speaker/room) isn't heard as the user.
const ECHO_TAIL: Duration = Duration::from_millis(450);

pub struct AgentHandle {
    stop: Option<oneshot::Sender<()>>,
}

impl AgentHandle {
    fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentStateEvent {
    state: &'static str,
    message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentTranscriptEvent {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ServerEvent {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

fn emit_state(app: &AppHandle, state: &'static str, message: Option<String>) {
    let _ = app.emit("agent-state", AgentStateEvent { state, message });
}

/// Shows why the conversation couldn't start, then closes the dock.
fn fail_start(app: &AppHandle, message: String) {
    emit_state(app, "error", Some(message));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(4)).await;
        if !is_running(&app.state::<AppState>()).await {
            hide_dock(&app);
        }
    });
}

pub async fn is_running(state: &AppState) -> bool {
    state.voice_agent.lock().await.is_some()
}

/// Starts the conversation, or ends it if one is already running.
pub async fn toggle(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    if let Some(handle) = state.voice_agent.lock().await.take() {
        handle.stop();
        return Ok(());
    }
    start(app, state).await
}

pub async fn stop(state: &AppState) {
    if let Some(handle) = state.voice_agent.lock().await.take() {
        handle.stop();
    }
}

pub async fn start(app: &AppHandle, state: &AppState) -> Result<(), AppError> {
    let mut slot = state.voice_agent.lock().await;
    if slot.is_some() {
        return Ok(());
    }
    let settings = state.settings.read().await.clone();
    show_dock(app, &settings.overlay_position);
    emit_state(app, "connecting", None);

    let api_key = tokio::task::spawn_blocking(|| security::get(CredentialKind::Deepgram))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))?
        .inspect_err(|error| fail_start(app, error.to_string()))?;

    let memory = if settings.assistant_remember_chats {
        recent_memory(state)
    } else {
        None
    };
    let settings_message = agent_settings(settings.assistant_voice.as_deref(), memory.as_deref());

    let mut request = AGENT_URL
        .into_client_request()
        .map_err(|error| AppError::Transcription(error.to_string()))?;
    request.headers_mut().insert(
        "Authorization",
        format!("Token {api_key}")
            .parse()
            .map_err(|_| AppError::Credential("Deepgram credential contains invalid characters".into()))?,
    );
    let connected = tokio::time::timeout(Duration::from_secs(10), tokio_tungstenite::connect_async(request))
        .await
        .unwrap_or_else(|_| Err(tokio_tungstenite::tungstenite::Error::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "timed out after 10 seconds",
        ))));
    let (mut socket, _) = match connected {
        Ok(value) => value,
        Err(error) => {
            let message = format!("Couldn't reach the Deepgram voice agent: {error}");
            fail_start(app, message.clone());
            return Err(AppError::Transcription(message));
        }
    };
    socket
        .send(Message::Text(settings_message.to_string().into()))
        .await
        .map_err(|error| AppError::Transcription(error.to_string()))?;

    let (capture, microphone) = match CaptureSession::start(
        app.clone(),
        "voice-agent".into(),
        settings.microphone_name.as_deref(),
        settings.noise_floor_db,
    ) {
        Ok(value) => value,
        Err(error) => {
            fail_start(app, error.to_string());
            return Err(error);
        }
    };
    if capture.stream_sample_rate != INPUT_RATE {
        log::warn!(
            "microphone streams at {} Hz; the voice agent expects {INPUT_RATE} Hz",
            capture.stream_sample_rate
        );
    }
    let player = match Player::start(OUTPUT_RATE, settings.output_device_name.as_deref()) {
        Ok(player) => player,
        Err(error) => {
            fail_start(app, error.to_string());
            return Err(error);
        }
    };

    let (stop_sender, stop_receiver) = oneshot::channel();
    *slot = Some(AgentHandle {
        stop: Some(stop_sender),
    });
    drop(slot);

    let app = app.clone();
    let interruptible = settings.voice_agent_interruptible;
    let remember = settings.assistant_remember_chats;
    // The capture and player live in this task and stop when it ends.
    tauri::async_runtime::spawn(async move {
        let result = run(
            &app,
            socket,
            microphone,
            &player,
            stop_receiver,
            interruptible,
            remember,
        )
        .await;
        drop(capture.stop());
        drop(player);
        let state = app.state::<AppState>();
        state.voice_agent.lock().await.take();
        match result {
            Ok(()) => emit_state(&app, "ended", None),
            Err(error) => emit_state(&app, "error", Some(error.to_string())),
        }
        tokio::time::sleep(Duration::from_millis(1_200)).await;
        if !is_running(&state).await {
            hide_dock(&app);
        }
    });
    Ok(())
}

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn run(
    app: &AppHandle,
    mut socket: Socket,
    mut microphone: mpsc::Receiver<Vec<u8>>,
    player: &Player,
    mut stop: oneshot::Receiver<()>,
    interruptible: bool,
    remember: bool,
) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let chat_id = format!("voice-{}", uuid::Uuid::new_v4());
    let mut pending_question: Option<String> = None;
    let mut last_agent_audio: Option<Instant> = None;
    let mut level_tick = tokio::time::interval(Duration::from_millis(50));
    loop {
        tokio::select! {
            _ = &mut stop => {
                let _ = socket.close(None).await;
                return Ok(());
            }
            chunk = microphone.recv() => {
                let Some(chunk) = chunk else { return Ok(()) };
                let agent_talking = player.is_playing()
                    || last_agent_audio.is_some_and(|at| at.elapsed() < ECHO_TAIL);
                let payload = if agent_talking && !interruptible { vec![0u8; chunk.len()] } else { chunk };
                socket
                    .send(Message::Binary(payload.into()))
                    .await
                    .map_err(|error| AppError::Transcription(error.to_string()))?;
            }
            _ = level_tick.tick() => {
                let _ = app.emit("agent-level", player.level());
            }
            message = socket.next() => {
                let Some(message) = message else { return Ok(()) };
                match message.map_err(|error| AppError::Transcription(error.to_string()))? {
                    Message::Binary(bytes) => {
                        player.push_pcm16(&bytes);
                        last_agent_audio = Some(Instant::now());
                    }
                    Message::Text(text) => {
                        let Ok(event) = serde_json::from_str::<ServerEvent>(&text) else { continue };
                        match event.kind.as_str() {
                            "SettingsApplied" => emit_state(app, "listening", None),
                            "UserStartedSpeaking" => {
                                // Barge-in: stop talking when the user starts.
                                player.clear();
                                emit_state(app, "listening", None);
                            }
                            "AgentThinking" => emit_state(app, "thinking", None),
                            "AgentStartedSpeaking" => emit_state(app, "speaking", None),
                            "AgentAudioDone" => emit_state(app, "listening", None),
                            "ConversationText" => {
                                let role = event.role.unwrap_or_default();
                                let content = event.content.unwrap_or_default();
                                if content.trim().is_empty() { continue; }
                                let _ = app.emit("agent-transcript", AgentTranscriptEvent { role: role.clone(), content: content.clone() });
                                if role == "user" {
                                    pending_question = Some(match pending_question.take() {
                                        Some(previous) => format!("{previous} {content}"),
                                        None => content,
                                    });
                                } else if remember {
                                    let question = pending_question.take().unwrap_or_else(|| "(voice conversation)".into());
                                    if let Err(error) = state.database.append_assistant_turn(&chat_id, Some("Voice agent"), &question, &content) {
                                        log::warn!("voice agent turn was not saved: {error}");
                                    }
                                }
                            }
                            "Error" => {
                                return Err(AppError::Transcription(
                                    event.description.or(event.message).unwrap_or_else(|| "The voice agent reported an error".into()),
                                ));
                            }
                            "Warning" => log::warn!("voice agent warning: {text}"),
                            _ => {}
                        }
                    }
                    Message::Close(frame) => {
                        if let Some(frame) = frame {
                            if !frame.reason.is_empty() && frame.code != tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal {
                                return Err(AppError::Transcription(format!("Voice agent closed: {}", frame.reason)));
                            }
                        }
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}

/// The Settings message that configures the conversation.
pub(crate) fn agent_settings(voice: Option<&str>, memory: Option<&str>) -> serde_json::Value {
    let voice = voice.filter(|value| value.starts_with("aura-")).unwrap_or(crate::tts::DEFAULT_VOICE);
    let mut prompt = String::from(
        "You are VoiceFlow's voice assistant, talking with the user out loud. Keep replies short and conversational: one to three sentences, no lists, no Markdown, no code blocks, nothing that only makes sense when read. Ask a short follow-up question when it helps. If you don't know something, say so.",
    );
    if let Some(memory) = memory {
        prompt.push_str("\n\n");
        prompt.push_str(memory);
    }
    serde_json::json!({
        "type": "Settings",
        "audio": {
            "input": { "encoding": "linear16", "sample_rate": INPUT_RATE },
            "output": { "encoding": "linear16", "sample_rate": OUTPUT_RATE, "container": "none" }
        },
        "agent": {
            "language": "en",
            "listen": { "provider": { "type": "deepgram", "model": "nova-3" } },
            "think": {
                "provider": { "type": "open_ai", "model": "gpt-4o-mini", "temperature": 0.7 },
                "prompt": prompt
            },
            "speak": { "provider": { "type": "deepgram", "model": voice } },
            "greeting": "Hey! What's on your mind?"
        }
    })
}

/// A short digest of recent saved chats so the agent can pick up threads
/// ("how did that interview go?"). Voice gives no question to search on up
/// front, so recency stands in for relevance.
fn recent_memory(state: &AppState) -> Option<String> {
    let chats = state.database.list_assistant_chats().ok()?;
    if chats.is_empty() {
        return None;
    }
    let mut lines = String::from("Recent conversations with this user (mention them naturally only if relevant):\n");
    for chat in chats.iter().take(6) {
        let day = chat.updated_at.get(..10).unwrap_or(&chat.updated_at);
        let preview: String = chat.preview.chars().take(140).collect();
        lines.push_str(&format!("- [{day}] {}: {preview}\n", chat.title));
    }
    Some(lines)
}

fn show_dock(app: &AppHandle, overlay_position: &str) {
    let Some(window) = app.get_webview_window("agent") else {
        return;
    };
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|position| app.monitor_from_point(position.x, position.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    if let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) {
        // Its bottom ("you") pill takes the recording dock's spot.
        let area = crate::dock_layout::MonitorArea::of(&monitor);
        let (x, y, _) = crate::dock_layout::beside_dock(area, overlay_position, (size.width, size.height), true);
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }
    let _ = window.set_always_on_top(true);
    let _ = window.show();
}

fn hide_dock(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("agent") {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::agent_settings;

    #[test]
    fn settings_message_matches_the_agent_api_shape() {
        let settings = agent_settings(Some("aura-2-helena-en"), Some("Recent: interview prep"));
        assert_eq!(settings["type"], "Settings");
        assert_eq!(settings["audio"]["input"]["sample_rate"], 16_000);
        assert_eq!(settings["audio"]["output"]["encoding"], "linear16");
        assert_eq!(settings["agent"]["listen"]["provider"]["model"], "nova-3");
        assert_eq!(settings["agent"]["speak"]["provider"]["model"], "aura-2-helena-en");
        assert!(settings["agent"]["think"]["prompt"].as_str().unwrap().contains("interview prep"));
        // Non-Deepgram voices fall back to the default.
        assert_eq!(agent_settings(Some("Microsoft Zira"), None)["agent"]["speak"]["provider"]["model"], "aura-2-thalia-en");
    }
}
