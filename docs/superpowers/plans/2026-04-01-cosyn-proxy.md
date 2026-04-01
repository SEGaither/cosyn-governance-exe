# cosyn.exe Governance Proxy — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform cosyn from a desktop GUI app into a local HTTP proxy that sits between any LLM client and any LLM provider, governing both requests and responses through the DCC pipeline.

**Architecture:** cosyn.exe runs as a local HTTP server exposing OpenAI-compatible API endpoints. User configures their tool to point at `localhost:<port>`. cosyn receives requests, runs input governance (DCC gates), forwards to the real LLM API, runs output governance (structural + semantic grounding, constitutional enforcement), and returns the governed response. Blocked requests/responses return structured error responses.

**Tech Stack:** Rust, axum (HTTP server), tokio (async runtime), reqwest (async HTTP client), serde/serde_json, toml (config), clap (CLI args). Removes eframe/egui dependency entirely.

---

## Codebase Understanding

### What exists (v4.1.1)
- **DCC pipeline** (`src/orchestrator/mod.rs`): Subject binding → Evidence → Ambiguity → Version truth → Draft (LLM) → Structural grounding → Semantic grounding → Release → Block evaluation. With revision loop (up to 3 attempts).
- **Governance layer** (`src/governance_layer/mod.rs`): Constitutional enforcement — min/max length, sentinel detection, verbatim echo, empty structure.
- **LLM client** (`src/llm_client/mod.rs`): Blocking reqwest, hardcoded to OpenAI gpt-4o-mini.
- **Authority loader** (`src/authority_loader.rs`): Partially refactored — supports embedded (`include_str!`) and external directory loading. `load_authorities_from_dir()` exists but is not wired into main/cli.
- **Telemetry** (`src/telemetry/mod.rs`, `src/dcc/telemetry.rs`): Stage logging, DCC metrics, file output.
- **Audit** (`src/audit/mod.rs`): Append-only audit record log.
- **UI** (`src/ui_runtime/mod.rs`): eframe/egui desktop GUI — **will be removed**.
- **CLI** (`src/cli.rs`): Takes a single prompt argument, runs pipeline, prints output.
- **Tests**: 30 passing (14 unit, 16 integration), 2 ignored (require API key).

### What changes
- **Remove**: `eframe` dependency, `src/ui_runtime/mod.rs`, GUI binary target
- **Add**: `axum`, `tokio`, `clap`, `toml` dependencies
- **Refactor**: `llm_client` to async + provider abstraction
- **Refactor**: `orchestrator` pipeline to work with proxy request/response model
- **Add**: HTTP proxy server, config system, multi-provider support
- **Keep**: All DCC logic, governance layer, authority loader, telemetry, audit, all existing tests

### File Map

#### New files
- `src/proxy/mod.rs` — HTTP server setup, routing
- `src/proxy/handlers.rs` — Request handlers (chat completions, health)
- `src/proxy/middleware.rs` — Tower middleware for logging/telemetry
- `src/proxy/types.rs` — OpenAI-compatible request/response types
- `src/provider/mod.rs` — Provider trait and factory
- `src/provider/openai.rs` — OpenAI provider (refactored from llm_client)
- `src/provider/anthropic.rs` — Anthropic provider
- `src/provider/ollama.rs` — Ollama provider
- `src/config.rs` — TOML config loading, CLI arg parsing
- `tests/proxy_integration.rs` — Proxy endpoint tests
- `tests/provider_tests.rs` — Provider abstraction tests
- `cosyn.toml.example` — Example config file

#### Modified files
- `Cargo.toml` — dependency changes
- `src/lib.rs` — new module declarations
- `src/main.rs` — replace GUI launch with proxy server launch
- `src/cli.rs` — update to use config/profile-dir args
- `src/orchestrator/mod.rs` — make pipeline usable from proxy context
- `src/orchestrator/bootstrap.rs` — expand bootstrap for proxy mode
- `src/llm_client/mod.rs` — refactor to async, extract to provider
- `src/dcc/version.rs` — decouple from ui_runtime (version truth source changes)

#### Removed files
- `src/ui_runtime/mod.rs` — GUI no longer needed
- `src/output_mode.rs` — artifact mode was GUI-specific

