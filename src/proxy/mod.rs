pub mod handlers;
pub mod middleware;
pub mod types;

use crate::config::ProxyConfig;
use crate::provider::LlmProvider;
use std::sync::Arc;

pub struct ProxyState {
    pub config: ProxyConfig,
    pub provider: Arc<dyn LlmProvider>,
}

impl ProxyState {
    pub fn test_default() -> Self {
        Self {
            config: ProxyConfig::default(),
            provider: Arc::new(crate::provider::openai::OpenAiProvider::new()),
        }
    }
}

pub fn build_router(state: ProxyState) -> axum::Router {
    use axum::routing::{get, post};

    let shared_state = Arc::new(state);

    axum::Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .layer(axum::middleware::from_fn(middleware::log_request))
        .with_state(shared_state)
}
