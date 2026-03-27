fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version") {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.iter().any(|a| a == "--help") {
        println!("cosyn --version | --help");
        return;
    }

    let bundle = cosyn::authority_loader::load_embedded_authorities();
    if let Err(e) = cosyn::authority_loader::validate_authorities(&bundle) {
        eprintln!("{}", e);
        std::process::exit(1);
    }

    let startup_warning = match cosyn::orchestrator::bootstrap::bootstrap() {
        Ok(()) => None,
        Err(e) => {
            eprintln!("warning: {}", e);
            Some(e)
        }
    };

    if let Err(e) = cosyn::ui_runtime::launch_with_status(startup_warning) {
        eprintln!("fatal: {}", e);
    }
}