---

## Design Decisions

1. **OpenAI-compatible API first.** Most tools support custom OpenAI endpoints. Exposing `/v1/chat/completions` gives maximum compatibility with zero client changes beyond a URL.

2. **Non-streaming v1.** Streaming (SSE) requires buffering the full response before governance can run, which complicates the architecture. v1 returns complete responses. Streaming is a v2 enhancement.

3. **Provider trait, not protocol translation.** cosyn accepts OpenAI-format requests regardless of backend. The provider layer translates to/from each LLM's native format. The governance pipeline always works on a normalized internal representation.

4. **Config file + CLI overrides.** `cosyn.toml` holds persistent config (port, provider, API key reference). CLI flags override for ad-hoc use. Environment variables for API keys (never in config file).

5. **Governance on both sides.** Input governance (DCC gates) runs on the user's message before it reaches the LLM. Output governance (structural + semantic + constitutional) runs on the LLM's response before it reaches the user. This is the key value proposition.

6. **Blocked responses return structured errors.** When governance blocks a request or response, cosyn returns a valid OpenAI-format response with the block reason in the message content and a `cosyn-governance-blocked: true` header. The user's tool doesn't crash — it shows the block message as if it were a normal response.

7. **Version truth adapts.** Currently version truth compares runtime version to UI version. In proxy mode, there's no UI. Version truth will compare runtime version to config-declared version (or be satisfied by default when no UI is present).

---

## Phase 1: Foundation — Async HTTP Proxy Skeleton

### Task 1: Update dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update Cargo.toml**

Replace eframe with proxy dependencies. Keep reqwest but switch from blocking to async.

```toml
[package]
name = "cosyn"
version = "5.0.0"
edition = "2021"
authors = ["Shane Gaither"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
log = "0.4"
env_logger = "0.10"
uuid = { version = "1", features = ["v4"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
axum = "0.8"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
clap = { version = "4", features = ["derive"] }
toml = "0.8"

[[bin]]
name = "cosyn"
path = "src/main.rs"

[profile.release]
opt-level = 3
```

- [ ] **Step 2: Verify compilation after dependency change**

Run: `cargo check 2>&1`
Expected: Compilation errors from ui_runtime references — these are expected and will be fixed in Task 2.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: replace eframe with axum/tokio proxy dependencies, bump to v5.0.0"
```

### Task 2: Remove GUI modules

**Files:**
- Remove: `src/ui_runtime/mod.rs`
- Remove: `src/output_mode.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/orchestrator/mod.rs`
- Modify: `src/dcc/version.rs`

- [ ] **Step 1: Write failing test — version truth without UI**

```rust
// In tests/dcc_enforcement.rs, add:
#[test]
fn version_truth_ok_when_both_from_cargo() {
    let v = cosyn::dcc::version::RUNTIME_VERSION;
    let truth = cosyn::dcc::version::evaluate_version_truth(v, v);
    assert_eq!(truth, cosyn::dcc::types::VersionTruth::Ok);
}
```

- [ ] **Step 2: Run test to verify it passes (this one should pass already)**

Run: `cargo test version_truth_ok_when_both_from_cargo -- --nocapture 2>&1`
Expected: PASS

- [ ] **Step 3: Remove ui_runtime and output_mode modules**

Delete `src/ui_runtime/mod.rs` and `src/output_mode.rs`.

Update `src/lib.rs` — remove `pub mod ui_runtime;` and `pub mod output_mode;`.

- [ ] **Step 4: Update main.rs — minimal placeholder**

```rust
fn main() {
    println!("cosyn v{}", env!("CARGO_PKG_VERSION"));
}
```

- [ ] **Step 5: Update orchestrator to remove ui_runtime reference**

In `src/orchestrator/mod.rs`, replace:
```rust
let ui_version = crate::ui_runtime::APP_VERSION;
```
with:
```rust
let ui_version = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 6: Verify all existing tests still pass**

Run: `cargo test 2>&1`
Expected: All 30 non-ignored tests pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: remove GUI modules, decouple version truth from ui_runtime"
```

### Task 3: Config system

**Files:**
- Create: `src/config.rs`
- Create: `cosyn.toml.example`
- Modify: `src/lib.rs`
- Test: `tests/config_tests.rs`

- [ ] **Step 1: Write failing test — config parsing**

```rust
// tests/config_tests.rs
use cosyn::config::ProxyConfig;

