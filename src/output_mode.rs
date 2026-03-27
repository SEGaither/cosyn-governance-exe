pub enum OutputMode {
    Standard,
    Artifact,
}

pub struct RenderContext {
    pub mode: OutputMode,
}

impl RenderContext {
    pub fn is_artifact(&self) -> bool {
        matches!(self.mode, OutputMode::Artifact)
    }
}

pub fn render_output(ctx: &RenderContext, content: &str) -> String {
    match ctx.mode {
        OutputMode::Artifact => content.to_string(),
        OutputMode::Standard => content.to_string(),
    }
}
