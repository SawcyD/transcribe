use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    app_state::AppState,
    assistant::{self, AssistantRequest},
    audio,
    context::ScreenContext,
    dictation,
    errors::AppError,
    insertion,
    models::{
        AppSettings, CredentialStatus, DashboardStats, DictationMode, DictationSnapshot,
        DictionaryEntry, DictionaryEntryInput, TranscriptRecord,
    },
    security::{self, CredentialKind},
    transforms::{self, TransformRequest, TransformResponse},
};

#[tauri::command]
pub async fn get_dictation_snapshot(
    state: State<'_, AppState>,
) -> Result<DictationSnapshot, AppError> {
    Ok(state.dictation.lock().await.machine.snapshot())
}

#[tauri::command]
pub async fn start_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: DictationMode,
) -> Result<DictationSnapshot, AppError> {
    dictation::start(&app, &state, mode).await
}

#[tauri::command]
pub async fn finish_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DictationSnapshot, AppError> {
    dictation::finish(&app, &state).await
}

#[tauri::command]
pub async fn cancel_dictation(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DictationSnapshot, AppError> {
    dictation::cancel(&app, &state).await
}

#[tauri::command]
pub fn list_transcripts(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<TranscriptRecord>, AppError> {
    state.database.list_transcripts(&query)
}

#[tauri::command]
pub fn get_transcript(
    state: State<'_, AppState>,
    id: String,
) -> Result<TranscriptRecord, AppError> {
    state.database.get_transcript(&id)
}

#[tauri::command]
pub fn delete_transcript(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state.database.delete_transcript(&id)
}

#[tauri::command]
pub fn set_transcript_favorite(
    state: State<'_, AppState>,
    id: String,
    favorite: bool,
) -> Result<(), AppError> {
    state.database.set_transcript_favorite(&id, favorite)
}

/// Reads text aloud with the chosen Deepgram voice on the chosen output device.
#[tauri::command]
pub async fn speak_text(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<(), AppError> {
    let voice = state.settings.read().await.assistant_voice.clone();
    let pcm = crate::tts::speak(&text, voice.as_deref()).await?;
    crate::sound::play(&app, crate::sound::Channel::Speech, pcm, crate::sound::RATE).await
}

#[tauri::command]
pub async fn stop_speaking(app: AppHandle) -> Result<(), AppError> {
    crate::sound::stop(&app, crate::sound::Channel::Speech).await;
    Ok(())
}

/// Plays a UI tone ("start", "stop", "success", "error", "test").
#[tauri::command]
pub async fn play_tone(app: AppHandle, name: String) -> Result<(), AppError> {
    let pcm = crate::sound::named_tone(&name)
        .ok_or_else(|| AppError::Configuration(format!("unknown tone '{name}'")))?;
    crate::sound::play(&app, crate::sound::Channel::Tone, pcm, crate::sound::RATE).await
}

/// Plays the test chime on `device` (what the picker shows, saved or not).
#[tauri::command]
pub async fn test_output_device(app: AppHandle, device: Option<String>) -> Result<(), AppError> {
    let pcm = crate::sound::named_tone("test").unwrap_or_default();
    crate::sound::play_on(&app, crate::sound::Channel::Tone, pcm, crate::sound::RATE, device).await
}

#[tauri::command]
pub fn list_output_devices() -> Result<Vec<String>, AppError> {
    audio::list_output_devices()
}

#[tauri::command]
pub async fn start_voice_agent(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    crate::voice_agent::start(&app, &state).await
}

/// Longest typed message passed to the agent. Longer pastes are cut, not refused.
const MAX_TYPED_CHARS: usize = 12_000;

/// Sends text typed or pasted into the voice-agent dock, with any pasted
/// images (data: URLs). The agent only takes text, so each image is first
/// described by the assistant's model and the description goes along.
#[tauri::command]
pub async fn agent_send_message(app: AppHandle, state: State<'_, AppState>, text: String, images: Vec<String>) -> Result<(), AppError> {
    let text = text.trim().to_string();
    if text.is_empty() && images.is_empty() {
        return Err(AppError::Configuration("there is nothing to send".into()));
    }
    if images.len() > 4 || images.iter().any(|image| !image.starts_with("data:image/") || image.len() > 8_000_000) {
        return Err(AppError::Configuration("send up to 4 images, each under 6 MB".into()));
    }
    if !crate::voice_agent::is_running(&state).await {
        return Err(AppError::Configuration("there is no conversation to send this to".into()));
    }
    let mut message = text.clone();
    if !images.is_empty() {
        let _ = app.emit("agent-state", serde_json::json!({ "state": "thinking", "message": null }));
        let settings = state.settings.read().await.clone();
        let instruction = format!(
            "The user shared this image in a conversation with their voice assistant, with this message: \"{text}\". Describe it in detail for someone who can't see it, quoting any text or code exactly."
        );
        for (index, image) in images.iter().enumerate() {
            let description = crate::assistant::describe_image(&settings, image, &instruction).await?;
            message.push_str(&format!("\n\n[The user shared image {}. What it shows: {description}]", index + 1));
        }
    }
    let message: String = message.trim().chars().take(MAX_TYPED_CHARS).collect();
    crate::voice_agent::send_typed(&state, message).await
}

#[tauri::command]
pub async fn stop_voice_agent(state: State<'_, AppState>) -> Result<(), AppError> {
    crate::voice_agent::stop(&state).await;
    Ok(())
}

#[tauri::command]
pub fn list_assistant_chats(
    state: State<'_, AppState>,
) -> Result<Vec<crate::database::AssistantChatSummary>, AppError> {
    state.database.list_assistant_chats()
}

#[tauri::command]
pub fn get_assistant_chat(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<crate::database::AssistantChatTurn>, AppError> {
    state.database.get_assistant_chat(&id)
}

#[tauri::command]
pub fn delete_assistant_chat(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state.database.delete_assistant_chat(&id)
}

#[tauri::command]
pub fn list_dictionary_entries(
    state: State<'_, AppState>,
) -> Result<Vec<DictionaryEntry>, AppError> {
    state.database.list_dictionary_entries()
}

#[tauri::command]
pub fn save_dictionary_entry(
    state: State<'_, AppState>,
    id: Option<String>,
    entry: DictionaryEntryInput,
) -> Result<DictionaryEntry, AppError> {
    state
        .database
        .upsert_dictionary_entry(id.as_deref(), &entry)
}

#[tauri::command]
pub fn delete_dictionary_entry(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state.database.delete_dictionary_entry(&id)
}

#[tauri::command]
pub fn dashboard_stats(state: State<'_, AppState>) -> Result<DashboardStats, AppError> {
    state.database.dashboard_stats()
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, AppError> {
    settings.validate().map_err(AppError::Configuration)?;
    state.database.save_settings(&settings)?;
    state.sync_window_behaviour(&settings);
    // Rebinding takes effect immediately; the hook thread keeps running.
    crate::shortcuts::update_bindings(&settings.shortcuts);
    *state.settings.write().await = settings.clone();
    if !settings.keep_transcription_warm {
        state.set_transcription_standby(None);
    }
    Ok(settings)
}

#[tauri::command]
pub async fn set_provider_credential(provider: String, secret: String) -> Result<(), AppError> {
    let kind = CredentialKind::parse(&provider)?;
    tokio::task::spawn_blocking(move || security::set(kind, &secret))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))?
}

#[tauri::command]
pub async fn delete_provider_credential(provider: String) -> Result<(), AppError> {
    let kind = CredentialKind::parse(&provider)?;
    tokio::task::spawn_blocking(move || security::delete(kind))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))?
}

#[tauri::command]
pub async fn credential_status() -> Result<CredentialStatus, AppError> {
    tokio::task::spawn_blocking(|| CredentialStatus {
        deepgram: security::exists(CredentialKind::Deepgram),
        cleanup: security::exists(CredentialKind::Cleanup),
        assistant: security::exists(CredentialKind::Assistant),
        search: security::exists(CredentialKind::Search),
    })
    .await
    .map_err(|error| AppError::Credential(error.to_string()))
}

#[tauri::command]
pub fn list_microphones() -> Result<Vec<String>, AppError> {
    audio::list_input_devices()
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<(), AppError> {
    if text.chars().count() > 100_000 {
        return Err(AppError::Insertion(
            "clipboard text exceeds the 100,000 character limit".into(),
        ));
    }
    insertion::copy_text(&text)
}

#[tauri::command]
pub async fn paste_transcript(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let record = state.database.get_transcript(&id)?;
    paste_record_text(&state, record.final_transcript).await
}

/// Pastes the newest stored transcript. Backs the tray's "paste latest" action,
/// which has no transcript id to work from.
#[tauri::command]
pub async fn paste_latest_transcript(state: State<'_, AppState>) -> Result<(), AppError> {
    let record = state
        .database
        .list_transcripts("")?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Insertion("no transcript has been recorded yet".into()))?;
    paste_record_text(&state, record.final_transcript).await
}

/// Shared insertion path: restore the remembered target, paste, and fall back to
/// the clipboard so the text is never lost when insertion fails.
pub async fn paste_record_text(state: &AppState, text: String) -> Result<(), AppError> {
    let settings = state.settings.read().await.clone();
    let target = state
        .dictation
        .lock()
        .await
        .last_target
        .clone()
        .or_else(|| insertion::capture_active_target().ok())
        .ok_or_else(|| AppError::Insertion("no previous text target is available".into()))?;
    let fallback_text = text.clone();
    let result = tokio::task::spawn_blocking(move || {
        insertion::paste_into_target(
            &text,
            &target,
            crate::models::PostPasteAction::None,
            settings.paste_delay_ms,
            settings.restore_clipboard,
        )
    })
    .await
    .map_err(|error| AppError::Insertion(error.to_string()))?;
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = insertion::copy_text(&fallback_text);
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn transform_text(
    state: State<'_, AppState>,
    request: TransformRequest,
) -> Result<TransformResponse, AppError> {
    let settings = state.settings.read().await.clone();
    transforms::apply_transform(request, &settings).await
}

#[tauri::command]
pub async fn capture_screen_context(app: AppHandle) -> Result<ScreenContext, AppError> {
    // The button lives in VoiceFlow's own window, so capturing "the window in
    // front" would always capture VoiceFlow. Step aside briefly so the app the
    // user was working in comes back to the front, capture it, then return.
    let main = app.get_webview_window("main");
    if let Some(window) = &main {
        let _ = window.minimize();
    }
    tokio::time::sleep(std::time::Duration::from_millis(350)).await;
    let captured = tokio::task::spawn_blocking(crate::context::capture_context)
        .await
        .map_err(|error| AppError::Windows(error.to_string()))?;
    if let Some(window) = &main {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    captured
}

/// Opens the shared whiteboard window (see `src/whiteboard/Whiteboard.tsx`).
#[tauri::command]
pub fn show_whiteboard(app: AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("whiteboard")
        .ok_or_else(|| AppError::Windows("whiteboard window is unavailable".into()))?;
    let _ = window.unminimize();
    let _ = window.show();
    window
        .set_focus()
        .map_err(|error| AppError::Windows(error.to_string()))
}

/// Sends one request from the voice agent to the whiteboard window ("mermaid",
/// "add", "read", "erase" or "clear", with the tool's JSON arguments) and
/// returns the board's answer in words the agent can act on.
pub async fn ask_whiteboard(app: &AppHandle, action: &str, arguments: &str) -> String {
    use tauri::Listener;
    // Reading shouldn't pop the window up; anything that draws should.
    if action != "read" {
        if let Err(error) = show_whiteboard(app.clone()) {
            return format!("Couldn't open the whiteboard: {error}");
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let (sender, receiver) = tokio::sync::oneshot::channel::<String>();
    let sender = std::sync::Mutex::new(Some(sender));
    let wanted = id.clone();
    let listener = app.listen("whiteboard-result", move |event| {
        let Ok(reply) = serde_json::from_str::<serde_json::Value>(event.payload()) else { return };
        if reply["id"].as_str() != Some(wanted.as_str()) {
            return;
        }
        if let Some(sender) = sender.lock().ok().and_then(|mut slot| slot.take()) {
            let _ = sender.send(reply["text"].as_str().unwrap_or_default().to_owned());
        }
    });
    let _ = app.emit("whiteboard-request", serde_json::json!({ "id": id, "action": action, "arguments": arguments }));
    let reply = tokio::time::timeout(std::time::Duration::from_secs(8), receiver).await;
    app.unlisten(listener);
    match reply {
        Ok(Ok(text)) => text,
        _ => "The whiteboard did not answer in time.".into(),
    }
}

/// The file a named whiteboard is saved in. Names are restricted so a name can
/// never point outside the whiteboards folder.
fn whiteboard_path(app: &AppHandle, name: &str) -> Result<std::path::PathBuf, AppError> {
    let name = name.trim();
    let valid = !name.is_empty()
        && name.chars().count() <= 60
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'));
    if !valid {
        return Err(AppError::Configuration("whiteboard names can use letters, numbers, spaces, - and _".into()));
    }
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Configuration(error.to_string()))?
        .join("whiteboards");
    std::fs::create_dir_all(&folder).map_err(|error| AppError::Configuration(error.to_string()))?;
    Ok(folder.join(format!("{name}.json")))
}

/// Opens the user's flow.md in their default editor, creating it if needed.
#[tauri::command]
pub fn open_flow_profile(app: AppHandle) -> Result<(), AppError> {
    let file = crate::profile::path(&app)?;
    tauri_plugin_opener::open_path(&file, None::<&str>).map_err(|error| AppError::Windows(error.to_string()))
}

#[tauri::command]
pub fn list_whiteboards(app: AppHandle) -> Result<Vec<String>, AppError> {
    let folder = whiteboard_path(&app, "x")?.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .map_err(|error| AppError::Configuration(error.to_string()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
        .filter_map(|path| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    Ok(names)
}

#[tauri::command]
pub fn load_whiteboard(app: AppHandle, name: String) -> Result<Option<String>, AppError> {
    match std::fs::read_to_string(whiteboard_path(&app, &name)?) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::Configuration(error.to_string())),
    }
}

#[tauri::command]
pub fn save_whiteboard(app: AppHandle, name: String, content: String) -> Result<(), AppError> {
    // Pasted images are stored inside the file, so allow for a few of them.
    if content.len() > 60_000_000 {
        return Err(AppError::Configuration("this whiteboard is too large to save".into()));
    }
    let path = whiteboard_path(&app, &name)?;
    // Write beside the file, then swap, so a crash mid-save can't leave half a board.
    let partial = path.with_extension("json.part");
    std::fs::write(&partial, content)
        .and_then(|()| std::fs::rename(&partial, &path))
        .map_err(|error| AppError::Configuration(error.to_string()))
}

/// Brings the main window forward on the History page. Used by the overlay's
/// "open transcript" affordance after a failed insertion.
#[tauri::command]
pub fn show_history(app: AppHandle) -> Result<(), AppError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Windows("main window is unavailable".into()))?;
    let _ = window.show();
    let _ = window.set_focus();
    app.emit("navigate", "/history")
        .map_err(|error| AppError::Windows(error.to_string()))
}

/// Opens the folder holding the log files or the SQLite database, for support.
#[tauri::command]
pub fn open_data_folder(app: AppHandle, target: String) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;
    let directory = match target.as_str() {
        "logs" => app
            .path()
            .app_log_dir()
            .map_err(|error| AppError::Configuration(error.to_string()))?,
        "database" => app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::Configuration(error.to_string()))?,
        _ => return Err(AppError::Configuration("unknown data folder".into())),
    };
    std::fs::create_dir_all(&directory)?;
    app.opener()
        .open_path(directory.to_string_lossy(), None::<&str>)
        .map_err(|error| AppError::Windows(error.to_string()))
}

/// Builds a diagnostic summary. Deliberately excludes transcript text and every
/// credential — it is meant to be pasteable into a bug report.
#[tauri::command]
pub async fn diagnostic_report(state: State<'_, AppState>) -> Result<String, AppError> {
    let settings = state.settings.read().await.clone();
    let credentials = credential_status().await?;
    let transcripts = state.database.list_transcripts("")?.len();
    Ok(format!(
        "VoiceFlow diagnostic report\n\
         version: {}\n\
         transcription: {} {}\n\
         cleanup: enabled={} model={} style={}\n\
         credentials: deepgram={} cleanup={} assistant={}\n\
         audio: device={} noise_floor={}dB session_limit={}min\n\
         shortcuts: ptt={} hands_free={} command={} cancel={}\n\
         history: stored={} retention_days={} max_entries={}\n\
         overlay: enabled={} position={} opacity={}%\n",
        env!("CARGO_PKG_VERSION"),
        settings.transcription_provider,
        settings.transcription_model,
        settings.cleanup_enabled,
        settings.cleanup_model,
        settings.cleanup_style,
        credentials.deepgram,
        credentials.cleanup,
        credentials.assistant,
        settings.microphone_name.as_deref().unwrap_or("system default"),
        settings.noise_floor_db,
        settings.session_limit_minutes,
        settings.shortcuts.push_to_talk.display(),
        settings.shortcuts.hands_free.display(),
        settings.shortcuts.command_mode.display(),
        settings.shortcuts.cancel.display(),
        transcripts,
        settings.history_retention_days,
        settings.max_history_entries,
        settings.show_overlay,
        settings.overlay_position,
        settings.overlay_opacity,
    ))
}

/// Clears stored transcripts. Credentials and settings are left untouched.
#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<usize, AppError> {
    state.database.clear_transcripts()
}

#[tauri::command]
pub async fn open_assistant_drawer(
    app: AppHandle,
    state: State<'_, AppState>,
    screen_context: Option<ScreenContext>,
) -> Result<(), AppError> {
    assistant::open_drawer(&app, &state, screen_context, None).await
}

#[tauri::command]
pub async fn get_pending_assistant_context(
    state: State<'_, AppState>,
) -> Result<Option<ScreenContext>, AppError> {
    // Taken, not cloned: a reopened bubble must not replay an old request.
    Ok(state.assistant.lock().await.pending_screen_context.take())
}

#[tauri::command]
pub async fn get_pending_assistant_voice_prompt(
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    Ok(state.assistant.lock().await.pending_voice_prompt.take())
}

#[tauri::command]
pub async fn ask_assistant(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AssistantRequest,
) -> Result<String, AppError> {
    assistant::start(&app, &state, request).await
}
