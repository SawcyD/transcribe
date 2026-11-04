//! Spoken replies through Deepgram Aura, replacing the WebView's built-in
//! speech synthesis (which used whatever Windows voice happened to be
//! installed). The API key stays in Rust; the UI only receives audio.

use crate::{
    errors::AppError,
    security::{self, CredentialKind},
};

pub const DEFAULT_VOICE: &str = "aura-2-thalia-en";
/// Deepgram's per-request text limit for /v1/speak.
const MAX_CHARS: usize = 2_000;

/// Returns 16-bit mono PCM at `crate::sound::RATE` for `text`.
pub async fn speak(text: &str, voice: Option<&str>) -> Result<Vec<u8>, AppError> {
    let text = speakable(text);
    if text.is_empty() {
        return Err(AppError::Configuration("there is nothing to read aloud".into()));
    }
    let voice = voice
        .filter(|value| value.starts_with("aura-"))
        .unwrap_or(DEFAULT_VOICE);
    let api_key = tokio::task::spawn_blocking(|| security::get(CredentialKind::Deepgram))
        .await
        .map_err(|error| AppError::Credential(error.to_string()))??;
    let response = reqwest::Client::new()
        .post("https://api.deepgram.com/v1/speak")
        .query(&[
            ("model", voice),
            ("encoding", "linear16"),
            ("sample_rate", "24000"),
            ("container", "none"),
        ])
        .header("Authorization", format!("Token {api_key}"))
        .json(&serde_json::json!({ "text": text }))
        .send()
        .await
        .map_err(|error| AppError::Transcription(format!("Deepgram speech request failed: {error}")))?;
    if !response.status().is_success() {
        return Err(AppError::Transcription(format!(
            "Deepgram speech returned {}",
            response.status()
        )));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| AppError::Transcription(error.to_string()))?;
    Ok(bytes.to_vec())
}

/// Strips Markdown that would otherwise be read out literally ("asterisk
/// asterisk"), drops code blocks, and trims to Deepgram's limit at a
/// sentence boundary where possible.
pub fn speakable(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code_block = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if !in_code_block {
                out.push_str("(code omitted) ");
            }
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        let line = line.trim_start_matches(['#', '>', ' ']).trim_start_matches("- ");
        out.push_str(&line.replace(['*', '`', '_'], ""));
        out.push(' ');
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_CHARS {
        return collapsed;
    }
    let truncated: String = collapsed.chars().take(MAX_CHARS).collect();
    match truncated.rfind(['.', '!', '?']) {
        Some(end) if end > MAX_CHARS / 2 => truncated[..=end].to_string(),
        _ => truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::speakable;

    #[test]
    fn strips_markdown_and_code_blocks() {
        assert_eq!(
            speakable("## Fix\n**Restart** the `dev` server.\n```bash\npnpm dev\n```\n- then retry"),
            "Fix Restart the dev server. (code omitted) then retry"
        );
    }

    #[test]
    fn trims_long_text_at_a_sentence_boundary() {
        let long = "This is a sentence. ".repeat(200);
        let spoken = speakable(&long);
        assert!(spoken.len() <= 2_000);
        assert!(spoken.ends_with('.'));
    }
}
