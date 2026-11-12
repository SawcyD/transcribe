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

/// Deepgram's own default Flux voice.
pub const DEFAULT_VOICE: &str = "flux-kit-en";
const AGENT_URL: &str = "wss://agent.deepgram.com/v1/agent/converse";
const INPUT_RATE: u32 = 16_000;
const OUTPUT_RATE: u32 = 24_000;
/// After the agent's last audio, keep the mic muted briefly so the tail of
/// its voice (still in the speaker/room) isn't heard as the user.
const ECHO_TAIL: Duration = Duration::from_millis(450);
/// A sound must last this long before the agent hears it. Claps, snaps and key
/// clacks are over sooner; the start of a word is held back, not lost.
const GATE_OPEN_SAMPLES: usize = INPUT_RATE as usize * 150 / 1000;
/// Once open, the gate stays open through pauses this long.
const GATE_CLOSE_SAMPLES: usize = INPUT_RATE as usize * 400 / 1000;
/// How far above the noise floor counts as a sound.
const GATE_MARGIN_DB: f32 = 10.0;

/// Passes the mic through only for sustained sound, so a clap or snap doesn't
/// register as the user speaking (and cut the agent off).
// ponytail: duration-only gate; a long non-speech noise (a cough, music) still
// gets through. Swap in a real speech detector (e.g. Silero VAD) if it matters.
pub(crate) struct SpeechGate {
    threshold: f32,
    held: Vec<Vec<u8>>,
    loud: usize,
    quiet: usize,
    open: bool,
}

impl SpeechGate {
    pub(crate) fn new(noise_floor_db: f32) -> Self {
        Self {
            threshold: 10f32.powf((noise_floor_db + GATE_MARGIN_DB) / 20.0),
            held: Vec::new(),
            loud: 0,
            quiet: 0,
            open: false,
        }
    }

    /// Takes one PCM16 chunk and returns the chunks to send now, in order.
    pub(crate) fn push(&mut self, chunk: Vec<u8>) -> Vec<Vec<u8>> {
        let samples: Vec<i16> = chunk
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let is_loud = crate::audio::audio_level::rms(&samples) > self.threshold;
        if self.open {
            if is_loud {
                self.quiet = 0;
            } else {
                self.quiet += samples.len();
                if self.quiet >= GATE_CLOSE_SAMPLES {
                    self.open = false;
                    self.loud = 0;
                }
            }
            return vec![chunk];
        }
        if is_loud {
            self.loud += samples.len();
            self.held.push(chunk);
            if self.loud < GATE_OPEN_SAMPLES {
                return Vec::new();
            }
            self.open = true;
            self.quiet = 0;
            return std::mem::take(&mut self.held);
        }
        // It went quiet before lasting long enough: what was held was a
        // transient. Send silence of the same length to keep the timeline.
        self.loud = 0;
        let mut out: Vec<Vec<u8>> = self.held.drain(..).map(|held| vec![0u8; held.len()]).collect();
        out.push(chunk);
        out
    }
}

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
    #[serde(default)]
    functions: Vec<FunctionCall>,
}

/// One entry of a FunctionCallRequest: the agent asking us to run a tool.
#[derive(Deserialize)]
struct FunctionCall {
    id: String,
    name: String,
    /// The arguments as a JSON string.
    #[serde(default)]
    arguments: String,
    #[serde(default = "yes")]
    client_side: bool,
}