#[test]
fn parse_minimal_config() {
    let toml_str = r#"
        port = 8080
        provider = "openai"
    "#;
    let config: ProxyConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.port, 8080);
    assert_eq!(config.provider, "openai");
}

#[test]
fn config_defaults() {
    let config = ProxyConfig::default();
    assert_eq!(config.port, 8901);
    assert_eq!(config.provider, "openai");
    assert!(config.profile_dir.is_none());
}

#[test]
fn parse_full_config() {
    let toml_str = r#"
        port = 9000
        provider = "anthropic"
        profile_dir = "governance/profiles"
        log_level = "debug"
    "#;
    let config: ProxyConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.port, 9000);
    assert_eq!(config.provider, "anthropic");
    assert_eq!(config.profile_dir.as_deref(), Some("governance/profiles"));
    assert_eq!(config.log_level.as_deref(), Some("debug"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test config_tests 2>&1`
Expected: FAIL — `config` module doesn't exist.

- [ ] **Step 3: Implement config module**

```rust
// src/config.rs
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_provider")]
    pub provider: String,
    pub profile_dir: Option<String>,
    pub log_level: Option<String>,
}

fn default_port() -> u16 { 8901 }
fn default_provider() -> String { "openai".to_string() }

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            provider: default_provider(),
            profile_dir: None,
            log_level: None,
        }
    }
}

impl ProxyConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read config file {}: {}", path.display(), e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Invalid config: {}", e))
    }
}
```

Add `pub mod config;` to `src/lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test config_tests 2>&1`
Expected: PASS

- [ ] **Step 5: Create example config**

```toml
# cosyn.toml — CoSyn governance proxy configuration
# Copy to cosyn.toml and edit for your environment.

# Port the proxy listens on
port = 8901

# LLM provider: "openai", "anthropic", "ollama"
provider = "openai"

# Optional: directory containing external governance artifacts
# profile_dir = "governance/profiles"

# Log level: "error", "warn", "info", "debug", "trace"
# log_level = "info"
```

- [ ] **Step 6: Commit**

```bash
git add src/config.rs cosyn.toml.example tests/config_tests.rs src/lib.rs
git commit -m "feat: add TOML config system with defaults and file loading"
```

### Task 4: CLI argument parsing

**Files:**
- Modify: `src/main.rs`
- Modify: `src/config.rs`

- [ ] **Step 1: Write failing test — CLI merges with config**

```rust
// Add to tests/config_tests.rs
#[test]
fn cli_overrides_config() {
    let mut config = ProxyConfig::default();
    config.port = 8080;
    // Simulate CLI override
    config.apply_override_port(Some(9999));
    assert_eq!(config.port, 9999);
}

