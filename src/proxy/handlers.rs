use crate::proxy::types::*;
use crate::proxy::ProxyState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

pub async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
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
                }),
            };
            Ok(Json(response))
        }
        Err(e) => {
            // Breach behavior: Annotate — return as normal response with breach metadata
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
                        content: format!("[GOVERNANCE BREACH] {}", e),
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
                    breach: Some(e.to_string()),
                }),
            };
            Ok(Json(response))
        }
    }
}
