use serde::{Deserialize, Serialize};

/// A user-configurable shortcut: a set of modifiers plus an optional main key.
///
/// Modifier-only bindings are supported deliberately — VoiceFlow's default
/// push-to-talk gesture is "hold Ctrl + Win" with no letter key at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    /// Any of "ctrl", "alt", "shift", "win". Order is not significant.
    pub modifiers: Vec<String>,
    /// Main key name, e.g. "Space", "B", "Escape". `None` for modifier-only.
    pub key: Option<String>,
}

impl ShortcutBinding {
    pub fn new(modifiers: &[&str], key: Option<&str>) -> Self {
        Self {
            modifiers: modifiers.iter().map(|value| (*value).to_string()).collect(),
            key: key.map(str::to_string),
        }
    }

    /// Canonical form used for equality and conflict checks, insensitive to the
    /// order the user pressed the modifiers in.
    pub fn canonical(&self) -> String {
        let mut modifiers: Vec<String> = self
            .modifiers
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect();
        modifiers.sort();
        modifiers.dedup();
        let key = self
            .key
            .as_deref()
            .map(str::to_ascii_uppercase)
            .unwrap_or_default();
        format!("{}+{}", modifiers.join("+"), key)
    }

    /// Human-readable label, e.g. "Ctrl + Win + Space".
    pub fn display(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        // Fixed order so the label reads the way Windows writes shortcuts.
        for name in ["ctrl", "alt", "shift", "win"] {
            if self
                .modifiers
                .iter()
                .any(|value| value.eq_ignore_ascii_case(name))
            {
                parts.push(match name {
                    "ctrl" => "Ctrl",
                    "alt" => "Alt",
                    "shift" => "Shift",
                    _ => "Win",
                });
            }
        }
        let mut label = parts.join(" + ");
        if let Some(key) = &self.key {
            if !label.is_empty() {
                label.push_str(" + ");
            }
            label.push_str(key);
        }
        label
    }

    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty() && self.key.is_none()
    }
}

/// The four rebindable dictation gestures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBindings {
    pub push_to_talk: ShortcutBinding,
    pub hands_free: ShortcutBinding,
    pub command_mode: ShortcutBinding,
    pub cancel: ShortcutBinding,
}

impl Default for ShortcutBindings {
    fn default() -> Self {
        Self {
            push_to_talk: ShortcutBinding::new(&["ctrl", "win"], None),
            hands_free: ShortcutBinding::new(&["ctrl", "win"], Some("Space")),
            command_mode: ShortcutBinding::new(&["ctrl", "alt"], Some("B")),
            cancel: ShortcutBinding::new(&[], Some("Escape")),
        }
    }
}

impl ShortcutBindings {
    pub fn entries(&self) -> [(ShortcutAction, &ShortcutBinding); 4] {
        [
            (ShortcutAction::PushToTalk, &self.push_to_talk),
            (ShortcutAction::HandsFree, &self.hands_free),
            (ShortcutAction::CommandMode, &self.command_mode),
            (ShortcutAction::Cancel, &self.cancel),
        ]
    }

    /// Returns the label of the first duplicated gesture, if any. Two actions
    /// firing on one gesture would make the dispatcher's behaviour ambiguous.
    pub fn conflict(&self) -> Option<String> {
        let mut seen: Vec<(String, &str)> = Vec::new();
        for (action, binding) in self.entries() {
            if binding.is_empty() {
                return Some(format!("{} has no shortcut assigned", action.label()));
            }
            let canonical = binding.canonical();
            if let Some((_, other)) = seen.iter().find(|(value, _)| value == &canonical) {
                return Some(format!(
                    "{} and {} both use {}",
                    other,
                    action.label(),
                    binding.display()
                ));
            }
            seen.push((canonical, action.label()));
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    PushToTalk,
    HandsFree,
    CommandMode,
    Cancel,
}

impl ShortcutAction {
    pub fn label(self) -> &'static str {
        match self {
            ShortcutAction::PushToTalk => "Push to talk",
            ShortcutAction::HandsFree => "Hands-free dictation",
            ShortcutAction::CommandMode => "Command Mode",
            ShortcutAction::Cancel => "Cancel",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_ignores_modifier_order_and_case() {
        let a = ShortcutBinding::new(&["win", "CTRL"], Some("space"));
        let b = ShortcutBinding::new(&["ctrl", "win"], Some("SPACE"));
        assert_eq!(a.canonical(), b.canonical());
    }

    #[test]
    fn display_uses_windows_modifier_order() {
        let binding = ShortcutBinding::new(&["win", "ctrl"], Some("Space"));
        assert_eq!(binding.display(), "Ctrl + Win + Space");
    }

    #[test]
    fn modifier_only_binding_has_no_trailing_separator() {
        let binding = ShortcutBinding::new(&["ctrl", "win"], None);
        assert_eq!(binding.display(), "Ctrl + Win");
    }

    #[test]
    fn defaults_do_not_conflict() {
        assert_eq!(ShortcutBindings::default().conflict(), None);
    }

    #[test]
    fn duplicate_gestures_are_reported() {
        let bindings = ShortcutBindings {
            push_to_talk: ShortcutBinding::new(&["ctrl", "alt"], Some("B")),
            ..ShortcutBindings::default()
        };
        assert!(bindings.conflict().is_some());
    }

    #[test]
    fn empty_binding_is_rejected() {
        let bindings = ShortcutBindings {
            cancel: ShortcutBinding::new(&[], None),
            ..ShortcutBindings::default()
        };
        assert!(bindings.conflict().is_some());
    }
}
