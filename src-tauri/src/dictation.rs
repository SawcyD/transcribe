use std::{path::PathBuf, time::Instant};

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position};
use tokio::sync::mpsc;

use crate::{
    app_state::{ActiveSession, AppState, RecoveryAudio},
    assistant,
    audio::{self, CapturedAudio},
    cleanup::{
        backtracking::{resolve_explicit_backtracking, resolve_with_options, BacktrackingOptions},
        dictionary::apply_dictionary,
        voice_actions::extract_trailing_action, CleanupProvider, OpenAiCompatibleCleanup,
    },
    errors::AppError,
    insertion,
    models::{
        AppErrorPayload, CleanupResult, DictationMode, DictationSnapshot, DictationState,
        DictionaryCategory, InsertionStatus, PostPasteAction, TranscriptRecord,
        TranscriptionConfig,
    },
    security::{self, CredentialKind},
    transcription::{DeepgramProvider, ProviderEvent, TranscriptionProvider},
    transforms::{self, TransformRequest},
};

pub async fn start(
    app: &AppHandle,
    state: &AppState,
    mode: DictationMode,
) -> Result<DictationSnapshot, AppError> {
    let target = insertion::capture_active_target()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let settings = state.settings.read().await.clone();
    let started_at = Utc::now();
    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.begin(session_id.clone(), mode)?;
    }
    emit_snapshot(app, state).await;
    if let Err(error) = show_overlay(app, &settings) {
        set_error(app, state, error.payload()).await;
        return Err(error);
    }

    // Call Mode intentionally captures an explicitly configured *input* route.
    // On Windows this can be a hardware loopback (Stereo Mix) or a virtual cable
    // receiving audio from Discord/Teams; normal dictation keeps the microphone.
    let capture_device = if mode == DictationMode::Call {
        settings
            .call_mode_output_device_name
            .as_deref()
            .or(settings.microphone_name.as_deref())
    } else {
        settings.microphone_name.as_deref()
    };
    let capture_result = audio::CaptureSession::start(
        app.clone(),
        session_id.clone(),
        capture_device,
        settings.noise_floor_db,
    );
    let (capture, receiver) = match capture_result {
        Ok(value) => value,
        Err(error) => {
            set_error(app, state, error.payload()).await;
            return Err(error);
        }
    };
    let keyterms = match state.database.dictionary_keyterms() {
        Ok(value) => value,
        Err(error) => {
            let _ = capture.stop();
            set_error(app, state, error.payload()).await;
            return Err(error);
        }
    };
    let config = TranscriptionConfig {
        language: settings.language.clone(),
        model: settings.transcription_model.clone(),
        sample_rate: capture.format.sample_rate,
        encoding: "linear16".into(),
        interim_results: true,
        punctuation: true,
        smart_formatting: true,
        dictionary_keyterms: keyterms,
        active_application: target.process_name.clone(),
        // A code-oriented cleanup style is the signal that this app wants the
        // developer transcription profile too.
        developer_mode: matches!(
            settings
                .cleanup_style_for(target.process_name.as_deref())
                .as_str(),
            "developer" | "code_literal"
        ),
        endpointing_ms: 300,
    };
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    spawn_interim_forwarder(app.clone(), session_id.clone(), event_receiver);
    let transcription = tokio::spawn(stream_transcription(receiver, config, event_sender));

    {
        let mut runtime = state.dictation.lock().await;
        let effective_mode = runtime.machine.snapshot().mode.unwrap_or(mode);
        let listening_state = match effective_mode {
            DictationMode::HandsFree | DictationMode::Call => DictationState::ListeningHandsFree,
            _ => DictationState::ListeningPushToTalk,
        };
        runtime.active = Some(ActiveSession {
            id: session_id,
            mode: effective_mode,
            target,
            started_at,
            capture: Some(capture),
            transcription,
        });
        runtime.machine.transition(listening_state)?;
    }
    emit_snapshot(app, state).await;
    if let Some(active_session_id) = state.dictation.lock().await.machine.snapshot().session_id {
        schedule_session_limit(
            app.clone(),
            active_session_id,
            settings.session_limit_minutes,
        );
    }
    Ok(state.dictation.lock().await.machine.snapshot())
}

