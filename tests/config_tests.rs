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
    assert_eq!(config.streaming_mode, "buffer");
    assert!(!config.multi_turn);
    assert_eq!(config.fallback, "fail_closed");
}

#[test]
fn parse_full_config() {
    let toml_str = r#"
        port = 9000
        provider = "anthropic"
        profile_dir = "governance/profiles"
        log_level = "debug"
        streaming_mode = "stream_hold"
        multi_turn = true
        fallback = "passthrough"
    "#;
    let config: ProxyConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.port, 9000);
    assert_eq!(config.provider, "anthropic");
    assert_eq!(config.profile_dir.as_deref(), Some("governance/profiles"));
    assert_eq!(config.log_level.as_deref(), Some("debug"));
    assert_eq!(config.streaming_mode, "stream_hold");
    assert!(config.multi_turn);
    assert_eq!(config.fallback, "passthrough");
}

#[test]
fn cli_overrides_config() {
    let mut config = ProxyConfig::default();
    config.port = 8080;
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
