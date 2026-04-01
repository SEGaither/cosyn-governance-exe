use crate::core::errors::{CosynError, CosynResult};
use crate::provider::{LlmProvider, LlmRequest, LlmResponse};

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            base_url: "http://localhost:11434".into(),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str { "ollama" }

    async fn complete(&self, request: &LlmRequest) -> CosynResult<LlmResponse> {
        let model = request.model.clone()
            .unwrap_or_else(|| "llama3".into());

        let messages: Vec<serde_json::Value> = request.messages.iter()
            .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.3),
                "num_predict": request.max_tokens.unwrap_or(1024),
            }
        });

        let resp = self.client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| CosynError::Draft(format!("Ollama request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CosynError::Draft(format!("Ollama error {}: {}", status, text)));
        }

        let resp_json: serde_json::Value = resp.json().await
            .map_err(|e| CosynError::Draft(format!("Response parse error: {}", e)))?;

        let content = resp_json["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err(CosynError::Draft("Ollama returned empty content".into()));
        }

        Ok(LlmResponse {
            content,
            model,
            input_tokens: 0,
            output_tokens: 0,
        })
    }
}
