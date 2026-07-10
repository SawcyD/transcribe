use regex::Regex;

use crate::models::PostPasteAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceActionResult {
    pub text: String,
    pub action: PostPasteAction,
    pub finish_requested: bool,
    pub cancel_requested: bool,
}

pub fn extract_trailing_action(input: &str, enter_enabled: bool) -> VoiceActionResult {
    let action = Regex::new(r"(?i)(?:[,.;—-]\s*|\s+)(press enter|submit|press tab|new line|finish dictation|cancel dictation)[.!?]*\s*$").expect("static regex is valid");
    let Some(captures) = action.captures(input) else {
        return VoiceActionResult {
            text: input.trim().to_string(),
            action: PostPasteAction::None,
            finish_requested: false,
            cancel_requested: false,
        };
    };
    let phrase = captures
        .get(1)
        .map(|value| value.as_str().to_lowercase())
        .unwrap_or_default();
    let full = captures.get(0).expect("full match exists");
    let text = input[..full.start()].trim_end().to_string();
    match phrase.as_str() {
        "press enter" | "submit" if enter_enabled => VoiceActionResult {
            text,
            action: PostPasteAction::Enter,
            finish_requested: false,
            cancel_requested: false,
        },
        "press tab" => VoiceActionResult {
            text,
            action: PostPasteAction::Tab,
            finish_requested: false,
            cancel_requested: false,
        },
        "new line" => VoiceActionResult {
            text,
            action: PostPasteAction::Newline,
            finish_requested: false,
            cancel_requested: false,
        },
        "finish dictation" => VoiceActionResult {
            text,
            action: PostPasteAction::None,
            finish_requested: true,
            cancel_requested: false,
        },
        "cancel dictation" => VoiceActionResult {
            text,
            action: PostPasteAction::None,
            finish_requested: false,
            cancel_requested: true,
        },
        _ => VoiceActionResult {
            text: input.trim().to_string(),
            action: PostPasteAction::None,
            finish_requested: false,
            cancel_requested: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_is_only_recognized_at_the_end() {
        let result = extract_trailing_action("Explain what press enter means in a terminal", true);
        assert_eq!(result.action, PostPasteAction::None);
        let result = extract_trailing_action("Run the command, press enter.", true);
        assert_eq!(result.text, "Run the command");
        assert_eq!(result.action, PostPasteAction::Enter);
    }

    #[test]
    fn enter_respects_the_safety_setting() {
        assert_eq!(
            extract_trailing_action("Run it, press enter", false).action,
            PostPasteAction::None
        );
    }
}
