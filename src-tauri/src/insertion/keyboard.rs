use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_INSERT, VK_RETURN, VK_SHIFT, VK_TAB, VK_V,
};

use crate::{errors::AppError, models::PostPasteAction};

pub fn send_paste(terminal: bool) -> Result<(), AppError> {
    if terminal {
        send_chord(VK_SHIFT, VK_INSERT)
    } else {
        send_chord(VK_CONTROL, VK_V)
    }
}

pub fn send_post_action(action: PostPasteAction) -> Result<(), AppError> {
    match action {
        PostPasteAction::None => Ok(()),
        PostPasteAction::Enter | PostPasteAction::Newline => send_key(VK_RETURN),
        PostPasteAction::Tab => send_key(VK_TAB),
    }
}

fn send_chord(modifier: VIRTUAL_KEY, key: VIRTUAL_KEY) -> Result<(), AppError> {
    send(&[
        keyboard_input(modifier, false),
        keyboard_input(key, false),
        keyboard_input(key, true),
        keyboard_input(modifier, true),
    ])
}

fn send_key(key: VIRTUAL_KEY) -> Result<(), AppError> {
    send(&[keyboard_input(key, false), keyboard_input(key, true)])
}

fn keyboard_input(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> Result<(), AppError> {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(AppError::Insertion(format!(
            "Windows accepted {sent} of {} keyboard inputs",
            inputs.len()
        )))
    }
}
