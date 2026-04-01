use clap::Parser;
use cosyn::config::ProxyConfig;
use std::sync::Arc;

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

#[tokio::main]
async fn main() {
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

    let provider = cosyn::provider::create_provider(&config.provider)
        .unwrap_or_else(|e| {
            eprintln!("FATAL: {}", e);
            std::process::exit(1);
        });

    let state = cosyn::proxy::ProxyState {
        config: config.clone(),
        provider: Arc::from(provider),
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
