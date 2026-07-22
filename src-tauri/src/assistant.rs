use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    context::ScreenContext,
    errors::AppError,
    security::{self, CredentialKind},
};

const SYSTEM_PROMPT: &str = "You are Buddy, VoiceFlow's concise desktop assistant. Help the user understand the explicitly supplied screen context. Be practical, clear, and direct. Do not claim to have access to anything that was not supplied. Never suggest or perform desktop actions unless the user explicitly asks; this release is read-only.";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantRequest {
    pub prompt: String,
    pub screen_context: Option<ScreenContext>,
    #[serde(default)]
    pub history: Vec<AssistantConversationTurn>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantConversationTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssistantStateEvent {
    request_id: String,
    state: &'static str,
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssistantDeltaEvent {
    request_id: String,
    delta: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    stream: bool,
    temperature: f32,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
struct NonStreamingChatResponse {
    choices: Vec<NonStreamingChoice>,
}

#[derive(Deserialize)]
struct NonStreamingChoice {
    message: NonStreamingMessage,
}

#[derive(Deserialize)]
struct NonStreamingMessage {
    content: String,
}

pub async fn start(
    app: &AppHandle,
    state: &State<'_, AppState>,
    request: AssistantRequest,
) -> Result<String, AppError> {
    let prompt = request.prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > 8_000 {
        return Err(AppError::Configuration(
            "assistant prompt must be between 1 and 8,000 characters".into(),
        ));
    }
    if request
        .screen_context
        .as_ref()
        .is_some_and(|context| context.screenshot_data_url.len() > 8_000_000)
    {
        return Err(AppError::Configuration(
            "captured screen context exceeds the size limit".into(),
        ));
    }
    if request.history.len() > 12
        || request.history.iter().any(|turn| {
            !matches!(turn.role.as_str(), "user" | "assistant")
                || turn.content.trim().is_empty()
                || turn.content.chars().count() > 8_000
        })
    {
        return Err(AppError::Configuration(
            "assistant conversation history is invalid or exceeds its limit".into(),
        ));
    }

    let settings = state.settings.read().await.clone();
    let api_key = tokio::task::spawn_blocking(|| security::get(CredentialKind::Cleanup))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))??;
    let request_id = Uuid::new_v4().to_string();
    state.assistant.lock().await.active_request_id = Some(request_id.clone());

    let app = app.clone();
    let event_request_id = request_id.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.emit(
            "assistant-state",
            AssistantStateEvent {
                request_id: event_request_id.clone(),
                state: "thinking",
                message: None,
            },
        );
        let result = stream_response(
            &app,
            &event_request_id,
            settings.cleanup_endpoint,
            settings.cleanup_model,
            api_key,
            request,
        )
        .await;
        let (state, message) = match result {
            Ok(()) => ("completed", None),
            Err(error) => ("error", Some(error.to_string())),
        };
        let _ = app.emit(
            "assistant-state",
            AssistantStateEvent {
                request_id: event_request_id,
                state,
                message,
            },
        );
    });
    Ok(request_id)
}

pub async fn open_drawer(
    app: &AppHandle,
    state: &AppState,
    screen_context: Option<ScreenContext>,
    voice_prompt: Option<String>,
) -> Result<(), AppError> {
    {
        let mut runtime = state.assistant.lock().await;
        runtime.pending_screen_context = screen_context.clone();
        runtime.pending_voice_prompt = voice_prompt.clone();
    }
    let window = app
        .get_webview_window("assistant")
        .ok_or_else(|| AppError::Windows("assistant window is unavailable".into()))?;
    position_above_buddy(app, &window);
    window
        .set_always_on_top(true)
        .map_err(|error| AppError::Windows(error.to_string()))?;
    window
        .show()
        .map_err(|error| AppError::Windows(error.to_string()))?;
    window
        .set_focus()
        .map_err(|error| AppError::Windows(error.to_string()))?;
    if let Some(screen_context) = screen_context {
        app.emit("assistant-screen-context", screen_context)
            .map_err(|error| AppError::Windows(error.to_string()))?;
    }
    if let Some(voice_prompt) = voice_prompt {
        app.emit("assistant-voice-prompt", voice_prompt)
            .map_err(|error| AppError::Windows(error.to_string()))?;
    }
    Ok(())
}