fn yes() -> bool {
    true
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
    let can_search = tokio::task::spawn_blocking(|| security::exists(CredentialKind::Search))
        .await
        .unwrap_or(false);
    let profile = crate::profile::prompt_section(app);
    let settings_message = agent_settings(&AgentSetup {
        voice: &settings.voice_agent_voice,
        expressivity: settings.voice_agent_expressivity,
        personality: &settings.voice_agent_personality,
        profile: profile.as_deref(),
        memory: memory.as_deref(),
        can_see_screen: settings.assistant_allow_screen_context,
        can_act: settings.voice_agent_can_act,
        can_search,
    });

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
    // The capture and player live in this task and stop when it ends.
    tauri::async_runtime::spawn(async move {
        let result = run(
            &app,
            socket,
            microphone,
            &player,
            stop_receiver,
            settings,
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
    settings: crate::models::AppSettings,
) -> Result<(), AppError> {
    let interruptible = settings.voice_agent_interruptible;
    let remember = settings.assistant_remember_chats;
    let mut gate = SpeechGate::new(settings.noise_floor_db);
    // Tool calls (looking at the screen) take seconds, so they run on the
    // side and hand their answer back here: (id, name, content).
    let (tool_done, mut tool_results) = mpsc::unbounded_channel::<(String, String, String)>();
    let state = app.state::<AppState>();
    let mut transcript = VoiceTranscript {
        database: state.database.clone(),
        chat_id: format!("voice-{}", uuid::Uuid::new_v4()),
        remember,
        question: String::new(),
        answer: String::new(),
    };
    let mut last_agent_audio: Option<Instant> = None;
    // Set when the user said goodbye: the call ends once the farewell has
    // played, or at this time if it never comes.
    let mut hang_up_by: Option<Instant> = None;
    let mut level_tick = tokio::time::interval(Duration::from_millis(33));
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
                for chunk in gate.push(chunk) {
                    let payload = if agent_talking && !interruptible { vec![0u8; chunk.len()] } else { chunk };
                    socket
                        .send(Message::Binary(payload.into()))
                        .await
                        .map_err(|error| AppError::Transcription(error.to_string()))?;
                }
            }
            Some((id, name, content)) = tool_results.recv() => {
                let reply = serde_json::json!({ "type": "FunctionCallResponse", "id": id, "name": name, "content": content });
                socket
                    .send(Message::Text(reply.to_string().into()))
                    .await
                    .map_err(|error| AppError::Transcription(error.to_string()))?;
            }
            _ = level_tick.tick() => {
                let _ = app.emit("agent-level", player.level());
                if hang_up_by.is_some_and(|by| Instant::now() > by) {
                    let _ = socket.close(None).await;
                    return Ok(());
                }
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
                            "FunctionCallRequest" => {
                                for call in event.functions.into_iter().filter(|call| call.client_side) {
                                    if call.name == "end_conversation" {
                                        hang_up_by = Some(Instant::now() + Duration::from_secs(10));
                                        let reply = serde_json::json!({
                                            "type": "FunctionCallResponse",
                                            "id": call.id,
                                            "name": call.name,
                                            "content": "Say a short goodbye now, a few words in your personality. The call ends right after it."
                                        });
                                        socket
                                            .send(Message::Text(reply.to_string().into()))
                                            .await
                                            .map_err(|error| AppError::Transcription(error.to_string()))?;
                                        continue;
                                    }
                                    emit_state(app, "thinking", None);
                                    let (app, settings, tool_done) = (app.clone(), settings.clone(), tool_done.clone());
                                    tauri::async_runtime::spawn(async move {
                                        let content = run_tool(&app, &settings, &call.name, &call.arguments).await;
                                        let _ = tool_done.send((call.id, call.name, content));
                                    });
                                }
                            }
                            "AgentStartedSpeaking" => emit_state(app, "speaking", None),
                            "AgentAudioDone" if hang_up_by.is_some() => {
                                // Let the goodbye finish playing, then hang up.
                                let until = Instant::now() + Duration::from_secs(6);
                                while player.is_playing() && Instant::now() < until {
                                    tokio::time::sleep(Duration::from_millis(50)).await;
                                }
                                let _ = socket.close(None).await;
                                return Ok(());
                            }
                            "AgentAudioDone" => emit_state(app, "listening", None),
                            "ConversationText" => {
                                let role = event.role.unwrap_or_default();
                                let content = event.content.unwrap_or_default();
                                if content.trim().is_empty() { continue; }
                                let _ = app.emit("agent-transcript", AgentTranscriptEvent { role: role.clone(), content: content.clone() });
                                transcript.add(&role, &content);
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

/// Saves a voice conversation as chat turns: one per exchange, joining the
/// several lines the agent may speak in a row. The greeting, which answers
/// nothing, isn't saved. Whatever is pending is saved when this is dropped,
/// however the call ended.
struct VoiceTranscript {
    database: crate::database::Database,
    chat_id: String,
    remember: bool,
    question: String,
    answer: String,
}

impl VoiceTranscript {
    fn add(&mut self, role: &str, content: &str) {
        // The user speaking after the agent has answered starts a new exchange.
        if role == "user" && !self.answer.is_empty() {
            self.flush();
        }
        let side = if role == "user" { &mut self.question } else { &mut self.answer };
        if !side.is_empty() {
            side.push(' ');
        }
        side.push_str(content.trim());
    }

    fn flush(&mut self) {
        let (question, answer) = (std::mem::take(&mut self.question), std::mem::take(&mut self.answer));
        if !self.remember || question.is_empty() || answer.is_empty() {
            return;
        }
        if let Err(error) = self.database.append_assistant_turn(&self.chat_id, Some("Voice agent"), &question, &answer) {
            log::warn!("voice agent turn was not saved: {error}");
        }
    }
}

impl Drop for VoiceTranscript {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Runs one tool for the agent. Failures go back as text so it can tell the
/// user what went wrong instead of going silent.
async fn run_tool(app: &AppHandle, settings: &crate::models::AppSettings, name: &str, arguments: &str) -> String {
    let raw = arguments;
    let arguments: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let argument = |key: &str| arguments[key].as_str().unwrap_or_default().to_owned();
    match name {
        "look_at_screen" if settings.assistant_allow_screen_context => {
            let question = Some(argument("question")).filter(|question| !question.is_empty());
            match crate::assistant::describe_screen(settings, question.as_deref().unwrap_or("What is on the screen?")).await {
                Ok(description) => description,
                Err(error) => format!("Couldn't look at the screen: {error}"),
            }
        }
        "open_app" if settings.voice_agent_can_act => crate::launcher::open_app(&argument("name")),
        "open_website" if settings.voice_agent_can_act => crate::launcher::open_url(&argument("url")),
        "draw_on_whiteboard" => crate::commands::ask_whiteboard(app, "mermaid", raw).await,
        "add_to_whiteboard" => crate::commands::ask_whiteboard(app, "add", raw).await,
        "read_whiteboard" => crate::commands::ask_whiteboard(app, "read", raw).await,
        "erase_from_whiteboard" => crate::commands::ask_whiteboard(app, "erase", raw).await,
        "clear_whiteboard" => crate::commands::ask_whiteboard(app, "clear", raw).await,
        "search_web" => crate::launcher::search_web(&argument("query")).await,
        "remember" => crate::profile::remember(app, &argument("fact")),
        _ => "That tool is not available.".into(),
    }
}

/// The Settings message that configures the conversation.
/// Everything that shapes a conversation, from settings and the user's files.
pub(crate) struct AgentSetup<'a> {
    pub voice: &'a str,
    pub expressivity: i32,
    pub personality: &'a str,
    /// The user's flow.md, already formatted as a prompt section.
    pub profile: Option<&'a str>,
    /// Recent saved chats, already formatted as a prompt section.
    pub memory: Option<&'a str>,
    pub can_see_screen: bool,
    pub can_act: bool,
    pub can_search: bool,
}

pub(crate) fn agent_settings(setup: &AgentSetup) -> serde_json::Value {
    let AgentSetup { voice, expressivity, personality, profile, memory, can_see_screen, can_act, can_search } = *setup;
    let personality = crate::personality::get(personality);
    let voice = if voice.starts_with("flux-") { voice } else { DEFAULT_VOICE };
    let mut prompt = String::from(
        "Your name is Flow. You are VoiceFlow's voice assistant, talking with the user out loud. Keep replies short and conversational: one to three sentences, no lists, no Markdown, no code blocks, nothing that only makes sense when read. Everything you write is spoken aloud, so never write stage directions, actions or sound effects such as *sigh*, (laughs) or [pause]: they would be read out as words. Show feeling through what you say instead, like \"Ugh. Fine.\" or \"Ha, nice.\" When the user says goodbye or wants to stop, call end_conversation. Ask a short follow-up question when it helps. If you don't know something, say so. Match the user's energy: upbeat when they're excited, gentle when they're down.

If the user asks for a mock interview, interview prep or interview mode, become the interviewer. First ask what role and company it's for and which kind they want (behavioral, technical, or both) unless they already said. With no preference, start with behavioral questions. Ask exactly one question at a time and wait for the full answer. Follow up on vague answers the way a real interviewer would. For technical questions, state the problem clearly, let them think out loud, and give a small hint only if they are stuck or ask. Don't give feedback after each answer unless they ask. Stay in the interviewer role until they say they're done, then give brief, honest feedback: what was strong, what to improve, and one concrete thing to practice.",
    );
    if can_see_screen {
        prompt.push_str("\n\nYou can see the user's screen with the look_at_screen tool. Use it when they ask about what they're working on or what's on screen. In a technical interview, use it to read their code when they say they're done, ask for a hint, or ask you to check it. Say a few words first, like \"let me take a look\", because it takes a few seconds.");
    }
    if can_act {
        prompt.push_str("\n\nYou can open apps on the user's computer with open_app and web pages with open_website. Use them when asked, then confirm in a few words. You cannot do anything else on the computer, so say so if asked.");
    }
    prompt.push_str("\n\nYou share a whiteboard with the user and can use all of it. For flowcharts use draw_on_whiteboard with Mermaid flowchart code: short plain labels, no quotes or brackets inside a label. For everything else use add_to_whiteboard: shapes, text, arrows, lines, frames and web embeds, in any colour, size and stroke. To write an explanation, add a rectangle with a label so the text wraps, and give it a soft background colour. Call read_whiteboard first when you need to know what is there, to place something next to the user's work, or to get ids for erase_from_whiteboard. read_whiteboard cannot see inside freehand drawings or pasted images; look at the screen for those if you can. After drawing, say in one sentence what you added. If a drawing is rejected, simplify it and try once more.");
    if can_search {
        prompt.push_str("\n\nYou can search the web with search_web. Use it for current facts and best practices, then say what you found in a sentence or two. When the user is planning on the whiteboard, also add the key points there, each with its source link.");
    }
    prompt.push_str("\n\nWhen the user asks you to remember something about them, or tells you a lasting preference, save it with remember so you know it next time. Keep each one to a short sentence.");
    for section in [profile, memory].into_iter().flatten() {
        prompt.push_str("\n\n");
        prompt.push_str(section);
    }
    // Last, so the chosen personality sets the tone over the defaults above.
    if !personality.tone.is_empty() {
        prompt.push_str("\n\n");
        prompt.push_str(personality.tone);
    }
    let mut settings = serde_json::json!({
        "type": "Settings",
        "audio": {
            "input": { "encoding": "linear16", "sample_rate": INPUT_RATE },
            "output": { "encoding": "linear16", "sample_rate": OUTPUT_RATE, "container": "none" }
        },
        "agent": {
            "language": "en",
            "listen": { "provider": { "type": "deepgram", "model": "nova-3" } },
            "think": {
                // Not a reasoning model: OpenAI rejects tools combined with reasoning (gpt-5.x) on this API.
                "provider": { "type": "open_ai", "model": "gpt-4.1", "temperature": 0.7 },
                "prompt": prompt
            },
            // Flux voices need version v2 and raw linear16 output (set above).
            "speak": { "provider": { "type": "deepgram", "version": "v2", "model": voice, "expressivity": expressivity.clamp(-2, 2) } },
            "greeting": personality.greeting
        }
    });
    let string = |description: &str| serde_json::json!({ "type": "string", "description": description });
    let tool = |name: &str, description: &str, argument: &str, argument_description: &str| {
        serde_json::json!({
            "name": name,
            "description": description,
            "parameters": { "type": "object", "properties": { argument: string(argument_description) }, "required": [argument] }
        })
    };
    let mut tools = Vec::new();
    if can_see_screen {
        tools.push(tool("look_at_screen", "Look at the user's screen and get a description of what is on it, including any code.", "question", "What to look for on the screen"));
    }
    if can_act {
        tools.push(tool("open_app", "Open an app installed on the user's computer.", "name", "The app's full official name, for example \"Visual Studio Code\" or \"OBS Studio\""));
        tools.push(tool("open_website", "Open a web page in the user's browser.", "url", "The full address, starting with https://"));
    }
    tools.push(tool("draw_on_whiteboard", "Draw a flowchart on the whiteboard from Mermaid code. It is placed next to what is already there.", "mermaid", "Mermaid flowchart code, for example: flowchart TD\\n  A[Idea] --> B{Worth it}\\n  B -->|Yes| C[Build it]"));
    tools.push(serde_json::json!({
        "name": "add_to_whiteboard",
        "description": "Add shapes, text, arrows, lines, frames or web embeds to the whiteboard. Each element is an object with type (rectangle, ellipse, diamond, text, arrow, line, frame or embeddable), x and y, and usually width and height. Optional on any element: id (unique on the board), strokeColor and backgroundColor (hex colours), fillStyle (solid, hachure, cross-hatch), strokeWidth (1, 2 or 4), strokeStyle (solid, dashed, dotted), roughness (0 neat, 1 hand-drawn, 2 sketchy), roundness ({\"type\": 3} for rounded corners), opacity (0-100), link (a web address). Shapes and arrows take label: {\"text\": \"...\"}. Text takes text and fontSize. Arrows and lines connect shapes with start: {\"id\": \"...\"} and end: {\"id\": \"...\"}, or follow any path with points: [[0,0],[dx,dy],...], which is how to sketch freehand. Frames take name and children (a list of ids). Embeddable takes link.",
        "parameters": {
            "type": "object",
            "properties": {
                "elements": { "type": "array", "items": { "type": "object" }, "description": "The elements to add" },
                "position": { "type": "string", "enum": ["free_space", "as_given"], "description": "free_space (default) moves the group to empty space beside existing content; as_given keeps your x and y, for placing things relative to what read_whiteboard returned" }
            },
            "required": ["elements"]
        }
    }));
    tools.push(serde_json::json!({
        "name": "end_conversation",
        "description": "Hang up. Use when the user says goodbye or clearly wants to stop, for example \"bye\", \"that's all\", \"thanks, I'm done\" or \"talk later\". Not when they only pause or thank you mid-conversation.",
        "parameters": { "type": "object", "properties": {} }
    }));
    tools.push(tool("remember", "Save a short fact or preference about the user to their flow.md profile, so you know it in future conversations.", "fact", "One sentence, for example: Prefers TypeScript and Rust for code"));
    tools.push(serde_json::json!({
        "name": "read_whiteboard",
        "description": "Get everything on the whiteboard as a list: id, type, position, size, text, colour and link of each element.",
        "parameters": { "type": "object", "properties": {} }
    }));
    tools.push(serde_json::json!({
        "name": "erase_from_whiteboard",
        "description": "Erase specific elements from the whiteboard by id. Get ids from read_whiteboard.",
        "parameters": {
            "type": "object",
            "properties": { "ids": { "type": "array", "items": { "type": "string" }, "description": "Ids of the elements to erase" } },
            "required": ["ids"]
        }
    }));
    tools.push(serde_json::json!({
        "name": "clear_whiteboard",
        "description": "Erase everything on the whiteboard. Only when the user asks.",
        "parameters": { "type": "object", "properties": {} }
    }));
    if can_search {
        tools.push(tool("search_web", "Search the web. Returns a short summary and the top results with their links.", "query", "What to search for"));
    }
    settings["agent"]["think"]["functions"] = tools.into();
    settings
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
    // Placed over the recording dock's spot the first time only: after that it
    // stays wherever the user dragged it.
    static PLACED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let first = !PLACED.swap(true, std::sync::atomic::Ordering::Relaxed);
    if let (true, Some(monitor), Ok(size)) = (first, monitor, window.outer_size()) {
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
    use super::{agent_settings, AgentSetup, SpeechGate, GATE_OPEN_SAMPLES};

    fn setup(voice: &str) -> AgentSetup<'_> {
        AgentSetup {
            voice,
            expressivity: 0,
            personality: "friendly",
            profile: None,
            memory: None,
            can_see_screen: false,
            can_act: false,
            can_search: false,
        }
    }

    fn chunk(value: i16) -> Vec<u8> {
        // 10 ms at 16 kHz.
        std::iter::repeat(value.to_le_bytes()).take(160).flatten().collect()
    }

    #[test]
    fn gate_silences_a_clap_but_keeps_all_of_sustained_speech() {
        let mut gate = SpeechGate::new(-52.0);
        // A 50 ms burst, then quiet: everything that comes out is silence.
        let mut out = Vec::new();
        for _ in 0..5 {
            out.extend(gate.push(chunk(20_000)));
        }
        assert!(out.is_empty(), "a burst is held until it proves sustained");
        out.extend(gate.push(chunk(0)));
        assert_eq!(out.len(), 6);
        assert!(out.iter().flatten().all(|byte| *byte == 0));

        // Sustained sound: released from its very first chunk once it has lasted.
        let needed = GATE_OPEN_SAMPLES / 160;
        let mut out = Vec::new();
        for _ in 0..needed + 3 {
            out.extend(gate.push(chunk(8_000)));
        }
        assert_eq!(out.len(), needed + 3);
        assert!(out.iter().all(|sent| *sent == chunk(8_000)));
        // A short pause mid-sentence passes straight through.
        assert_eq!(gate.push(chunk(0)).len(), 1);
    }

    #[test]
    fn settings_message_matches_the_agent_api_shape() {
        let settings = agent_settings(&AgentSetup {
            expressivity: 5,
            personality: "smug",
            profile: Some("- Name: Sam"),
            memory: Some("Recent: interview prep"),
            can_see_screen: true,
            can_act: true,
            can_search: true,
            ..setup("flux-cliff-en")
        });
        assert_eq!(settings["type"], "Settings");
        assert_eq!(settings["audio"]["input"]["sample_rate"], 16_000);
        assert_eq!(settings["audio"]["output"]["encoding"], "linear16");
        assert_eq!(settings["agent"]["listen"]["provider"]["model"], "nova-3");
        assert_eq!(settings["agent"]["speak"]["provider"]["model"], "flux-cliff-en");
        assert_eq!(settings["agent"]["speak"]["provider"]["expressivity"], 2);
        let prompt = settings["agent"]["think"]["prompt"].as_str().unwrap();
        assert!(prompt.contains("interview prep"));
        assert!(prompt.contains("- Name: Sam"));
        // The personality comes last, so it sets the tone.
        assert!(prompt.trim_end().ends_with("be kind."));
        assert_eq!(settings["agent"]["greeting"], "Oh, you again. What is it, boss?");
        assert_eq!(settings["agent"]["think"]["functions"][0]["name"], "look_at_screen");
        assert_eq!(settings["agent"]["think"]["functions"][1]["name"], "open_app");
        assert_eq!(settings["agent"]["think"]["functions"][1]["parameters"]["required"][0], "name");
        // With screen and actions off, only the whiteboard tools are offered.
        let functions = &agent_settings(&setup("flux-kit-en"))["agent"]["think"]["functions"];
        assert_eq!(functions.as_array().map(Vec::len), Some(7));
        assert!(!functions.to_string().contains("search_web"));
        assert!(settings["agent"]["think"]["functions"].to_string().contains("search_web"));
        assert_eq!(functions[0]["name"], "draw_on_whiteboard");
        // Anything that isn't a Flux voice falls back to the default.
        assert_eq!(agent_settings(&setup("Microsoft Zira"))["agent"]["speak"]["provider"]["model"], "flux-kit-en");
    }
}
