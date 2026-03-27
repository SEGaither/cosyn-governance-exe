use crate::core::errors::{CosynError, CosynResult};
use crate::core::types::DraftOutput;

const MIN_DRAFT_LENGTH: usize = 20;

const BLOCKED_SENTINELS: &[&str] = &[
    "todo",
    "placeholder",
    "mock failure",
    "not yet implemented",
    "fixme",
    "lorem ipsum",
];

pub fn validate(draft: &DraftOutput) -> CosynResult<()> {
    let trimmed = draft.text.trim();

    if trimmed.is_empty() {
        return Err(CosynError::Validation("draft output is empty".into()));
    }

    if trimmed.len() < MIN_DRAFT_LENGTH {
        return Err(CosynError::Validation(format!(
            "draft too short ({} chars, minimum {})",
            trimmed.len(),
            MIN_DRAFT_LENGTH
        )));
    }

    let lower = trimmed.to_lowercase();
    for sentinel in BLOCKED_SENTINELS {
        if lower.contains(sentinel) {
            return Err(CosynError::Validation(format!(
                "draft contains blocked sentinel: \"{}\"",
                sentinel
            )));
        }
    }

    Ok(())
}
