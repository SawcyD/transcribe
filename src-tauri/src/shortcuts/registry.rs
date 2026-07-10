use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU,
            VK_RWIN,
        },
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN,
            WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutSignal {
    PushToTalkPressed,
    PushToTalkReleased,
    HandsFreeToggle,
    CommandModePressed,
    CommandModeReleased,
    Cancel,
}

struct HookContext {
    sender: UnboundedSender<ShortcutSignal>,
    control_left: AtomicBool,
    control_right: AtomicBool,
    control_generic: AtomicBool,
    windows_left: AtomicBool,
    windows_right: AtomicBool,
    space: AtomicBool,
    alt_left: AtomicBool,
    alt_right: AtomicBool,
    alt_generic: AtomicBool,
    combo_active: AtomicBool,
    command_active: AtomicBool,
}

static CONTEXT: OnceLock<HookContext> = OnceLock::new();

pub fn start_modifier_hook() -> Result<UnboundedReceiver<ShortcutSignal>, AppError> {
    let (sender, receiver) = unbounded_channel();
    CONTEXT
        .set(HookContext {
            sender,
            control_left: AtomicBool::new(false),
            control_right: AtomicBool::new(false),
            control_generic: AtomicBool::new(false),
            windows_left: AtomicBool::new(false),
            windows_right: AtomicBool::new(false),
            space: AtomicBool::new(false),
            alt_left: AtomicBool::new(false),
            alt_right: AtomicBool::new(false),
            alt_generic: AtomicBool::new(false),
            combo_active: AtomicBool::new(false),
            command_active: AtomicBool::new(false),
        })
        .map_err(|_| {
            AppError::Windows("push-to-talk keyboard hook was already initialized".into())
        })?;
    let (init_sender, init_receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("voiceflow-shortcut-hook".into())
        .spawn(move || unsafe {
            let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) {
                Ok(hook) => {
                    log::debug!("global keyboard hook installed");
                    let _ = init_sender.send(Ok(()));
                    hook
                }
                Err(error) => {
                    log::error!("failed to install keyboard hook: {error}");
                    let _ = init_sender.send(Err(error.to_string()));
                    return;
                }
            };
            let mut message = MSG::default();
            loop {
                let result = GetMessageW(&mut message, None, 0, 0);
                if result.0 == 0 {
                    break;
                }
                if result.0 == -1 {
                    log::error!("global keyboard hook message loop failed");
                    break;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            log::debug!("global keyboard hook stopped");
            let _ = UnhookWindowsHookEx(hook);
        })
        .map_err(|error| AppError::Windows(error.to_string()))?;
    match init_receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(())) => Ok(receiver),
        Ok(Err(error)) => Err(AppError::Windows(format!(
            "failed to install global keyboard hook: {error}"
        ))),
        Err(error) => Err(AppError::Windows(format!(
            "timed out waiting for global keyboard hook: {error}"
        ))),
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let event = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let down = wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize;
        let up = wparam.0 == WM_KEYUP as usize || wparam.0 == WM_SYSKEYUP as usize;
        if down || up {
            if let Some(context) = CONTEXT.get() {
                let key = event.vkCode as u16;
                if key == VK_ESCAPE.0 && down {
                    send_signal(context, ShortcutSignal::Cancel);
                }
                if key == VK_CONTROL.0 {
                    context.control_generic.store(down, Ordering::Relaxed);
                } else if key == VK_LCONTROL.0 {
                    context.control_left.store(down, Ordering::Relaxed);
                } else if key == VK_RCONTROL.0 {
                    context.control_right.store(down, Ordering::Relaxed);
                }
                if key == VK_LWIN.0 {
                    context.windows_left.store(down, Ordering::Relaxed);
                } else if key == VK_RWIN.0 {
                    context.windows_right.store(down, Ordering::Relaxed);
                }
                if key == VK_MENU.0 {
                    context.alt_generic.store(down, Ordering::Relaxed);
                } else if key == VK_LMENU.0 {
                    context.alt_left.store(down, Ordering::Relaxed);
                } else if key == VK_RMENU.0 {
                    context.alt_right.store(down, Ordering::Relaxed);
                }
                if key == windows::Win32::UI::Input::KeyboardAndMouse::VK_SPACE.0 {
                    let was_space = context.space.swap(down, Ordering::Relaxed);
                    if down && !was_space && control_down(context) && windows_down(context) {
                        send_signal(context, ShortcutSignal::HandsFreeToggle);
                    }
                }
                let control = control_down(context);
                let windows = windows_down(context);
                let alt = alt_down(context);
                let command = control && windows && alt;
                let was_command = context.command_active.swap(command, Ordering::Relaxed);
                if command != was_command {
                    let signal = if command {
                        ShortcutSignal::CommandModePressed
                    } else {
                        ShortcutSignal::CommandModeReleased
                    };
                    send_signal(context, signal);
                }
                let active = control && windows;
                let was_active = context.combo_active.swap(active, Ordering::Relaxed);
                if active != was_active && !command && !was_command {
                    let signal = if active {
                        ShortcutSignal::PushToTalkPressed
                    } else {
                        ShortcutSignal::PushToTalkReleased
                    };
                    send_signal(context, signal);
                }
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn control_down(context: &HookContext) -> bool {
    context.control_left.load(Ordering::Relaxed)
        || context.control_right.load(Ordering::Relaxed)
        || context.control_generic.load(Ordering::Relaxed)
}

fn windows_down(context: &HookContext) -> bool {
    context.windows_left.load(Ordering::Relaxed) || context.windows_right.load(Ordering::Relaxed)
}

fn alt_down(context: &HookContext) -> bool {
    context.alt_left.load(Ordering::Relaxed)
        || context.alt_right.load(Ordering::Relaxed)
        || context.alt_generic.load(Ordering::Relaxed)
}

fn send_signal(context: &HookContext, signal: ShortcutSignal) {
    log::debug!("shortcut signal queued: {signal:?}");
    if context.sender.send(signal).is_err() {
        log::error!("shortcut event dropped because the dispatcher stopped");
    }
}