#[test]
fn cli_none_preserves_config() {
    let mut config = ProxyConfig::default();
    config.port = 8080;
    config.apply_override_port(None);
    assert_eq!(config.port, 8080);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test cli_overrides 2>&1`
Expected: FAIL

- [ ] **Step 3: Add override methods to ProxyConfig**

```rust
// Add to src/config.rs
impl ProxyConfig {
    pub fn apply_override_port(&mut self, port: Option<u16>) {
        if let Some(p) = port {
            self.port = p;
        }
    }

    pub fn apply_override_provider(&mut self, provider: Option<String>) {
        if let Some(p) = provider {
            self.provider = p;
        }
    }

    pub fn apply_override_profile_dir(&mut self, dir: Option<String>) {
        if dir.is_some() {
            self.profile_dir = dir;
        }
    }
}
```

- [ ] **Step 4: Implement main.rs with clap**

```rust
// src/main.rs
use clap::Parser;
use cosyn::config::ProxyConfig;

#[derive(Parser)]
#[command(name = "cosyn", version, about = "CoSyn governance proxy")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "cosyn.toml")]
    config: String,

    /// Port to listen on (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// LLM provider (overrides config)
    #[arg(long)]
    provider: Option<String>,

    /// Directory containing governance profile artifacts
    #[arg(long)]
    profile_dir: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    env_logger::init();

    // Load config file (use defaults if not found)
    let mut config = match std::path::Path::new(&cli.config).exists() {
        true => ProxyConfig::from_file(std::path::Path::new(&cli.config))
            .unwrap_or_else(|e| {
                eprintln!("warning: {}", e);
                ProxyConfig::default()
            }),
        false => ProxyConfig::default(),
    };

    // Apply CLI overrides
    config.apply_override_port(cli.port);
    config.apply_override_provider(cli.provider);
    config.apply_override_profile_dir(cli.profile_dir);

    // Validate authorities
    let bundle = if let Some(ref dir) = config.profile_dir {
        cosyn::authority_loader::load_authorities_from_dir(std::path::Path::new(dir))
            .unwrap_or_else(|e| {
                log::warn!("External authorities failed: {}. Using embedded.", e);
                cosyn::authority_loader::load_embedded_authorities()
            })
    } else {
        cosyn::authority_loader::load_embedded_authorities()
    };

    if let Err(e) = cosyn::authority_loader::validate_authorities(&bundle) {
        eprintln!("FATAL: {}", e);
        std::process::exit(1);
    }

    println!("cosyn v{} — governance proxy", env!("CARGO_PKG_VERSION"));
    println!("  provider: {}", config.provider);
    println!("  port: {}", config.port);
    if let Some(ref dir) = config.profile_dir {
        println!("  profile_dir: {}", dir);
    }
    println!("  status: ready");
    println!();
    println!("Point your LLM client at: http://localhost:{}/v1/chat/completions", config.port);
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test 2>&1`
Expected: All tests pass.

- [ ] **Step 6: Verify binary runs**

Run: `cargo run -- --help 2>&1`
Expected: Shows help with --config, --port, --provider, --profile-dir options.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/config.rs tests/config_tests.rs
git commit -m "feat: add clap CLI with config file loading and override support"
```

---

## Phase 2: Provider Abstraction

### Task 5: Provider trait

**Files:**
- Create: `src/provider/mod.rs`
- Create: `src/provider/openai.rs`
- Modify: `src/lib.rs`
- Test: `tests/provider_tests.rs`

- [ ] **Step 1: Write failing test — provider trait contract**

```rust
// tests/provider_tests.rs
use cosyn::provider::{LlmProvider, LlmRequest, LlmMessage};

#[test]
fn openai_provider_builds_without_panic() {
    let provider = cosyn::provider::openai::OpenAiProvider::new();
    assert_eq!(provider.name(), "openai");
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test provider_tests 2>&1`
Expected: FAIL

- [ ] **Step 3: Implement provider trait and types**

```rust
// src/provider/mod.rs
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
```

Note: Add `async-trait = "0.1"` to Cargo.toml dependencies.

- [ ] **Step 4: Implement OpenAI provider (refactor from llm_client)**

```rust
// src/provider/openai.rs
use crate::core::errors::{CosynError, CosynResult};
use crate::provider::{LlmProvider, LlmRequest, LlmResponse, LlmMessage};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_MODEL: &str = "gpt-4o-mini";
const TIMEOUT_SECS: u64 = 30;

pub struct OpenAiProvider {
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }

    pub fn name(&self) -> &str { "openai" }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<LlmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    model: String,
    usage: Option<Usage>,
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

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }

    async fn complete(&self, request: &LlmRequest) -> CosynResult<LlmResponse> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| CosynError::Draft("OPENAI_API_KEY not set".into()))?;

        let body = ChatRequest {
            model: request.model.clone().unwrap_or_else(|| DEFAULT_MODEL.into()),
            messages: request.messages.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let resp = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| CosynError::Draft(format!("API request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(CosynError::Draft(format!("API error {}: {}", status, text)));
        }

        let chat: ChatResponse = resp.json().await
            .map_err(|e| CosynError::Draft(format!("Response parse error: {}", e)))?;

        let content = chat.choices.first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
            .to_string();

        if content.is_empty() {
            return Err(CosynError::Draft("API returned empty content".into()));
        }

        let (input_tokens, output_tokens) = match chat.usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens),
            None => (0, 0),
        };

        Ok(LlmResponse {
            content,
            model: chat.model,
            input_tokens,
            output_tokens,
        })
    }
}
```

- [ ] **Step 5: Implement stub Anthropic provider**

```rust
// src/provider/anthropic.rs
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

        // Build Anthropic-native request format
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
```

- [ ] **Step 6: Implement stub Ollama provider**

```rust
// src/provider/ollama.rs
use crate::core::errors::{CosynError, CosynResult};
use crate::provider::{LlmProvider, LlmRequest, LlmResponse, LlmMessage};

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
```

- [ ] **Step 7: Run tests**

Run: `cargo test provider_tests 2>&1`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/provider/ tests/provider_tests.rs Cargo.toml src/lib.rs
git commit -m "feat: add provider abstraction with OpenAI, Anthropic, Ollama implementations"
```

---

## Phase 3: HTTP Proxy Server

### Task 6: Proxy skeleton with health endpoint

**Files:**
- Create: `src/proxy/mod.rs`
- Create: `src/proxy/handlers.rs`
- Create: `src/proxy/types.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Test: `tests/proxy_integration.rs`

- [ ] **Step 1: Write failing test — health endpoint**

```rust
// tests/proxy_integration.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let resp = app
        .oneshot(Request::builder().uri("/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test proxy_integration 2>&1`
Expected: FAIL

- [ ] **Step 3: Implement proxy types**

```rust
// src/proxy/types.rs
use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat completion request
#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub stream: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// OpenAI-compatible chat completion response
#[derive(Debug, Serialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: ChatUsage,
}

#[derive(Debug, Serialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize)]
pub struct ChatUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}
```

- [ ] **Step 4: Implement proxy module and handlers**

```rust
// src/proxy/mod.rs
pub mod handlers;
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
        .with_state(shared_state)
}
```

```rust
// src/proxy/handlers.rs
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
    // Streaming not supported in v1
    if request.stream.unwrap_or(false) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Streaming not supported. Set stream: false".into(),
        ));
    }

    // TODO: Input governance, LLM call, output governance
    // Placeholder — will be wired in Task 7
    Err((StatusCode::NOT_IMPLEMENTED, "Governance pipeline not yet wired".into()))
}
```

Add `pub mod proxy;` to `src/lib.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test proxy_integration 2>&1`
Expected: PASS (health returns 200, unknown route returns 404)

- [ ] **Step 6: Update main.rs to start the server**

```rust
// Replace the println-only main with actual server startup
#[tokio::main]
async fn main() {
    // ... (keep existing CLI parsing and authority loading from Task 4) ...

    let provider = cosyn::provider::create_provider(&config.provider)
        .unwrap_or_else(|e| {
            eprintln!("FATAL: {}", e);
            std::process::exit(1);
        });

    let state = cosyn::proxy::ProxyState {
        config: config.clone(),
        provider: std::sync::Arc::from(provider),
    };

    let app = cosyn::proxy::build_router(state);
    let addr = format!("0.0.0.0:{}", config.port);

    println!("cosyn v{} — governance proxy", env!("CARGO_PKG_VERSION"));
    println!("  provider: {}", config.provider);
    println!("  listening: http://localhost:{}", config.port);
    println!("  endpoint: http://localhost:{}/v1/chat/completions", config.port);

    let listener = tokio::net::TcpListener::bind(&addr).await
        .unwrap_or_else(|e| {
            eprintln!("FATAL: cannot bind to {}: {}", addr, e);
            std::process::exit(1);
        });

    axum::serve(listener, app).await
        .unwrap_or_else(|e| {
            eprintln!("FATAL: server error: {}", e);
            std::process::exit(1);
        });
}
```

- [ ] **Step 7: Run all tests**

Run: `cargo test 2>&1`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/proxy/ src/main.rs src/lib.rs tests/proxy_integration.rs
git commit -m "feat: add HTTP proxy skeleton with health endpoint and OpenAI-compatible route"
```

---

## Phase 4: Wire Governance Pipeline into Proxy

### Task 7: Input governance + LLM call + output governance

**Files:**
- Modify: `src/proxy/handlers.rs`
- Modify: `src/orchestrator/mod.rs`
- Test: `tests/proxy_integration.rs`

This is the core task. The proxy handler needs to:
1. Extract user message from OpenAI-format request
2. Run DCC input gates (subject binding, evidence, ambiguity)
3. Forward to LLM provider
4. Run DCC output gates (structural, semantic, constitutional enforcement)
5. Return governed response in OpenAI format

- [ ] **Step 1: Write failing test — input governance blocks empty message**

```rust
// Add to tests/proxy_integration.rs
use axum::http::header;

#[tokio::test]
async fn empty_message_returns_governance_block() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": ""}]
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Should return 200 with governance block message in the response
    // (not a 4xx — the client tool should see a "response", not an error)
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let content = resp_json["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(content.contains("BR-"), "Expected block reason code in response");
}
```

- [ ] **Step 2: Write failing test — valid request passes through (requires API key, mark ignored)**

```rust
#[tokio::test]
#[ignore] // requires OPENAI_API_KEY
async fn valid_request_returns_governed_response() {
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "What is the capital of France?"}]
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let content = resp_json["choices"][0]["message"]["content"].as_str().unwrap();
    assert!(!content.is_empty());
    assert!(resp_json["usage"]["total_tokens"].as_u64().unwrap() > 0);
}
```

- [ ] **Step 3: Refactor orchestrator to expose a proxy-compatible pipeline function**

Add a new function to `src/orchestrator/mod.rs` that accepts a provider and returns results without depending on the blocking llm_client:

```rust
// Add to src/orchestrator/mod.rs
use crate::provider::{LlmProvider, LlmRequest, LlmMessage, LlmResponse};

