use crate::core::errors::{CosynError, CosynResult};
use crate::core::types::DraftOutput;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const SYSTEM_PROMPT: &str =
    "You are a controlled drafting engine. Produce a concise, direct response to the user input. Do not include placeholders, filler, or speculative content.";

const MODEL: &str = "gpt-4o-mini";
const MAX_TOKENS: u32 = 1024;
const TEMPERATURE: f64 = 0.3;
const TIMEOUT_SECS: u64 = 30;
const COST_PER_1K_INPUT: f64 = 0.00015;
const COST_PER_1K_OUTPUT: f64 = 0.0006;

static REQ_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_TOKENS: AtomicU64 = AtomicU64::new(0);

fn estimate_tokens(text: &str) -> u64 {
    // ~4 chars per token is a standard rough estimate
    (text.len() as u64 + 3) / 4
}

#[derive(Serialize)]
struct ChatRequest {
    model: &'static str,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f64,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
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

pub fn draft(input: &str) -> CosynResult<DraftOutput> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| CosynError::Draft("OPENAI_API_KEY not set".into()))?;

    let body = ChatRequest {
        model: MODEL,
        messages: vec![
            Message {
                role: "system",
                content: SYSTEM_PROMPT.into(),
            },
            Message {
                role: "user",
                content: input.into(),
            },
        ],
        max_tokens: MAX_TOKENS,
        temperature: TEMPERATURE,
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| CosynError::Draft(format!("HTTP client error: {}", e)))?;

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .map_err(|e| CosynError::Draft(format!("API request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        return Err(CosynError::Draft(format!("API error {}: {}", status, text)));
    }

    let chat: ChatResponse = resp
        .json()
        .map_err(|e| CosynError::Draft(format!("response parse error: {}", e)))?;

    let text = chat
        .choices
        .first()
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        return Err(CosynError::Draft("API returned empty content".into()));
    }

    // Usage telemetry
    let input_tokens = estimate_tokens(SYSTEM_PROMPT) + estimate_tokens(input);
    let output_tokens = estimate_tokens(&text);
    let call_tokens = input_tokens + output_tokens;
    TOTAL_TOKENS.fetch_add(call_tokens, Ordering::Relaxed);
    let reqs = REQ_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let total_tok = TOTAL_TOKENS.load(Ordering::Relaxed);
    let est_cost = (input_tokens as f64 * COST_PER_1K_INPUT
        + output_tokens as f64 * COST_PER_1K_OUTPUT)
        / 1000.0
        * reqs as f64;
    println!(
        "  USAGE → req_count: {} | est_tokens: {} | est_cost: ${:.6}",
        reqs, total_tok, est_cost
    );
    if reqs > 50 {
        println!("  WARNING → approaching request limit");
    }
    if total_tok > 50_000 {
        println!("  WARNING → approaching token limit");
    }

    Ok(DraftOutput { text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // requires OPENAI_API_KEY — run with: cargo test -- --ignored
    fn live_api_returns_draft() {
        let result = draft("What is 2 + 2?");
        assert!(result.is_ok(), "API call failed: {:?}", result.err());
        let output = result.unwrap();
        assert!(!output.text.is_empty());
    }
}
