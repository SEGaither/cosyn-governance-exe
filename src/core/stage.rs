#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Input,
    Draft,
    Validation,
    Lock,
    Output,
}

impl Stage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Draft => "draft",
            Self::Validation => "validation",
            Self::Lock => "lock",
            Self::Output => "output",
        }
    }

    pub fn sequence() -> &'static [Stage] {
        &[
            Stage::Input,
            Stage::Draft,
            Stage::Validation,
            Stage::Lock,
            Stage::Output,
        ]
    }
}
