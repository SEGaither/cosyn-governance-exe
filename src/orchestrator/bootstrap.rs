pub fn bootstrap() -> Result<(), String> {
    if std::env::var("OPENAI_API_KEY").unwrap_or_default().is_empty() {
        return Err("OPENAI_API_KEY not set — set it in your environment to use CoSyn.".into());
    }

    Ok(())
}
