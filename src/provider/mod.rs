pub mod openai;
pub mod anthropic;
pub mod ollama;

use crate::core::errors::CosynResult;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<LlmMessage>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, request: &LlmRequest) -> CosynResult<LlmResponse>;
}

pub fn create_provider(name: &str) -> Result<Box<dyn LlmProvider>, String> {
    match name {
        "openai" => Ok(Box::new(openai::OpenAiProvider::new())),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new())),
        "ollama" => Ok(Box::new(ollama::OllamaProvider::new())),
        _ => Err(format!("Unknown provider: {}", name)),
    }
}
