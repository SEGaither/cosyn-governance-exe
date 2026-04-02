//! Coaching integration tests with a mock LLM provider.

use cosyn::coaching::generate_coaching;
use cosyn::dcc::types::BlockReasonCode;
use cosyn::provider::{LlmProvider, LlmRequest, LlmResponse};
use cosyn::core::errors::{CosynError, CosynResult};

struct MockCoachingProvider;

#[async_trait::async_trait]
impl LlmProvider for MockCoachingProvider {
    fn name(&self) -> &str {
        "mock-coaching"
    }

    async fn complete(&self, _request: &LlmRequest) -> CosynResult<LlmResponse> {
        Ok(LlmResponse {
            content: "Try being more specific about what you want.".into(),
            model: "mock".into(),
            input_tokens: 10,
            output_tokens: 10,
        })
    }
}

struct FailingProvider;

#[async_trait::async_trait]
impl LlmProvider for FailingProvider {
    fn name(&self) -> &str {
        "failing"
    }

    async fn complete(&self, _request: &LlmRequest) -> CosynResult<LlmResponse> {
        Err(CosynError::Orchestration("simulated provider failure".into()))
    }
}

#[tokio::test]
async fn coaching_on_subject_block() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("asdfghjkl", BlockReasonCode::BrSubjectUnknown, &provider).await;
    assert!(!result.is_empty());
    assert!(result.contains("specific"));
}

#[tokio::test]
async fn coaching_on_evidence_block() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("is the a", BlockReasonCode::BrEvidenceUnsat, &provider).await;
    assert!(!result.is_empty());
}

#[tokio::test]
async fn coaching_on_ambiguity_block() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("do the thing with the stuff", BlockReasonCode::BrAmbiguity, &provider).await;
    assert!(!result.is_empty());
}

#[tokio::test]
async fn coaching_fallback_on_llm_failure() {
    let provider = FailingProvider;
    let result = generate_coaching("asdfghjkl", BlockReasonCode::BrSubjectUnknown, &provider).await;
    assert_eq!(result, BlockReasonCode::BrSubjectUnknown.user_message());
}

#[tokio::test]
async fn no_coaching_on_system_gates() {
    // BrReleaseDenied has empty tips → returns static message without calling LLM
    let provider = MockCoachingProvider;
    let result = generate_coaching("anything", BlockReasonCode::BrReleaseDenied, &provider).await;
    assert_eq!(result, BlockReasonCode::BrReleaseDenied.user_message());
}
