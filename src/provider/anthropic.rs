use crate::core::errors::{CosynError, CosynResult};
use crate::provider::{LlmProvider, LlmRequest, LlmResponse};

pub struct AnthropicProvider {
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str { "anthropic" }

    async fn complete(&self, request: &LlmRequest) -> CosynResult<LlmResponse> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| CosynError::Draft("ANTHROPIC_API_KEY not set".into()))?;

        let model = request.model.clone()
            .unwrap_or_else(|| "claude-sonnet-4-6".into());

        let messages: Vec<serde_json::Value> = request.messages.iter()
            .filter(|m| m.role != "system")
            .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect();

        let system = request.messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(1024),
        });

        if let Some(sys) = system {
            body["system"] = serde_json::json!(sys);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let resp = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| CosynError::Draft(format!("API request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CosynError::Draft(format!("API error {}: {}", status, text)));
        }

        let resp_json: serde_json::Value = resp.json().await
            .map_err(|e| CosynError::Draft(format!("Response parse error: {}", e)))?;

        let content = resp_json["content"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|b| b["text"].as_str())
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err(CosynError::Draft("API returned empty content".into()));
        }

        let input_tokens = resp_json["usage"]["input_tokens"].as_u64().unwrap_or(0);
        let output_tokens = resp_json["usage"]["output_tokens"].as_u64().unwrap_or(0);

        Ok(LlmResponse {
            content,
            model,
            input_tokens,
            output_tokens,
        })
    }
}