fn schedule_session_limit(app: AppHandle, session_id: String, minutes: u32) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(minutes.max(1) as u64 * 60)).await;
        let state = app.state::<AppState>();
        let matches = state
            .dictation
            .lock()
            .await
            .machine
            .snapshot()
            .session_id
            .as_deref()
            == Some(session_id.as_str());
        if matches {
            log::info!("session limit reached; session_id={session_id}");
            if let Err(error) = finish(&app, &state).await {
                log::warn!(
                    "session limit finish failed; category={}",
                    error.payload().category
                );
            }
        }
    });
}

pub async fn finish(app: &AppHandle, state: &AppState) -> Result<DictationSnapshot, AppError> {
    let processing_started = Instant::now();
    let mut active = {
        let mut runtime = state.dictation.lock().await;
        runtime
            .machine
            .transition(DictationState::FinalizingAudio)?;
        runtime.active.take().ok_or(AppError::NoActiveSession)?
    };
    state.dictation.lock().await.last_target = Some(active.target.clone());
    emit_snapshot(app, state).await;
    let capture = active
        .capture
        .take()
        .ok_or_else(|| AppError::Microphone("active capture stream was missing".into()))?;
    let captured = tokio::task::spawn_blocking(move || capture.stop())
        .await
        .map_err(|error| AppError::Microphone(error.to_string()))?;
    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.transition(DictationState::Transcribing)?;
    }
    emit_snapshot(app, state).await;

    let transcription = match active.transcription.await {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            retain_failed_audio(state, &active.id, captured).await;
            set_error(app, state, error.payload()).await;
            return Err(error);
        }
        Err(error) => {
            let error = AppError::Transcription(error.to_string());
            retain_failed_audio(state, &active.id, captured).await;
            set_error(app, state, error.payload()).await;
            return Err(error);
        }
    };
    log::info!(
        "session transcription complete; session_id={} duration_ms={} provider_latency_ms={} final_segments={} interim_segments={} words={} detected_language={}",
        active.id,
        transcription.duration_ms,
        transcription.provider_latency_ms,
        transcription.final_segments.len(),
        transcription.interim_segments.len(),
        transcription.words.len(),
        transcription.detected_language.as_deref().unwrap_or("unknown"),
    );
    if transcription.raw_text.trim().is_empty() {
        retain_failed_audio(state, &active.id, captured).await;
        let error = AppError::EmptyTranscript;
        set_error(app, state, error.payload()).await;
        return Err(error);
    }

    if active.mode == DictationMode::Command {
        let prompt = finish_sentence(&resolve_explicit_backtracking(
            transcription.raw_text.trim(),
        ));
        let screen_context = if command_needs_screen_context(&prompt) {
            match crate::context::capture_context() {
                Ok(context) => Some(context),
                Err(error) => {
                    log::debug!("Buddy screen context was unavailable: {error}");
                    None
                }
            }
        } else {
            None
        };
        {
            let mut runtime = state.dictation.lock().await;
            runtime.machine.transition(DictationState::Cleaning)?;
            runtime.machine.transition(DictationState::Inserting)?;
            runtime.machine.transition(DictationState::Completed)?;
        }
        emit_snapshot(app, state).await;
        hide_overlay(app);
        {
            let mut runtime = state.dictation.lock().await;
            runtime.machine.transition(DictationState::Idle)?;
            runtime.machine.reset();
        }
        emit_snapshot(app, state).await;
        assistant::open_drawer(app, state, screen_context, Some(prompt)).await?;
        return Ok(state.dictation.lock().await.machine.snapshot());
    }

    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.transition(DictationState::Cleaning)?;
    }
    emit_snapshot(app, state).await;
    let settings = state.settings.read().await.clone();
    let entries = state.database.enabled_dictionary_entries()?;
    let raw = transcription.raw_text.trim().to_string();
    let normalized = apply_dictionary(&raw, &entries)?;
    let structured = if settings.spoken_formatting_enabled {
        normalize_spoken_structure(&normalized)
    } else {
        normalized.clone()
    };
    let deterministic = finish_sentence(&resolve_with_options(
        &structured,
        BacktrackingOptions {
            backtracking: settings.backtracking_enabled,
            remove_filler_words: settings.remove_filler_words,
            remove_false_starts: settings.remove_false_starts,
        },
    ));
    let voice_action = if settings.voice_actions_enabled {
        extract_trailing_action(&deterministic, settings.press_enter_enabled)
    } else {
        crate::cleanup::voice_actions::VoiceActionResult::none(&deterministic)
    };
    if voice_action.cancel_requested {
        return finish_as_cancelled(app, state).await;
    }
    let protected_terms = entries
        .iter()
        .filter(|entry| entry.category == DictionaryCategory::ProtectedIdentifier)
        .map(|entry| entry.display_term.clone())
        .collect::<Vec<_>>();
    let cleanup = run_cleanup(
        &settings,
        &voice_action.text,
        &protected_terms,
        active.target.process_name.as_deref(),
    )
    .await;
    let cleaned = match cleanup {
        // The post-AI pass is the same user text, so it honours the same toggles.
        Ok(result) => sanitize_transcription_formatting(&resolve_with_options(
            result.cleaned_text.trim(),
            BacktrackingOptions {
                backtracking: settings.backtracking_enabled,
                remove_filler_words: settings.remove_filler_words,
                remove_false_starts: settings.remove_false_starts,
            },
        )),
        Err(error) => {
            log::warn!(
                "cleanup fallback activated; category={}",
                error.payload().category
            );
            sanitize_transcription_formatting(&voice_action.text)
        }
    };
    let mut final_text =
        sanitize_transcription_formatting(&apply_dictionary(&finish_sentence(&cleaned), &entries)?);
    let transform_id = settings
        .auto_apply_transform
        .clone()
        .filter(|value| !value.trim().is_empty());
    if let Some(transform_id) = transform_id.as_deref() {
        let transformed = transforms::apply_transform(
            TransformRequest {
                text: final_text.clone(),
                transform_id: transform_id.into(),
            },
            &settings,
        )
        .await?;
        final_text = transformed.transformed_text;
    }
    final_text = sanitize_transcription_formatting(&final_text);
    if active.mode == DictationMode::Call {
        final_text = format_call_notes(&final_text, &settings.call_mode_application);
    }
    if final_text.trim().is_empty() {
        let error = AppError::Cleanup("cleanup produced empty text".into());
        set_error(app, state, error.payload()).await;
        return Err(error);
    }
    let post_action =
        if voice_action.action == PostPasteAction::Enter && !settings.press_enter_enabled {
            PostPasteAction::None
        } else {
            voice_action.action
        };

    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.transition(DictationState::Inserting)?;
    }
    emit_snapshot(app, state).await;
    let insertion_text = final_text.clone();
    let insertion_target = active.target.clone();
    let is_call_mode = active.mode == DictationMode::Call;
    let insertion = tokio::task::spawn_blocking(move || {
        if is_call_mode {
            insertion::copy_text(&insertion_text)
        } else {
            insertion::paste_into_target(
                &insertion_text,
                &insertion_target,
                post_action,
                settings.paste_delay_ms,
                settings.restore_clipboard,
            )
        }
    })
    .await
    .map_err(|error| AppError::Insertion(error.to_string()))?;
    let (insertion_status, insertion_error) = match insertion {
        Ok(()) => (InsertionStatus::Inserted, None),
        Err(error) => {
            let fallback = insertion::copy_text(&final_text);
            let status = if fallback.is_ok() {
                InsertionStatus::Copied
            } else {
                InsertionStatus::Failed
            };
            (status, Some(error))
        }
    };

    let audio_path = if settings.save_audio {
        save_recording(app, &active.id, &captured).ok()
    } else {
        None
    };
    let record = TranscriptRecord {
        id: active.id.clone(),
        created_at: Utc::now(),
        started_at: active.started_at,
        duration_ms: captured.duration_ms,
        processing_ms: processing_started.elapsed().as_millis() as i64,
        application_name: active.target.application_name.clone(),
        process_name: active.target.process_name.clone(),
        window_title: active.target.window_title.clone(),
        mode: active.mode,
        raw_transcript: raw,
        normalized_transcript: normalized,
        cleaned_transcript: cleaned,
        final_transcript: final_text,
        transform_id,
        provider: "deepgram".into(),
        model: settings.transcription_model.clone(),
        confidence: transcription.confidence,
        insertion_status,
        post_paste_action: post_action,
        audio_path,
        is_favorite: false,
    };
    // Transcript text is kept out of the logs unless the user explicitly opts
    // in while reproducing a problem.
    if settings.include_transcript_in_logs {
        log::debug!(
            "transcript stages; raw={:?} normalized={:?} cleaned={:?} final={:?}",
            record.raw_transcript,
            record.normalized_transcript,
            record.cleaned_transcript,
            record.final_transcript
        );
    }

    // Captured before the record is consumed by the save path below.
    let inserted_word_count = record.final_transcript.split_whitespace().count();
    let inserted_application = record
        .application_name
        .clone()
        .unwrap_or_else(|| "the active app".to_string());

    if settings.save_history {
        // Intermediate stages are dropped before they reach the database when
        // the user has opted out of storing them.
        let mut record = record;
        if !settings.store_raw_transcript {
            record.raw_transcript.clear();
        }
        if !settings.store_normalized_transcript {
            record.normalized_transcript.clear();
        }
        if !settings.store_cleaned_transcript {
            record.cleaned_transcript.clear();
        }
        if let Err(error) = state.database.insert_transcript(&record) {
            retain_failed_audio(state, &active.id, captured).await;
            set_error(app, state, error.payload()).await;
            return Err(error);
        }
        if let Err(error) = state
            .database
            .prune_transcripts(settings.history_retention_days, settings.max_history_entries)
        {
            // Retention is housekeeping: log it, but never fail the dictation.
            log::warn!(
                "history pruning failed; category={}",
                error.payload().category
            );
        }
    }

    if let Some(error) = insertion_error {
        // The text is already on the clipboard and in History, so the toast
        // tells the user where to find it rather than just reporting failure.
        notify(
            app,
            &settings,
            "Paste failed",
            "Your transcript was copied to the clipboard and saved in History.",
        );
        set_error(app, state, error.payload()).await;
        return Err(error);
    }
    if matches!(insertion_status, InsertionStatus::Inserted) {
        notify(
            app,
            &settings,
            "Dictation inserted",
            &format!("{inserted_word_count} words inserted into {inserted_application}."),
        );
    }
    {
        let mut runtime = state.dictation.lock().await;
        runtime.recovery_audio = None;
        runtime.machine.transition(DictationState::Completed)?;
    }
    emit_snapshot(app, state).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    hide_overlay(app);
    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.transition(DictationState::Idle)?;
        runtime.machine.reset();
    }
    emit_snapshot(app, state).await;
    Ok(state.dictation.lock().await.machine.snapshot())
}

