fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|a| a == "--help") || args.len() < 2 {
        eprintln!("Usage: cosyn-cli <prompt>");
        eprintln!("       cosyn-cli --version");
        eprintln!("       cosyn-cli --help");
        if args.len() < 2 {
            std::process::exit(1);
        }
        return;
    }

    // Authority validation
    let bundle = cosyn::authority_loader::load_embedded_authorities();
    if let Err(e) = cosyn::authority_loader::validate_authorities(&bundle) {
        eprintln!("FATAL: {}", e);
        std::process::exit(1);
    }

    // API key check
    if std::env::var("OPENAI_API_KEY").unwrap_or_default().is_empty() {
        eprintln!("ERROR: OPENAI_API_KEY not set — set it in your environment to use CoSyn.");
        std::process::exit(1);
    }

    let input = &args[1];
    // Clear any prior telemetry
    cosyn::telemetry::take_log();

    let exit_code = match cosyn::orchestrator::run(input) {
        Ok(output) => {
            println!("{}", output.text);
            0
        }
        Err(e) => {
            eprintln!("BLOCKED: {}", e);
            1
        }
    };

    let stage_lines = cosyn::telemetry::take_log();
    let dcc_lines = cosyn::dcc::telemetry::take_dcc_log();
    cosyn::telemetry::flush_to_file(&stage_lines, &dcc_lines);

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
}
