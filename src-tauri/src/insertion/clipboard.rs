use std::{thread, time::Duration};

use arboard::Clipboard;

use crate::{
    errors::AppError,
    models::{ActiveTarget, PostPasteAction},
};

use super::{focus, keyboard, terminal};

pub fn copy_text(text: &str) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|error| AppError::Insertion(format!("clipboard is unavailable: {error}")))?;
    let text = super::context::contextualize_spacing(text, None, None);
    clipboard
        .set_text(text)
        .map_err(|error| AppError::Insertion(format!("clipboard write failed: {error}")))
}

pub fn paste_into_target(
    text: &str,
    target: &ActiveTarget,
    action: PostPasteAction,
    delay_ms: u64,
    restore_clipboard: bool,
) -> Result<(), AppError> {
    let mut clipboard = Clipboard::new()
        .map_err(|error| AppError::Insertion(format!("clipboard is unavailable: {error}")))?;
    let previous_text = if restore_clipboard {
        clipboard.get_text().ok()
    } else {
        None
    };
    clipboard
        .set_text(text.to_string())
        .map_err(|error| AppError::Insertion(format!("clipboard write failed: {error}")))?;
    if let Err(error) = focus::restore(target) {
        return Err(error);
    }
    thread::sleep(Duration::from_millis(35));
    keyboard::send_paste(terminal::is_terminal(target.process_name.as_deref()))?;
    thread::sleep(Duration::from_millis(delay_ms));
    keyboard::send_post_action(action)?;
    if let Some(previous) = previous_text {
        thread::sleep(Duration::from_millis(80));
        let _ = clipboard.set_text(previous);
    }
    Ok(())
}