/// The modifier-only shortcut starts immediately as push-to-talk. If Space is
/// pressed while the modifiers are still down, promote that live session to
/// hands-free instead of discarding the first audio frames.
pub async fn promote_to_hands_free(
    app: &AppHandle,
    state: &AppState,
) -> Result<DictationSnapshot, AppError> {
    {
        let mut runtime = state.dictation.lock().await;
        if runtime.machine.snapshot().state != DictationState::ListeningPushToTalk {
            return Err(AppError::InvalidTransition(
                "hands-free promotion requires an active push-to-talk session".into(),
            ));
        }
        runtime.machine.set_mode(DictationMode::HandsFree);
        runtime
            .machine
            .transition(DictationState::ListeningHandsFree)?;
        if let Some(active) = runtime.active.as_mut() {
            active.mode = DictationMode::HandsFree;
        }
    }
    emit_snapshot(app, state).await;
    Ok(state.dictation.lock().await.machine.snapshot())
}

pub async fn promote_to_command(
    app: &AppHandle,
    state: &AppState,
) -> Result<DictationSnapshot, AppError> {
    {
        let mut runtime = state.dictation.lock().await;
        let current_state = runtime.machine.snapshot().state;
        if !matches!(
            current_state,
            DictationState::Starting | DictationState::ListeningPushToTalk
        ) {
            return Err(AppError::InvalidTransition(
                "command mode promotion requires an active push-to-talk session".into(),
            ));
        }
        runtime.machine.set_mode(DictationMode::Command);
        if let Some(active) = runtime.active.as_mut() {
            active.mode = DictationMode::Command;
        }
    }
    emit_snapshot(app, state).await;
    Ok(state.dictation.lock().await.machine.snapshot())
}

