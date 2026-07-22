use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock, RwLock,
};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

use super::binding::{ShortcutAction, ShortcutBindings};
use super::keys::{key_to_vk, Modifier, VK_COUNT};
use crate::errors::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutSignal {
    pub action: ShortcutAction,
    /// `true` when the gesture became fully held, `false` when it was released.
    /// Hold-style actions use both edges; toggle-style actions ignore release.
    pub pressed: bool,
}

/// A binding reduced to the form the hook can evaluate on every keystroke.
struct CompiledBinding {
    action: ShortcutAction,
    modifiers: Vec<Modifier>,
    key: Option<u16>,
    /// Whether the gesture was satisfied on the previous evaluation, so we can
    /// emit transitions rather than a signal per key event.
    active: AtomicBool,
}

struct HookContext {
    sender: UnboundedSender<ShortcutSignal>,
    /// Physical key state indexed by virtual-key code.
    keys: Vec<AtomicBool>,
    bindings: RwLock<Vec<CompiledBinding>>,
}

static CONTEXT: OnceLock<HookContext> = OnceLock::new();

fn compile(bindings: &ShortcutBindings) -> Vec<CompiledBinding> {
    bindings
        .entries()
        .into_iter()
        .filter_map(|(action, binding)| {
            let modifiers = binding
                .modifiers
                .iter()
                .filter_map(|name| Modifier::parse(name))
                .collect::<Vec<_>>();
            let key = match binding.key.as_deref() {
                Some(name) => match key_to_vk(name) {
                    Some(vk) => Some(vk),
                    // An unknown key name would silently match nothing, so drop
                    // the binding and say so rather than appear to be bound.
                    None => {
                        log::warn!("ignoring shortcut with unrecognised key: {name}");
                        return None;
                    }
                },
                None => None,
            };
            if modifiers.is_empty() && key.is_none() {
                return None;
            }
            Some(CompiledBinding {
                action,
                modifiers,
                key,
                active: AtomicBool::new(false),
            })
        })
        .collect()
}

/// Replaces the live bindings. Called whenever settings are saved so rebinding
/// takes effect without restarting the hook thread.
pub fn update_bindings(bindings: &ShortcutBindings) {
    let Some(context) = CONTEXT.get() else {
        return;
    };
    match context.bindings.write() {
        Ok(mut guard) => *guard = compile(bindings),
        Err(_) => log::error!("shortcut binding table is poisoned; keeping previous bindings"),
    }
}

pub fn start_modifier_hook(
    bindings: &ShortcutBindings,
) -> Result<UnboundedReceiver<ShortcutSignal>, AppError> {
    let (sender, receiver) = unbounded_channel();
    CONTEXT
        .set(HookContext {
            sender,
            keys: (0..VK_COUNT).map(|_| AtomicBool::new(false)).collect(),
            bindings: RwLock::new(compile(bindings)),
        })
        .map_err(|_| AppError::Windows("shortcut keyboard hook was already initialized".into()))?;

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
                let vk = event.vkCode as usize;
                if vk < context.keys.len() {
                    context.keys[vk].store(down, Ordering::Relaxed);
                }
                evaluate(context);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

/// Recomputes every binding and emits an edge whenever one changes state.
fn evaluate(context: &HookContext) {
    let Ok(bindings) = context.bindings.read() else {
        return;
    };

    // A gesture with more keys wins: while Ctrl+Win+Space is held, Ctrl+Win is
    // also satisfied, and only the more specific one should fire.
    let best = bindings
        .iter()
        .filter(|binding| satisfied(context, binding))
        .map(|binding| binding.modifiers.len() + usize::from(binding.key.is_some()))
        .max();

    // A more-specific shortcut can be used to promote an active hold shortcut.
    // Queue press edges before release edges so Ctrl+Win+Space promotes a live
    // Ctrl+Win push-to-talk session instead of first finalizing it.
    let mut presses = Vec::new();
    let mut releases = Vec::new();

    for binding in bindings.iter() {
        let specificity = binding.modifiers.len() + usize::from(binding.key.is_some());
        let active = satisfied(context, binding) && Some(specificity) == best;
        if binding.active.swap(active, Ordering::Relaxed) != active {
            let signal = ShortcutSignal {
                action: binding.action,
                pressed: active,
            };
            if active {
                presses.push((specificity, signal));
            } else {
                releases.push((specificity, signal));
            }
        }
    }

    // Most-specific presses go first. Releases only clean up a previous hold
    // after the newly active action has had a chance to handle the gesture.
    presses.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, signal) in presses.into_iter().chain(releases) {
        log::debug!("shortcut signal queued: {signal:?}");
        if context.sender.send(signal).is_err() {
            log::error!("shortcut event dropped because the dispatcher stopped");
        }
    }
}

fn satisfied(context: &HookContext, binding: &CompiledBinding) -> bool {
    let held = |vk: u16| {
        context
            .keys
            .get(vk as usize)
            .is_some_and(|state| state.load(Ordering::Relaxed))
    };
    binding
        .modifiers
        .iter()
        .all(|modifier| modifier.virtual_keys().iter().copied().any(held))
        && binding.key.map(held).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(bindings: &ShortcutBindings) -> (HookContext, UnboundedReceiver<ShortcutSignal>) {
        let (sender, receiver) = unbounded_channel();
        (
            HookContext {
                sender,
                keys: (0..VK_COUNT).map(|_| AtomicBool::new(false)).collect(),
                bindings: RwLock::new(compile(bindings)),
            },
            receiver,
        )
    }

    fn set(context: &HookContext, vk: u16, held: bool) {
        context.keys[vk as usize].store(held, Ordering::Relaxed);
    }

    #[test]
    fn promotion_press_precedes_push_to_talk_release() {
        let bindings = ShortcutBindings::default();
        let (context, mut receiver) = context(&bindings);

        set(&context, Modifier::Ctrl.virtual_keys()[0], true);
        set(&context, Modifier::Win.virtual_keys()[0], true);
        evaluate(&context);
        assert_eq!(
            receiver.try_recv().unwrap(),
            ShortcutSignal {
                action: ShortcutAction::PushToTalk,
                pressed: true,
            }
        );

        set(&context, key_to_vk("Space").unwrap(), true);
        evaluate(&context);
        assert_eq!(
            receiver.try_recv().unwrap(),
            ShortcutSignal {
                action: ShortcutAction::HandsFree,
                pressed: true,
            }
        );
        assert_eq!(
            receiver.try_recv().unwrap(),
            ShortcutSignal {
                action: ShortcutAction::PushToTalk,
                pressed: false,
            }
        );
    }
}
