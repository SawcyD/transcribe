mod app_state;
mod audio;
mod brand;
mod cleanup;
mod commands;
mod database;
mod dictation;
mod errors;
mod insertion;
mod models;
mod security;
mod shortcuts;
mod state_machine;
mod transcription;
mod transforms;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use app_state::AppState;
use brand::{DATABASE_FILE, PRODUCT_NAME};
use models::{DictationMode, DictationState};
use shortcuts::ShortcutSignal;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app)
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(PRODUCT_NAME)
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .max_file_size(2_000_000)
                .build(),
        )
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let database = database::Database::open(&data_dir.join(DATABASE_FILE))?;
            let settings = database.load_settings()?;
            app.manage(AppState::new(database, settings));
            setup_tray(app)?;
            let mut receiver = shortcuts::start_modifier_hook()?;
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                log::debug!("shortcut dispatcher started");
                while let Some(signal) = receiver.recv().await {
                    log::debug!("shortcut dispatcher received: {signal:?}");
                    let app = handle.clone();
                    let state = app.state::<AppState>();
                    match signal {
                        ShortcutSignal::PushToTalkPressed => {
                            let current = state.dictation.lock().await.machine.snapshot().state;
                            match current {
                                DictationState::Idle => {
                                    if let Err(error) =
                                        dictation::start(&app, &state, DictationMode::PushToTalk)
                                            .await
                                    {
                                        log::warn!(
                                            "push-to-talk start failed; category={}",
                                            error.payload().category
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        ShortcutSignal::PushToTalkReleased => {
                            if state.dictation.lock().await.machine.snapshot().state
                                == DictationState::ListeningPushToTalk
                            {
                                if let Err(error) = dictation::finish(&app, &state).await {
                                    log::warn!(
                                        "push-to-talk finish failed; category={}",
                                        error.payload().category
                                    );
                                }
                            }
                        }
                        ShortcutSignal::HandsFreeToggle => {
                            let current = state.dictation.lock().await.machine.snapshot().state;
                            match current {
                                DictationState::Idle => {
                                    if let Err(error) =
                                        dictation::start(&app, &state, DictationMode::HandsFree)
                                            .await
                                    {
                                        log::warn!(
                                            "hands-free start failed; category={}",
                                            error.payload().category
                                        );
                                    }
                                }
                                DictationState::ListeningPushToTalk => {
                                    if let Err(error) =
                                        dictation::promote_to_hands_free(&app, &state).await
                                    {
                                        log::warn!(
                                            "hands-free promotion failed; category={}",
                                            error.payload().category
                                        );
                                    }
                                }
                                DictationState::ListeningHandsFree => {
                                    if let Err(error) = dictation::finish(&app, &state).await {
                                        log::warn!(
                                            "hands-free finish failed; category={}",
                                            error.payload().category
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        ShortcutSignal::CommandModePressed => {
                            let snapshot = state.dictation.lock().await.machine.snapshot();
                            match snapshot.state {
                                DictationState::Idle => {
                                    if let Err(error) =
                                        dictation::start(&app, &state, DictationMode::Command).await
                                    {
                                        log::warn!(
                                            "command mode start failed; category={}",
                                            error.payload().category
                                        );
                                    }
                                }
                                DictationState::Starting | DictationState::ListeningPushToTalk
                                    if snapshot.mode == Some(DictationMode::PushToTalk) =>
                                {
                                    if let Err(error) =
                                        dictation::promote_to_command(&app, &state).await
                                    {
                                        log::warn!(
                                            "command mode promotion failed; category={}",
                                            error.payload().category
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        ShortcutSignal::CommandModeReleased => {
                            let snapshot = state.dictation.lock().await.machine.snapshot();
                            if snapshot.state == DictationState::ListeningPushToTalk
                                && snapshot.mode == Some(DictationMode::Command)
                            {
                                if let Err(error) = dictation::finish(&app, &state).await {
                                    log::warn!(
                                        "command mode finish failed; category={}",
                                        error.payload().category
                                    );
                                }
                            }
                        }
                        ShortcutSignal::Cancel => {
                            let current = state.dictation.lock().await.machine.snapshot().state;
                            if current != DictationState::Idle {
                                let _ = dictation::cancel(&app, &state).await;
                            }
                        }
                    }
                }
                log::error!("shortcut dispatcher stopped");
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dictation_snapshot,
            commands::start_dictation,
            commands::finish_dictation,
            commands::cancel_dictation,
            commands::list_transcripts,
            commands::get_transcript,
            commands::delete_transcript,
            commands::list_dictionary_entries,
            commands::dashboard_stats,
            commands::get_settings,
            commands::save_settings,
            commands::set_provider_credential,
            commands::delete_provider_credential,
            commands::credential_status,
            commands::list_microphones,
            commands::copy_text,
            commands::paste_transcript,
            commands::transform_text,
            commands::save_dictionary_entry,
            commands::delete_dictionary_entry,
        ])
        .run(tauri::generate_context!())
        .expect("VoiceFlow Dev failed to start");
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open VoiceFlow Dev", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::with_id("voiceflow-tray")
        .icon(tray_icon())
        .tooltip(PRODUCT_NAME)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn tray_icon() -> Image<'static> {
    let size = 32usize;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let index = (y * size + x) * 4;
            let dx = x as f32 - 15.5;
            let dy = y as f32 - 15.5;
            if dx * dx + dy * dy <= 14.5 * 14.5 {
                rgba[index..index + 4].copy_from_slice(&[17, 21, 27, 255]);
            }
        }
    }
    for (x, half_height) in [(8usize, 3usize), (12, 7), (16, 11), (20, 7), (24, 3)] {
        for y in 16 - half_height..=16 + half_height {
            let index = (y * size + x) * 4;
            rgba[index..index + 4].copy_from_slice(&[141, 241, 189, 255]);
        }
    }
    Image::new_owned(rgba, size as u32, size as u32)
}
