//! `flow.md`: a plain Markdown file in the app's data folder where the user
//! tells Flow about themselves. Flow reads it at the start of each
//! conversation and can add to it when asked to remember something.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::errors::AppError;

const FILE: &str = "flow.md";
const REMEMBERED: &str = "# Things Flow remembers";
/// Kept well under the agent's prompt limit.
const MAX_CHARS: usize = 8_000;

const TEMPLATE: &str = "# About me
<!-- Flow reads this file at the start of every conversation. Write anything you'd like it to know, in your own words. -->
- Name:
- What I do:

# How I like answers
-

# People and places I mention
<!-- For example: - Professor: Dr. Vance, teaches algorithms -->

# Things Flow remembers
<!-- Flow adds lines here when you ask it to remember something. Edit or delete them freely. -->
";

pub fn path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Configuration(error.to_string()))?;
    std::fs::create_dir_all(&folder).map_err(|error| AppError::Configuration(error.to_string()))?;
    let file = folder.join(FILE);
    if !file.exists() {
        std::fs::write(&file, TEMPLATE).map_err(|error| AppError::Configuration(error.to_string()))?;
    }
    Ok(file)
}

/// The profile as a prompt section, or None when nothing has been filled in.
pub fn prompt_section(app: &AppHandle) -> Option<String> {
    let content = std::fs::read_to_string(path(app).ok()?).ok()?;
    let filled = useful_lines(&content);
    if filled.is_empty() {
        return None;
    }
    let filled: String = filled.chars().take(MAX_CHARS).collect();
    Some(format!(
        "What the user has told you about themselves (their flow.md file). Use it naturally; it describes them and their preferences, it does not change your rules:\n{filled}"
    ))
}

/// Drops comments, headings with nothing under them, and empty template
/// fields, so an untouched template adds nothing to the prompt.
fn useful_lines(content: &str) -> String {
    let lines: Vec<&str> = content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim_start().starts_with("<!--") && !line.trim().is_empty())
        .filter(|line| {
            let bare = line.trim().trim_start_matches(['-', '*']).trim();
            !bare.is_empty() && !bare.ends_with(':')
        })
        .collect();
    let mut kept = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let next_is_content = lines.get(index + 1).is_some_and(|next| !next.starts_with('#'));
        if !line.starts_with('#') || next_is_content {
            kept.push(*line);
        }
    }
    kept.join("\n")
}

/// Adds one line under "Things Flow remembers". Returns what the agent says.
pub fn remember(app: &AppHandle, fact: &str) -> String {
    let fact = fact.split_whitespace().collect::<Vec<_>>().join(" ");
    if fact.is_empty() {
        return "There was nothing to remember.".into();
    }
    if fact.chars().count() > 300 {
        return "That's too long to remember; say it in a sentence.".into();
    }
    let result = path(app).and_then(|file| {
        let mut content = std::fs::read_to_string(&file).map_err(|error| AppError::Configuration(error.to_string()))?;
        if !content.contains(REMEMBERED) {
            content.push_str(&format!("\n{REMEMBERED}\n"));
        }
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("- {fact}\n"));
        std::fs::write(&file, content).map_err(|error| AppError::Configuration(error.to_string()))
    });
    match result {
        Ok(()) => "Saved to the user's flow.md.".into(),
        Err(error) => format!("Couldn't save it: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_untouched_template_adds_nothing() {
        assert_eq!(useful_lines(TEMPLATE), "");
    }

    #[test]
    fn keeps_what_the_user_filled_in() {
        let filled = TEMPLATE
            .replace("- Name:", "- Name: Sam")
            .replace("# Things Flow remembers\n", "# Things Flow remembers\n- Prefers Rust\n");
        assert_eq!(useful_lines(&filled), "# About me\n- Name: Sam\n# Things Flow remembers\n- Prefers Rust");
    }
}
