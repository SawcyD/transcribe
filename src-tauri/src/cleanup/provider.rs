use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{errors::AppError, models::CleanupResult};

use super::prompt_builder::{
    build_user_message, cleanup_style_instruction, protected_terms_instruction,
    BASE_CLEANUP_SYSTEM_PROMPT,
};

#[async_trait]
pub trait CleanupProvider: Send + Sync {
    #[allow(dead_code)]
    async fn cleanup(
        &self,
        text: &str,
        protected_terms: &[String],
    ) -> Result<CleanupResult, AppError>;
    
    async fn cleanup_with_context(
        &self,
        text: &str,
        protected_terms: &[String],
        context: Option<&str>,
        dictionary_terms: Option<&[String]>,
        style: &str,
    ) -> Result<CleanupResult, AppError>;
}

pub struct OpenAiCompatibleCleanup {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleCleanup {
    pub fn new(endpoint: String, api_key: String, model: String) -> Result<Self, AppError> {
        if text_too_long(&endpoint, 2_048) || text_too_long(&model, 200) {
            return Err(AppError::Configuration(
                "cleanup provider configuration exceeds its size limit".into(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .map_err(|error| AppError::Cleanup(error.to_string()))?,
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key,
            model,
        })
    }

    /// Run a named transform through the same OpenAI-compatible boundary used
    /// for cleanup. The model returns plain text because transform results are
    /// previewed before any replacement occurs.
    pub async fn transform(&self, text: &str, system_prompt: &str) -> Result<String, AppError> {
        if text_too_long(text, 50_000) {
            return Err(AppError::Cleanup(
                "transform input exceeds the 50,000 character limit".into(),
            ));
        }
        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user",
                    content: text.to_string(),
                },
            ],
            temperature: 0.2,
            response_format: None,
        };
        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    AppError::Cleanup("transform provider timed out".into())
                } else {
                    AppError::Cleanup(error.to_string())
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::Cleanup(match status.as_u16() {
                401 | 403 => "cleanup provider rejected the API key".into(),
                429 => "cleanup provider rate limit reached".into(),
                value => format!("cleanup provider returned HTTP {value}"),
            }));
        }
        let body: ChatResponse = response.json().await.map_err(|error| {
            AppError::Cleanup(format!("transform response was invalid: {error}"))
        })?;
        let content = body
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::Cleanup("transform provider returned no text".into()))?;
        if text_too_long(&content, 100_000) {
            return Err(AppError::Cleanup(
                "transform output exceeded the safety limit".into(),
            ));
        }
        Ok(content)
    }

    #[allow(dead_code)]
    pub async fn cleanup_with_style(
        &self,
        text: &str,
        protected_terms: &[String],
        style: &str,
    ) -> Result<CleanupResult, AppError> {
        self.cleanup_with_context(text, protected_terms, None, None, style)
            .await
    }
}

#[async_trait]
impl CleanupProvider for OpenAiCompatibleCleanup {
    async fn cleanup(
        &self,
        text: &str,
        protected_terms: &[String],
    ) -> Result<CleanupResult, AppError> {
        self.cleanup_with_context(text, protected_terms, None, None, "balanced")
            .await
    }

    async fn cleanup_with_context(
        &self,
        text: &str,
        protected_terms: &[String],
        context: Option<&str>,
        dictionary_terms: Option<&[String]>,
        style: &str,
    ) -> Result<CleanupResult, AppError> {
        if text_too_long(text, 50_000) {
            return Err(AppError::Cleanup(
                "transcript exceeds the 50,000 character cleanup limit".into(),
            ));
        }

        // Build structured user message with XML tags
        let user_message = build_user_message(text, context, dictionary_terms, Some(style));

        let request = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: BASE_CLEANUP_SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "system",
                    content: protected_terms_instruction(protected_terms),
                },
                ChatMessage {
                    role: "system",
                    content: cleanup_style_instruction(style),
                },
                ChatMessage {
                    role: "user",
                    content: user_message,
                },
            ],
            temperature: 0.1, // Low temperature for deterministic syntactic cleanup
            response_format: Some(ResponseFormat {
                format_type: "json_object",
            }),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.endpoint))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    AppError::Cleanup("cleanup provider timed out".into())
                } else {
                    AppError::Cleanup(error.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(AppError::Cleanup(match status.as_u16() {
                401 | 403 => "cleanup provider rejected the API key".into(),
                429 => "cleanup provider rate limit reached".into(),
                value => format!("cleanup provider returned HTTP {value}"),
            }));
        }

        let body: ChatResponse = response
            .json()
            .await
            .map_err(|error| AppError::Cleanup(format!("cleanup response was invalid: {error}")))?;

        let content = body
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| AppError::Cleanup("cleanup provider returned no choices".into()))?;

        // Post-processing guardrails
        let cleaned_content = sanitize_llm_output(&content);

        let result: CleanupResult =
            serde_json::from_str(strip_code_fence(&cleaned_content)).map_err(|error| {
                AppError::Cleanup(format!("cleanup structured output was invalid: {error}"))
            })?;

        // Hallucination / Refusal Guard: If output is empty or contains meta-refusals,
        // the caller should fall back to deterministic cleanup
        if result.cleaned_text.trim().is_empty()
            || is_refusal_or_meta(&result.cleaned_text)
        {
            return Err(AppError::Cleanup(
                "cleanup provider returned empty or refusal response".into(),
            ));
        }

        if text_too_long(&result.cleaned_text, 100_000) {
            return Err(AppError::Cleanup(
                "cleanup output exceeded the safety limit".into(),
            ));
        }

        Ok(result)
    }
}

/// Sanitize LLM output by stripping accidental wrapper tags and quotes
fn sanitize_llm_output(output: &str) -> String {
    let mut cleaned = output.trim().to_string();

    // Strip <cleaned>...</cleaned> wrapper if model echoes it
    if let Some(stripped) = cleaned
        .strip_prefix("<cleaned>")
        .and_then(|s| s.strip_suffix("</cleaned>"))
    {
        cleaned = stripped.trim().to_string();
    }

    // Strip <raw_transcript>...</raw_transcript> wrapper if model echoes it
    if let Some(stripped) = cleaned
        .strip_prefix("<raw_transcript>")
        .and_then(|s| s.strip_suffix("</raw_transcript>"))
    {
        cleaned = stripped.trim().to_string();
    }

    // Strip outer quotes if the raw transcript was not quoted
    if (cleaned.starts_with('"') && cleaned.ends_with('"')
        || cleaned.starts_with('\'') && cleaned.ends_with('\''))
        && cleaned.len() >= 2
    {
        cleaned = cleaned[1..cleaned.len() - 1].to_string();
    }

    cleaned
}

/// Detect meta-refusals and AI chatter that should trigger fallback
fn is_refusal_or_meta(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    lower.starts_with("i cannot")
        || lower.starts_with("i can't")
        || lower.starts_with("sure, here is")
        || lower.starts_with("here is the")
        || lower.starts_with("as an ai")
        || lower.starts_with("as a language model")
        || lower.starts_with("i'm unable")
        || lower.starts_with("i am unable")
        || lower.starts_with("i apologize")
        || lower.starts_with("i'm sorry")
}

fn text_too_long(value: &str, limit: usize) -> bool {
    value.chars().count() > limit
}

fn strip_code_fence(value: &str) -> &str {
    value
        .trim()
        .strip_prefix("```json")
        .or_else(|| value.trim().strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or_else(|| value.trim())
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}