mod active_window;
mod clipboard;
mod context;
mod focus;
mod keyboard;
mod terminal;

pub use active_window::capture_active_target;
pub use clipboard::{copy_text, paste_into_target};
