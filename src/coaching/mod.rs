pub mod tips;

use crate::dcc::types::BlockReasonCode;
use crate::provider::{LlmMessage, LlmProvider, LlmRequest};

/// Generate coaching guidance for a user whose input was blocked.
/// Calls the LLM with gate-relevant tips to produce a helpful, specific suggestion.
/// Falls back to the static `user_message()` if the LLM call fails.
pub async fn generate_coaching(
    original_input: &str,
    block_code: BlockReasonCode,
    provider: &dyn LlmProvider,
) -> String {
    let tips = tips::tips_for_gate(block_code);

    // No coaching tips for this gate type — return static message
    if tips.is_empty() {
        return block_code.user_message().to_string();
    }

    let tip_text: String = tips
        .iter()
        .map(|t| format!("**{}**: {}", t.name, t.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let system_prompt = format!(
        "You are a coaching assistant for the CoSyn governance system. A user's input was \
        blocked by the {} gate (reason: {}). Your job is to help them understand why and \
        suggest how to revise their prompt.\n\n\
        Use these principles to guide your advice:\n\n{}\n\n\
        Be concise (2-3 sentences). Be specific to their actual input. \
        Do not repeat the block reason code. Do not apologize.",
        block_code.code(),
        block_code.user_message(),
        tip_text
    );

    let request = LlmRequest {
        messages: vec![
            LlmMessage {
                role: "system".into(),
                content: system_prompt,
            },
            LlmMessage {
                role: "user".into(),
                content: format!(
                    "My prompt was blocked. Here is what I wrote:\n\n\"{}\"",
                    original_input
                ),
            },
        ],
        model: None,
        max_tokens: Some(256),
        temperature: Some(0.5),
    };

    match provider.complete(&request).await {
        Ok(response) => response.content,
        Err(_) => block_code.user_message().to_string(),
    }
}