pub async fn cancel(app: &AppHandle, state: &AppState) -> Result<DictationSnapshot, AppError> {
    let active = {
        let mut runtime = state.dictation.lock().await;
        if let Some(active) = runtime.active.take() {
            runtime.machine.transition(DictationState::Cancelled)?;
            Some(active)
        } else {
            if let Some(recovery) = runtime.recovery_audio.take() {
                log::debug!(
                    "discarding recovery audio; session_id={} samples={}",
                    recovery.session_id,
                    recovery.audio.samples.len()
                );
            }
            runtime.machine.reset();
            None
        }
    };
    if let Some(mut active) = active {
        active.transcription.abort();
        if let Some(capture) = active.capture.take() {
            let _ = tokio::task::spawn_blocking(move || capture.stop()).await;
        }
    }
    emit_snapshot(app, state).await;
    hide_overlay(app);
    {
        let mut runtime = state.dictation.lock().await;
        if runtime.machine.snapshot().state == DictationState::Cancelled {
            runtime.machine.transition(DictationState::Idle)?;
        }
        runtime.machine.reset();
    }
    emit_snapshot(app, state).await;
    Ok(state.dictation.lock().await.machine.snapshot())
}

async fn stream_transcription(
    mut receiver: mpsc::Receiver<Vec<u8>>,
    config: TranscriptionConfig,
    event_sender: mpsc::UnboundedSender<ProviderEvent>,
) -> Result<crate::models::TranscriptionResult, AppError> {
    let api_key = tokio::task::spawn_blocking(|| security::get(CredentialKind::Deepgram))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))??;
    let provider = DeepgramProvider::new(api_key, event_sender);
    let session = provider.start_session(config).await?;
    while let Some(chunk) = receiver.recv().await {
        provider.send_audio(&session, &chunk).await?;
    }
    provider.finish_session(session).await
}

