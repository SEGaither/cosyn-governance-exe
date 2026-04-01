use crate::dcc::types::BlockReasonCode;
use crate::proxy::types::*;
use crate::proxy::ProxyState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

pub async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

/// Parse a BlockReasonCode from an error string containing a BR-* code.
fn parse_block_reason(error: &str) -> Option<BlockReasonCode> {
    if error.contains("BR-SUBJECT-UNKNOWN") {
        Some(BlockReasonCode::BrSubjectUnknown)
    } else if error.contains("BR-EVIDENCE-UNSAT") {
        Some(BlockReasonCode::BrEvidenceUnsat)
    } else if error.contains("BR-AMBIGUITY") {
        Some(BlockReasonCode::BrAmbiguity)
    } else if error.contains("BR-STRUCTURAL-FAIL") {
        Some(BlockReasonCode::BrStructuralFail)
    } else if error.contains("BR-GROUNDING-FAIL") {
        Some(BlockReasonCode::BrGroundingFail)
    } else if error.contains("BR-VERSION-CONFLICT") {
        Some(BlockReasonCode::BrVersionConflict)
    } else if error.contains("BR-VERSION-UNDEFINED") {
        Some(BlockReasonCode::BrVersionUndefined)
    } else if error.contains("BR-RELEASE-DENIED") {
        Some(BlockReasonCode::BrReleaseDenied)
    } else {
        None
    }
}

/// Input gate codes that benefit from user coaching.
fn is_input_gate(code: BlockReasonCode) -> bool {
    matches!(
        code,
        BlockReasonCode::BrSubjectUnknown
            | BlockReasonCode::BrEvidenceUnsat
            | BlockReasonCode::BrAmbiguity
    )
}

pub async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, String)> {
    if request.stream.unwrap_or(false) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Streaming not yet supported. Set stream: false".into(),
        ));
    }

    // Extract the last user message
    let user_message = request.messages.iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();

    // Run governed pipeline
    match crate::orchestrator::run_governed(&user_message, state.provider.as_ref()).await {
        Ok(output) => {
            let response = ChatCompletionResponse {
                id: format!("cosyn-{}", uuid::Uuid::new_v4()),
                object: "chat.completion".into(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                model: request.model.unwrap_or_else(|| "governed".into()),
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: output.text,
                    },
                    finish_reason: "stop".into(),
                }],
                usage: ChatUsage {
                    prompt_tokens: output.input_tokens,
                    completion_tokens: output.output_tokens,
                    total_tokens: output.input_tokens + output.output_tokens,
                },
                cosyn_metadata: Some(CosynMetadata {
                    governed: true,
                    breach: output.block_reason_code.map(|c| c.code().to_string()),
                    coaching: None,
                }),
            };
            Ok(Json(response))
        }
        Err(e) => {
            let error_str = e.to_string();
            let block_code = parse_block_reason(&error_str);

            // Generate coaching for input gate failures
            let coaching = match block_code {
                Some(code) if is_input_gate(code) => {
                    Some(
                        crate::coaching::generate_coaching(
                            &user_message,
                            code,
                            state.provider.as_ref(),
                        )
                        .await,
                    )
                }
                _ => None,
            };

            let display_message = if let Some(ref coaching_text) = coaching {
                format!("[GOVERNANCE BREACH] {}\n\n{}", error_str, coaching_text)
            } else {
                format!("[GOVERNANCE BREACH] {}", error_str)
            };

            let response = ChatCompletionResponse {
                id: format!("cosyn-{}", uuid::Uuid::new_v4()),
                object: "chat.completion".into(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                model: "cosyn-governance".into(),
                choices: vec![ChatChoice {
                    index: 0,
                    message: ChatMessage {
                        role: "assistant".into(),
                        content: display_message,
                    },
                    finish_reason: "stop".into(),
                }],
                usage: ChatUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                cosyn_metadata: Some(CosynMetadata {
                    governed: true,
                    breach: Some(error_str),
                    coaching,
                }),
            };
            Ok(Json(response))
        }
    }
}
