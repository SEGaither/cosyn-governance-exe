use std::fmt;

#[derive(Debug)]
pub enum CosynError {
    Input(String),
    Draft(String),
    Validation(String),
    Governance(String),
    Lock(String),
    Orchestration(String),
}

impl fmt::Display for CosynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(msg) => write!(f, "[input] {}", msg),
            Self::Draft(msg) => write!(f, "[draft] {}", msg),
            Self::Validation(msg) => write!(f, "[validation] {}", msg),
            Self::Governance(msg) => write!(f, "[governance] {}", msg),
            Self::Lock(msg) => write!(f, "[lock] {}", msg),
            Self::Orchestration(msg) => write!(f, "[orchestration] {}", msg),
        }
    }
}

impl std::error::Error for CosynError {}

pub type CosynResult<T> = Result<T, CosynError>;
