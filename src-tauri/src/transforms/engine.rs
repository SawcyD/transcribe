use serde::{Deserialize, Serialize};

use crate::{
    cleanup::{backtracking::resolve_explicit_backtracking, OpenAiCompatibleCleanup},
    errors::AppError,
    security::{self, CredentialKind},
};

use super::presets::{label_for, prompt_for};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformRequest {
    pub text: String,
    pub transform_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformResponse {
    pub transform_id: String,
    pub original_text: String,
    pub transformed_text: String,
    pub provider: String,
}

pub async fn apply_transform(
    request: TransformRequest,
    settings: &crate::models::AppSettings,
) -> Result<TransformResponse, AppError> {
    let text = request.text.trim().to_string();
    if text.is_empty() {
        return Err(AppError::Cleanup("transform input cannot be empty".into()));
    }
    if text.chars().count() > 50_000 {
        return Err(AppError::Cleanup(
            "transform input exceeds the 50,000 character limit".into(),
        ));
    }
    let transform_id = request.transform_id.trim().to_ascii_lowercase();
    let transformed = if settings.cleanup_enabled {
        if let (Some(prompt), Ok(key)) = (
            prompt_for(&transform_id),
            tokio::task::spawn_blocking(|| security::get(CredentialKind::Cleanup))
                .await
                .map_err(|error| AppError::Credential(error.to_string()))?,
        ) {
            let provider = OpenAiCompatibleCleanup::new(
                settings.cleanup_endpoint.clone(),
                key,
                settings.cleanup_model.clone(),
            )?;
            match provider.transform(&text, prompt).await {
                Ok(value) => (value, "cleanup"),
                Err(error) => {
                    log::warn!(
                        "transform provider fallback; transform={}; category={}",
                        label_for(&transform_id),
                        error.payload().category
                    );
                    (local_transform(&transform_id, &text), "local-fallback")
                }
            }
        } else {
            (local_transform(&transform_id, &text), "local")
        }
    } else {
        (local_transform(&transform_id, &text), "local")
    };
    Ok(TransformResponse {
        transform_id,
        original_text: text,
        transformed_text: transformed.0.trim().to_string(),
        provider: transformed.1.into(),
    })
}

fn local_transform(id: &str, text: &str) -> String {
    match id {
        "polish" => {
            let cleaned = resolve_explicit_backtracking(text);
            let mut chars = cleaned.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => cleaned,
            }
        }
        "prompt_engineer" => {
            let cleaned = resolve_explicit_backtracking(text);
            format!("Objective\n{cleaned}\n\nRequirements\n- Preserve the stated intent.\n- Return an implementation-ready result.")
        }
        "developer_task" => {
            let cleaned = resolve_explicit_backtracking(text);
            format!("Objective\n{cleaned}\n\nRequirements\n- Preserve the existing technical context.\n- Keep identifiers and interfaces unchanged unless requested.\n\nAcceptance criteria\n- The requested behavior is implemented and testable.")
        }
        "bug_report" => {
            let cleaned = resolve_explicit_backtracking(text);
            format!("Summary\n{cleaned}\n\nExpected behavior\n- Describe the intended result.\n\nActual behavior\n- Describe the observed result.\n\nReproduction steps\n1. Reproduce the reported behavior.")
        }
        "commit_message" => {
            let cleaned = resolve_explicit_backtracking(text);
            let subject = cleaned.trim_end_matches(['.', '!', '?']);
            format!("chore: {subject}")
        }
        "documentation" => {
            let cleaned = resolve_explicit_backtracking(text);
            let heading = cleaned
                .split_whitespace()
                .take(7)
                .collect::<Vec<_>>()
                .join(" ");
            format!("# {heading}\n\n{cleaned}")
        }
        "fix_grammar" => resolve_explicit_backtracking(text),
        "make_concise" => text.split_whitespace().collect::<Vec<_>>().join(" "),
        "turn_into_list" => text
            .split(|character| matches!(character, '.' | ';' | '\n'))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .enumerate()
            .map(|(index, value)| format!("{}. {}", index + 1, value))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::local_transform;

    #[test]
    fn local_polish_preserves_explicit_correction() {
        assert_eq!(
            local_transform("polish", "Use the red button, scratch that, green button"),
            "Green button"
        );
    }

    #[test]
    fn local_prompt_engineer_has_a_stable_structure() {
        let result = local_transform("prompt_engineer", "Fix the scrolling frame");
        assert!(result.starts_with("Objective\nFix the scrolling frame"));
        assert!(result.contains("Requirements"));
    }

    #[test]
    fn local_developer_transforms_are_actionable() {
        assert!(local_transform("developer_task", "Fix the parser").contains("Acceptance criteria"));
        assert!(local_transform("bug_report", "The preview is blank").contains("Actual behavior"));
        assert_eq!(
            local_transform("commit_message", "Add a safer paste fallback."),
            "chore: Add a safer paste fallback"
        );
        assert!(local_transform("documentation", "Run cargo test").starts_with("# Run cargo test"));
    }
}