/// Async governance pipeline for proxy mode.
/// Runs input gates, calls the provider, runs output gates.
pub async fn run_governed(
    input: &str,
    provider: &dyn LlmProvider,
) -> CosynResult<LockedOutput> {
    // Input governance (reuse existing DCC logic)
    let mut ctrl = RuntimeControl::new();
    crate::telemetry::log_event("input_received", input);

    // DCC input gates (same as existing run())
    // ... [Subject binding, evidence, ambiguity, version truth, reasoning permission]
    // ... [Same logic as current run(), extracted into helper or inlined]

    // LLM call via provider
    let llm_request = LlmRequest {
        messages: vec![
            LlmMessage { role: "system".into(), content: "You are a controlled drafting engine. Produce a concise, direct response to the user input. Do not include placeholders, filler, or speculative content.".into() },
            LlmMessage { role: "user".into(), content: input.into() },
        ],
        model: None,
        max_tokens: Some(1024),
        temperature: Some(0.3),
    };

    let llm_response = provider.complete(&llm_request).await?;
    let draft = crate::core::types::DraftOutput { text: llm_response.content };

    // Output governance (same structural + semantic + release logic)
    // ... [Same as existing run() post-draft logic]

    Ok(LockedOutput {
        text: draft.text,
        locked: true,
        block_reason_code: None,
    })
}
```

Note: The actual implementation will extract the shared DCC gate logic into helper functions to avoid duplicating the ~200 lines of gate checks between `run()` and `run_governed()`. The blocking `run()` stays for backward compatibility with the CLI binary.

- [ ] **Step 4: Implement chat_completions handler with governance**

```rust
// Replace placeholder in src/proxy/handlers.rs
pub async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, (StatusCode, String)> {
    if request.stream.unwrap_or(false) {
        return Err((StatusCode::BAD_REQUEST, "Streaming not supported. Set stream: false".into()));
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
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
            };
            Ok(Json(response))
        }
        Err(e) => {
            // Return governance block as a normal response (not HTTP error)
            // so the client tool displays it gracefully
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
                        content: format!("[GOVERNANCE BLOCKED] {}", e),
                    },
                    finish_reason: "stop".into(),
                }],
                usage: ChatUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
            };
            Ok(Json(response))
        }
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test 2>&1`
Expected: All tests pass including new proxy governance tests.

- [ ] **Step 6: Commit**

```bash
git add src/orchestrator/mod.rs src/proxy/handlers.rs tests/proxy_integration.rs
git commit -m "feat: wire DCC governance pipeline into proxy chat completions endpoint"
```

---

## Phase 5: Telemetry & Breach Logging

### Task 8: Structured proxy telemetry

**Files:**
- Modify: `src/proxy/handlers.rs`
- Modify: `src/telemetry/mod.rs`
- Create: `src/proxy/middleware.rs`

- [ ] **Step 1: Write failing test — request logs telemetry**

```rust
// Add to tests/proxy_integration.rs
#[tokio::test]
async fn blocked_request_emits_telemetry() {
    cosyn::telemetry::take_log(); // clear
    let app = cosyn::proxy::build_router(cosyn::proxy::ProxyState::test_default());
    let body = serde_json::json!({
        "messages": [{"role": "user", "content": ""}]
    });
    let _ = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let log = cosyn::telemetry::take_log();
    let joined = log.join("\n");
    assert!(joined.contains("input_received"), "missing input_received");
    assert!(joined.contains("final_release_decision"), "missing final_release_decision");
}
```

- [ ] **Step 2: Verify the test — should pass if governance already emits telemetry (it does)**

Run: `cargo test blocked_request_emits_telemetry 2>&1`
Expected: PASS (existing telemetry calls in orchestrator already cover this)

- [ ] **Step 3: Add request logging middleware**

```rust
// src/proxy/middleware.rs
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn log_request<B>(request: Request<B>, next: Next<B>) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    log::info!("{} {}", method, uri);
    let response = next.run(request).await;
    log::info!("{} {} -> {}", method, uri, response.status());
    response
}
```

- [ ] **Step 4: Commit**

```bash
git add src/proxy/middleware.rs src/telemetry/mod.rs tests/proxy_integration.rs
git commit -m "feat: add request logging middleware and verify telemetry on governed requests"
```

---

## Phase 6: Polish & Distribution

### Task 9: Remove old CLI binary target, clean up

**Files:**
- Modify: `Cargo.toml` — remove `[[bin]]` for cosyn-cli
- Modify: `src/cli.rs` — either remove or convert to a thin wrapper
- Modify: `src/llm_client/mod.rs` — keep for backward compat or remove

- [ ] **Step 1: Decide on CLI binary**

The CLI can remain as a convenience tool for testing governance without running the proxy. Keep it but have it use the same config system.

- [ ] **Step 2: Update cli.rs to use config and provider**

Update to use `ProxyConfig`, `--profile-dir`, and the provider abstraction. Keep it as a simple "send one prompt, get governed response" tool.

- [ ] **Step 3: Run all tests**

Run: `cargo test 2>&1`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/cli.rs
git commit -m "refactor: update CLI binary to use config system and provider abstraction"
```

