use crate::core::errors::{CosynError, CosynResult};
use crate::provider::{LlmProvider, LlmRequest, LlmResponse, LlmMessage};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_MODEL: &str = "gpt-4o-mini";
const TIMEOUT_SECS: u64 = 30;

pub struct OpenAiProvider {
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<LlmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    model: String,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }

    async fn complete(&self, request: &LlmRequest) -> CosynResult<LlmResponse> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| CosynError::Draft("OPENAI_API_KEY not set".into()))?;

        let body = ChatRequest {
            model: request.model.clone().unwrap_or_else(|| DEFAULT_MODEL.into()),
            messages: request.messages.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| CosynError::Draft(format!("API request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CosynError::Draft(format!("API error {}: {}", status, text)));
        }

        let chat: ChatResponse = resp.json().await
            .map_err(|e| CosynError::Draft(format!("Response parse error: {}", e)))?;

        let content = chat.choices.first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err(CosynError::Draft("API returned empty content".into()));
        }

        let (input_tokens, output_tokens) = match chat.usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens),
            None => (0, 0),
        };

        Ok(LlmResponse {
            content,
            model: chat.model,
            input_tokens,
            output_tokens,
        })
    }
}
