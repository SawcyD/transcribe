mod app_state;
mod assistant;
mod audio;
mod brand;
mod cleanup;
mod commands;
mod context;
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
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager, PhysicalPosition, Position,
};

use app_state::AppState;
use brand::{DATABASE_FILE, PRODUCT_NAME};
use models::{DictationMode, DictationSnapshot, DictationState};
use shortcuts::ShortcutAction;

const TRAY_ID: &str = "voiceflow-tray";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app)
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // Only the main window has a user-chosen size and position. The overlay,
        // buddy, and assistant windows are positioned programmatically.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&["overlay", "buddy", "assistant"])
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(PRODUCT_NAME)
                .build(),
        )
        .plugin(
            tauri_plugin_log::Builder::default()
                // The stored preference is read before the plugin starts, since
                // the log level is fixed at build time for the process.
                .level(if cfg!(debug_assertions) || debug_logging_enabled() {
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
            watch_tray_state(app);
            show_buddy(app);
            let mut receiver = {
                let state = app.state::<AppState>();
                let bindings = tauri::async_runtime::block_on(state.settings.read())
                    .shortcuts
                    .clone();
                shortcuts::start_modifier_hook(&bindings)?
            };
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                log::debug!("shortcut dispatcher started");
                while let Some(signal) = receiver.recv().await {
                    log::debug!("shortcut dispatcher received: {signal:?}");
                    let app = handle.clone();
                    let state = app.state::<AppState>();
                    match (signal.action, signal.pressed) {
                        (ShortcutAction::PushToTalk, true) => {
                            let current = state.dictation.lock().await.machine.snapshot().state;
                            if current == DictationState::Idle {
                                if let Err(error) =
                                    dictation::start(&app, &state, DictationMode::PushToTalk).await
                                {
                                    log::warn!(
                                        "push-to-talk start failed; category={}",
                                        error.payload().category
                                    );
                                }
                            }
                        }
                        (ShortcutAction::PushToTalk, false) => {
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
                        // Hands-free is a toggle: only the press edge is meaningful.
                        (ShortcutAction::HandsFree, true) => {
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
                        (ShortcutAction::CommandMode, true) => {
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
                        (ShortcutAction::CommandMode, false) => {
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
                        (ShortcutAction::Cancel, true) => {
                            let current = state.dictation.lock().await.machine.snapshot().state;
                            if current != DictationState::Idle {
                                let _ = dictation::cancel(&app, &state).await;
                            }
                        }
                        // Release edges for toggle-style actions carry no meaning.
                        (ShortcutAction::HandsFree, false) | (ShortcutAction::Cancel, false) => {}
                    }
                }
                log::error!("shortcut dispatcher stopped");
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let state = window.app_handle().state::<AppState>();
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if state.close_to_tray() {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    // Otherwise fall through: closing the window exits the app.
                }
                tauri::WindowEvent::Resized(_) if state.minimize_to_tray() => {
                    if window.is_minimized().unwrap_or(false) {
                        let _ = window.hide();
                    }
                }
                _ => {}
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
            commands::paste_latest_transcript,
            commands::transform_text,
            commands::save_dictionary_entry,
            commands::delete_dictionary_entry,
            commands::capture_screen_context,
            commands::hide_buddy,
            commands::apply_buddy_settings,
            commands::show_history,
            commands::show_buddy_settings,
            commands::open_data_folder,
            commands::diagnostic_report,
            commands::clear_history,
            commands::open_assistant_drawer,
            commands::get_pending_assistant_context,
            commands::get_pending_assistant_voice_prompt,
            commands::ask_assistant,
        ])
        .run(tauri::generate_context!())
        .expect("VoiceFlow Dev failed to start");
}

/// Reads the debug-logging preference before Tauri is built.
///
/// The log plugin fixes its level at registration time, which happens before
/// `AppState` exists, so the database is opened directly at the well-known
/// Windows location. Any failure simply means the default level is used.
fn debug_logging_enabled() -> bool {
    let Ok(app_data) = std::env::var("APPDATA") else {
        return false;
    };
    let path = std::path::Path::new(&app_data)
        .join("dev.voiceflow.desktop")
        .join(DATABASE_FILE);
    if !path.exists() {
        return false;
    }
    database::Database::open(&path)
        .and_then(|database| database.load_settings())
        .map(|settings| settings.debug_logging)
        .unwrap_or(false)
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open VoiceFlow", true, None::<&str>)?;
    let hands_free = MenuItem::with_id(
        app,
        "hands_free",
        "Start hands-free dictation",
        true,
        None::<&str>,
    )?;
    let command_mode =
        MenuItem::with_id(app, "command_mode", "Open Command Mode", true, None::<&str>)?;
    let call_mode = MenuItem::with_id(app, "call_mode", "Call Mode", true, None::<&str>)?;
    let paste_latest = MenuItem::with_id(
        app,
        "paste_latest",
        "Paste latest transcript",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Exit VoiceFlow", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &hands_free,
            &command_mode,
            &call_mode,
            &PredefinedMenuItem::separator(app)?,
            &paste_latest,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_icon(TrayVisual::Idle))
        .tooltip(format!("{PRODUCT_NAME} — Ready\nHold Ctrl + Win to dictate"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "hands_free" => spawn_dictation(app, DictationMode::HandsFree),
            "command_mode" => spawn_dictation(app, DictationMode::Command),
            "call_mode" => spawn_dictation(app, DictationMode::Call),
            "paste_latest" => spawn_paste_latest(app),
            "settings" => {
                show_main_window(app);
                let _ = app.emit("navigate", "/settings");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            let TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                ..
            } = event
            else {
                return;
            };
            match button {
                MouseButton::Left => show_main_window(app),
                MouseButton::Middle => spawn_paste_latest(app),
                MouseButton::Right => {}
            }
        })
        .build(app)?;
    Ok(())
}

/// Mirrors dictation state onto the tray icon and tooltip so the app stays
/// legible while the main window is hidden.
fn watch_tray_state(app: &tauri::App) {
    let handle = app.handle().clone();
    app.listen("dictation-state", move |event| {
        let Ok(snapshot) = serde_json::from_str::<DictationSnapshot>(event.payload()) else {
            return;
        };
        let (visual, tooltip) = match snapshot.state {
            DictationState::Idle => (
                TrayVisual::Idle,
                format!("{PRODUCT_NAME} — Ready\nHold Ctrl + Win to dictate"),
            ),
            DictationState::Starting
            | DictationState::ListeningPushToTalk
            | DictationState::ListeningHandsFree => {
                (TrayVisual::Listening, format!("{PRODUCT_NAME} — Listening"))
            }
            DictationState::Error => (
                TrayVisual::Error,
                snapshot
                    .error
                    .as_ref()
                    .map(|error| format!("{PRODUCT_NAME} — {}", error.message))
                    .unwrap_or_else(|| format!("{PRODUCT_NAME} — Error")),
            ),
            _ => (
                TrayVisual::Processing,
                format!("{PRODUCT_NAME} — Processing transcript"),
            ),
        };
        if let Some(tray) = handle.tray_by_id(TRAY_ID) {
            let _ = tray.set_icon(Some(tray_icon(visual)));
            let _ = tray.set_tooltip(Some(&tooltip));
        }
    });
}

fn spawn_dictation(app: &tauri::AppHandle, mode: DictationMode) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if state.dictation.lock().await.machine.snapshot().state != DictationState::Idle {
            return;
        }
        if let Err(error) = dictation::start(&app, &state, mode).await {
            log::warn!(
                "tray dictation start failed; category={}",
                error.payload().category
            );
        }
    });
}

fn spawn_paste_latest(app: &tauri::AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let record = match state.database.list_transcripts("") {
            Ok(records) => records.into_iter().next(),
            Err(error) => {
                log::warn!("tray paste lookup failed; category={}", error.payload().category);
                return;
            }
        };
        let Some(record) = record else {
            log::info!("tray paste requested with no stored transcripts");
            return;
        };
        if let Err(error) = commands::paste_record_text(&state, record.final_transcript).await {
            log::warn!("tray paste failed; category={}", error.payload().category);
        }
    });
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_buddy(app: &tauri::App) {
    let Some(window) = app.get_webview_window("buddy") else {
        return;
    };
    let settings = tauri::async_runtime::block_on(app.state::<AppState>().settings.read()).clone();
    if !settings.buddy_enabled || !settings.buddy_show_at_startup {
        let _ = window.hide();
        return;
    }
    let side = match settings.buddy_size.as_str() {
        "small" => 104u32,
        "large" => 192,
        _ => 144,
    };
    let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(side, side)));
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let size = monitor.size();
        let origin = monitor.position();
        let x = origin.x + size.width.saturating_sub(176) as i32;
        let y = origin.y + size.height.saturating_sub(178) as i32;
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }
    let _ = window.set_always_on_top(settings.buddy_always_on_top);
    let _ = window.show();
}

#[derive(Clone, Copy)]
enum TrayVisual {
    Idle,
    Listening,
    Processing,
    Error,
}

impl TrayVisual {
    /// Windows tray icons are tiny, so state reads as a colour shift on the
    /// waveform rather than a different glyph.
    fn waveform(self) -> [u8; 4] {
        match self {
            TrayVisual::Idle => [212, 218, 226, 255],
            TrayVisual::Listening => [96, 205, 255, 255],
            TrayVisual::Processing => [255, 200, 120, 255],
            TrayVisual::Error => [255, 141, 141, 255],
        }
    }
}

fn tray_icon(visual: TrayVisual) -> Image<'static> {
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
    let waveform = visual.waveform();
    for (x, half_height) in [(8usize, 3usize), (12, 7), (16, 11), (20, 7), (24, 3)] {
        for y in 16 - half_height..=16 + half_height {
            let index = (y * size + x) * 4;
            rgba[index..index + 4].copy_from_slice(&waveform);
        }
    }
    Image::new_owned(rgba, size as u32, size as u32)
}