### Task 10: GitHub Actions release workflow

**Files:**
- Create: `.github/workflows/release.yml`
- Create: `.github/workflows/test.yml`

- [ ] **Step 1: Create test workflow**

```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test
```

- [ ] **Step 2: Create release workflow**

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: cosyn
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: cosyn.exe
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: cosyn
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - uses: actions/upload-artifact@v4
        with:
          name: cosyn-${{ matrix.target }}
          path: target/release/${{ matrix.artifact }}
  release:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
      - uses: softprops/action-gh-release@v2
        with:
          files: cosyn-*/*
```

- [ ] **Step 3: Commit**

```bash
git add .github/
git commit -m "ci: add test and release GitHub Actions workflows"
```

---

## Deferred: Future Enhancements (Not in This Plan)

- **Streaming support (SSE)**: Buffer-then-govern pattern for streaming responses
- **Google Gemini provider**: Similar pattern to Anthropic
- **Anthropic-native endpoint**: Expose `/v1/messages` in addition to OpenAI-compatible
- **Profile hot-reloading**: Watch profile directory for changes, reload without restart
- **Dashboard/status page**: Simple HTML page at `/` showing config, recent telemetry
- **Binary obfuscation**: Release build hardening
- **Code signing**: Windows Authenticode / macOS notarization
- **Breach dashboard**: Accumulated breach data with patterns

---

## Verification Checklist

Before declaring any phase complete:
- [ ] All existing tests still pass (`cargo test`)
- [ ] New tests cover the new functionality
- [ ] `cargo clippy` produces no warnings
- [ ] Binary compiles in release mode (`cargo build --release`)
- [ ] Binary starts and responds to `/health`
- [ ] Empty input is governance-blocked through the proxy
- [ ] Telemetry file is written on proxy requests
