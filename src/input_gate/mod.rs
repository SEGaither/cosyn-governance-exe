pub mod integrity;

use crate::core::errors::{CosynError, CosynResult};
use crate::core::types::ExecutionRequest;
use crate::dcc::subject::bind_subject;
use crate::input_gate::integrity::evaluate_integrity;

pub fn accept(input: &str) -> CosynResult<ExecutionRequest> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(CosynError::Input("empty input rejected".into()));
    }

    let signal = evaluate_integrity(trimmed);
    if !signal.proceed {
        return Err(CosynError::Input(
            signal
                .reason
                .unwrap_or_else(|| "Integrity check failed".into()),
        ));
    }

    let binding = bind_subject(trimmed);

    Ok(ExecutionRequest {
        id: "req-001".into(),
        input: trimmed.to_string(),
        canonical_subject: binding.canonical_subject,
    })
}