fn spawn_interim_forwarder(
    app: AppHandle,
    session_id: String,
    mut receiver: mpsc::UnboundedReceiver<ProviderEvent>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(ProviderEvent::Interim(text)) = receiver.recv().await {
            let state = app.state::<AppState>();
            let should_finish = {
                let mut runtime = state.dictation.lock().await;
                if runtime.machine.snapshot().session_id.as_deref() != Some(&session_id) {
                    break;
                }
                runtime.machine.set_interim(text);
                let snapshot = runtime.machine.snapshot();
                let press_enter_enabled = state.settings.read().await.press_enter_enabled;
                snapshot.state == DictationState::ListeningHandsFree
                    && (snapshot
                        .interim_transcript
                        .trim_end()
                        .to_ascii_lowercase()
                        .ends_with("finish dictation")
                        || (press_enter_enabled
                            && snapshot
                                .interim_transcript
                                .trim_end()
                                .to_ascii_lowercase()
                                .ends_with("press enter")))
            };
            let snapshot = state.dictation.lock().await.machine.snapshot();
            let _ = app.emit("dictation-state", snapshot);
            if should_finish {
                if let Err(error) = finish(&app, &state).await {
                    log::warn!(
                        "voice finish action failed; category={}",
                        error.payload().category
                    );
                }
                break;
            }
        }
    });
}

