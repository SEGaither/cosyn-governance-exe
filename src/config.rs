use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProxyConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_provider")]
    pub provider: String,
    pub profile_dir: Option<String>,
    pub log_level: Option<String>,
    #[serde(default = "default_streaming_mode")]
    pub streaming_mode: String,
    #[serde(default)]
    pub multi_turn: bool,
    #[serde(default = "default_fallback")]
    pub fallback: String,
}

fn default_port() -> u16 { 8901 }
fn default_provider() -> String { "openai".to_string() }
fn default_streaming_mode() -> String { "buffer".to_string() }
fn default_fallback() -> String { "fail_closed".to_string() }

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            provider: default_provider(),
            profile_dir: None,
            log_level: None,
            streaming_mode: default_streaming_mode(),
            multi_turn: false,
            fallback: default_fallback(),
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