fn position_above_buddy(app: &AppHandle, assistant: &tauri::WebviewWindow) {
    let Some(buddy) = app.get_webview_window("buddy") else {
        return;
    };
    let Ok(buddy_position) = buddy.outer_position() else {
        return;
    };
    let Ok(buddy_size) = buddy.outer_size() else {
        return;
    };
    let Ok(assistant_size) = assistant.outer_size() else {
        return;
    };
    let (left, top) = assistant
        .current_monitor()
        .ok()
        .flatten()
        .map(|monitor| {
            (
                monitor.work_area().position.x + 8,
                monitor.work_area().position.y + 8,
            )
        })
        .unwrap_or((0, 0));
    let x = (buddy_position.x + buddy_size.width as i32 - assistant_size.width as i32).max(left);
    let y = (buddy_position.y - assistant_size.height as i32 - 14).max(top);
    let _ = assistant.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
        x, y,
    )));
}

async fn stream_response(
    app: &AppHandle,
    request_id: &str,
    endpoint: String,
    model: String,
    api_key: String,
    request: AssistantRequest,
) -> Result<(), AppError> {
    let mut user_content = vec![serde_json::json!({ "type": "text", "text": request.prompt })];
    if let Some(context) = request.screen_context {
        let target = [context.application, context.window_title]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" — ");
        if !target.is_empty() {
            user_content.push(
                serde_json::json!({ "type": "text", "text": format!("Active context: {target}") }),
            );
        }
        user_content.push(serde_json::json!({
            "type": "image_url",
            "image_url": { "url": context.screenshot_data_url, "detail": "low" }
        }));
    }
    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: serde_json::Value::String(SYSTEM_PROMPT.into()),
    }];
    messages.extend(request.history.into_iter().map(|turn| ChatMessage {
        role: turn.role,
        content: serde_json::Value::String(turn.content),
    }));
    messages.push(ChatMessage {
        role: "user".into(),
        content: serde_json::Value::Array(user_content),
    });
    let body = ChatRequest {
        model,
        stream: true,
        temperature: 0.2,
        messages,
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    let response = client
        .post(format!(
            "{}/chat/completions",
            endpoint.trim_end_matches('/')
        ))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| AppError::Configuration(format!("assistant request failed: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Configuration(match status.as_u16() {
            401 | 403 => "assistant provider rejected the API key".into(),
            429 => "assistant provider rate limit reached".into(),
            code => format!("assistant provider returned HTTP {code}"),
        }));
    }
    let streams_sse = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/event-stream"));

    let _ = app.emit(
        "assistant-state",
        AssistantStateEvent {
            request_id: request_id.to_string(),
            state: "streaming",
            message: None,
        },
    );
    if !streams_sse {
        let body: NonStreamingChatResponse = response.json().await.map_err(|error| {
            AppError::Configuration(format!(
                "assistant provider returned an invalid response: {error}"
            ))
        })?;
        let content = body
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|content| !content.is_empty())
            .ok_or_else(|| AppError::Configuration("assistant provider returned no text".into()))?;
        let _ = app.emit(
            "assistant-delta",
            AssistantDeltaEvent {
                request_id: request_id.to_string(),
                delta: content,
            },
        );
        return Ok(());
    }
    let mut buffer = String::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            AppError::Configuration(format!("assistant stream interrupted: {error}"))
        })?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(index) = buffer.find('\n') {
            let line = buffer[..index].trim_end_matches('\r').to_string();
            buffer.drain(..=index);
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Ok(());
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(delta) = value
                        .pointer("/choices/0/delta/content")
                        .and_then(|value| value.as_str())
                    {
                        let _ = app.emit(
                            "assistant-delta",
                            AssistantDeltaEvent {
                                request_id: request_id.to_string(),
                                delta: delta.to_string(),
                            },
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
