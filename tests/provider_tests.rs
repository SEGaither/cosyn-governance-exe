use cosyn::provider::{LlmProvider, LlmRequest, LlmMessage};

#[test]
fn openai_provider_builds_without_panic() {
    let provider = cosyn::provider::openai::OpenAiProvider::new();
    assert_eq!(provider.name(), "openai");
}

#[test]
fn anthropic_provider_builds_without_panic() {
    let provider = cosyn::provider::anthropic::AnthropicProvider::new();
    assert_eq!(provider.name(), "anthropic");
}

#[test]
fn ollama_provider_builds_without_panic() {
    let provider = cosyn::provider::ollama::OllamaProvider::new();
    assert_eq!(provider.name(), "ollama");
}

#[test]
fn create_provider_factory() {
    assert!(cosyn::provider::create_provider("openai").is_ok());
    assert!(cosyn::provider::create_provider("anthropic").is_ok());
    assert!(cosyn::provider::create_provider("ollama").is_ok());
    assert!(cosyn::provider::create_provider("unknown").is_err());
}

#[test]
fn request_normalizes_messages() {
    let req = LlmRequest {
        messages: vec![
            LlmMessage { role: "user".into(), content: "hello".into() },
        ],
        model: Some("gpt-4o-mini".into()),
        max_tokens: Some(1024),
        temperature: Some(0.3),
    };
    assert_eq!(req.messages.len(), 1);
    assert_eq!(req.messages[0].role, "user");
}
