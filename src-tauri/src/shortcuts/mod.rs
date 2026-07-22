mod binding;
mod keys;
mod registry;

pub use binding::{ShortcutAction, ShortcutBindings};
pub use registry::{start_modifier_hook, update_bindings};