async fn run_cleanup(
    settings: &crate::models::AppSettings,
    text: &str,
    protected_terms: &[String],
    active_process: Option<&str>,
) -> Result<CleanupResult, AppError> {
    if !settings.cleanup_enabled {
        return Ok(CleanupResult {
            cleaned_text: text.to_string(),
            corrections_applied: vec![],
            post_paste_action: PostPasteAction::None,
            confidence: 1.0,
        });
    }
    let key = tokio::task::spawn_blocking(|| security::get(CredentialKind::Cleanup))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))??;
    let provider = OpenAiCompatibleCleanup::new(
        settings.cleanup_endpoint.clone(),
        key,
        settings.cleanup_model.clone(),
    )?;
    let style = settings.cleanup_style_for(active_process);
    if style == "balanced" {
        provider.cleanup(text, protected_terms).await
    } else {
        provider
            .cleanup_with_style(text, protected_terms, &style)
            .await
    }
}

async fn set_error(app: &AppHandle, state: &AppState, error: AppErrorPayload) {
    state.dictation.lock().await.machine.fail(error);
    emit_snapshot(app, state).await;
}

async fn retain_failed_audio(state: &AppState, session_id: &str, audio: CapturedAudio) {
    state.dictation.lock().await.recovery_audio = Some(RecoveryAudio {
        session_id: session_id.into(),
        audio,
    });
}

async fn finish_as_cancelled(
    app: &AppHandle,
    state: &AppState,
) -> Result<DictationSnapshot, AppError> {
    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.transition(DictationState::Cancelled)?;
    }
    emit_snapshot(app, state).await;
    hide_overlay(app);
    {
        let mut runtime = state.dictation.lock().await;
        runtime.machine.transition(DictationState::Idle)?;
        runtime.machine.reset();
    }
    Ok(state.dictation.lock().await.machine.snapshot())
}

async fn emit_snapshot(app: &AppHandle, state: &AppState) {
    let snapshot = state.dictation.lock().await.machine.snapshot();
    let _ = app.emit("dictation-state", snapshot);
}

const OVERLAY_WIDTH: u32 = 260;
const OVERLAY_HEIGHT: u32 = 44;
const OVERLAY_MARGIN: u32 = 56;

fn show_overlay(app: &AppHandle, settings: &crate::models::AppSettings) -> Result<(), AppError> {
    if !settings.show_overlay {
        return Ok(());
    }
    let window = app
        .get_webview_window("overlay")
        .ok_or_else(|| AppError::Windows("recording overlay is unavailable".into()))?;

    // Anchor to the monitor under the cursor, which is where the user is working,
    // rather than always the primary display.
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|position| app.monitor_from_point(position.x, position.y).ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let size = monitor.size();
        let origin = monitor.position();
        let centered_x = origin.x + (size.width.saturating_sub(OVERLAY_WIDTH) / 2) as i32;
        let right_x =
            origin.x + size.width.saturating_sub(OVERLAY_WIDTH + OVERLAY_MARGIN / 2) as i32;
        let top_y = origin.y + OVERLAY_MARGIN as i32;
        let bottom_y = origin.y + size.height.saturating_sub(OVERLAY_MARGIN + OVERLAY_HEIGHT) as i32;
        let (x, y) = match settings.overlay_position.as_str() {
            "top_center" => (centered_x, top_y),
            "top_right" => (right_x, top_y),
            "bottom_right" => (right_x, bottom_y),
            _ => (centered_x, bottom_y),
        };
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }

    window
        .set_always_on_top(true)
        .map_err(|error| AppError::Windows(error.to_string()))?;
    window
        .show()
        .map_err(|error| AppError::Windows(error.to_string()))
}

