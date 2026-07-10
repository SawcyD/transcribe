use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{errors::AppError, models::CleanupResult};

use super::prompt_builder::{
    cleanup_style_instruction, protected_terms_instruction, BASE_CLEANUP_SYSTEM_PROMPT,
};

#[async_trait]
pub trait CleanupProvider: Send + Sync {
    async fn cleanup(
        &self,
        text: &str,
        protected_terms: &[String],
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

    pub async fn cleanup_with_style(
        &self,
        text: &str,
        protected_terms: &[String],
        style: &str,
    ) -> Result<CleanupResult, AppError> {
        if text_too_long(text, 50_000) {
            return Err(AppError::Cleanup(
                "transcript exceeds the 50,000 character cleanup limit".into(),
            ));
        }
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
                    content: text.to_string(),
                },
            ],
            temperature: 0.0,
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
        let result: CleanupResult =
            serde_json::from_str(strip_code_fence(&content)).map_err(|error| {
                AppError::Cleanup(format!("cleanup structured output was invalid: {error}"))
            })?;
        if result.cleaned_text.trim().is_empty() {
            return Err(AppError::Cleanup(
                "cleanup provider returned empty text".into(),
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

#[async_trait]
impl CleanupProvider for OpenAiCompatibleCleanup {
    async fn cleanup(
        &self,
        text: &str,
        protected_terms: &[String],
    ) -> Result<CleanupResult, AppError> {
        self.cleanup_with_style(text, protected_terms, "balanced")
            .await
    }
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
