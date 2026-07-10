use tauri::{AppHandle, State};

use crate::{
    app_state::AppState,
    audio, dictation,
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
    *state.settings.write().await = settings.clone();
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
    let settings = state.settings.read().await.clone();
    let target = state
        .dictation
        .lock()
        .await
        .last_target
        .clone()
        .or_else(|| insertion::capture_active_target().ok())
        .ok_or_else(|| AppError::Insertion("no previous text target is available".into()))?;
    let text = record.final_transcript;
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