/// Sends a Windows toast, honouring the user's notification preference.
/// Notification failures are never surfaced — they must not derail dictation.
fn notify(app: &AppHandle, settings: &crate::models::AppSettings, title: &str, body: &str) {
    if !settings.show_notifications {
        return;
    }
    use tauri_plugin_notification::NotificationExt;
    if let Err(error) = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        log::debug!("notification could not be shown: {error}");
    }
}

fn hide_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
}

fn save_recording(
    app: &AppHandle,
    session_id: &str,
    audio: &CapturedAudio,
) -> Result<String, AppError> {
    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    directory.push("recordings");
    std::fs::create_dir_all(&directory)?;
    let path: PathBuf = directory.join(format!("{session_id}.wav"));
    audio::wav::write_pcm16(&path, audio)?;
    Ok(path.to_string_lossy().into_owned())
}

fn finish_sentence(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut value = trimmed.to_string();
    if let Some(first) = value.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    if !value.ends_with(['.', '!', '?', ':']) {
        value.push('.');
    }
    value
}

fn sanitize_transcription_formatting(input: &str) -> String {
    input
        .replace(" \u{2014} ", ", ")
        .replace('\u{2014}', "-")
        .replace('\u{2013}', "-")
        .replace("\n- \n", "\n")
        .trim()
        .to_string()
}

fn normalize_spoken_structure(input: &str) -> String {
    let mut value = input.to_string();
    for cue in ["new paragraph", "next paragraph"] {
        value = value.replace(cue, "\n\n");
    }
    for cue in ["new line", "next line"] {
        value = value.replace(cue, "\n");
    }
    for cue in ["bullet point", "bullet", "next point"] {
        value = value.replace(cue, "\n- ");
    }
    value
}

fn format_call_notes(text: &str, application: &str) -> String {
    let title = application.trim();
    let title = if title.is_empty() { "Call" } else { title };
    let body = text.trim();
    if body.lines().any(|line| line.trim_start().starts_with('-')) {
        format!("# {title} call notes\n\n{body}")
    } else {
        format!("# {title} call notes\n\n- {body}")
    }
}


fn command_needs_screen_context(prompt: &str) -> bool {
    let normalized = prompt.to_ascii_lowercase();
    [
        "explain this",
        "look at this",
        "what am i looking at",
        "what's on my screen",
        "what is on my screen",
        "summarize this",
        "summarize this page",
        "can you see this",
        "debug this",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_developer_application_profiles() {
        let settings = crate::models::AppSettings::default();
        assert_eq!(settings.cleanup_style_for(Some("Code.exe")), "developer");
        assert_eq!(
            settings.cleanup_style_for(Some("RobloxStudioBeta.exe")),
            "developer"
        );
        assert_eq!(settings.cleanup_style_for(Some("Discord.exe")), "casual");
        // Unmapped applications fall back to the configured default.
        assert_eq!(settings.cleanup_style_for(Some("notepad.exe")), "balanced");
    }

    #[test]
    fn app_detection_can_be_disabled() {
        let settings = crate::models::AppSettings {
            auto_detect_developer_apps: false,
            ..Default::default()
        };
        assert_eq!(settings.cleanup_style_for(Some("Code.exe")), "balanced");
    }

    #[test]
    fn identifies_screen_aware_buddy_requests() {
        assert!(command_needs_screen_context("Explain this screen for me."));
        assert!(command_needs_screen_context("What am I looking at?"));
        assert!(command_needs_screen_context("Can you summarize this page?"));
        assert!(!command_needs_screen_context(
            "Give me three names for a feature."
        ));
    }

    #[test]
    fn transcript_formatting_removes_long_dashes_and_keeps_spoken_lists() {
        assert_eq!(
            sanitize_transcription_formatting("Build \u{2014} test \u{2013} ship"),
            "Build, test - ship"
        );
        assert_eq!(
            normalize_spoken_structure("first bullet point add tests"),
            "first \n-  add tests"
        );
    }

    #[test]
    fn call_notes_are_copied_as_a_scannable_note() {
        assert_eq!(
            format_call_notes("Review the release plan.", "Discord"),
            "# Discord call notes\n\n- Review the release plan."
        );
    }
}
