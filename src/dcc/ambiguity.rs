use crate::dcc::types::AmbiguityState;
use crate::input_gate::integrity::evaluate_integrity;

/// Evaluate ambiguity state from input.
/// AMBIGUOUS when integrity check detects unresolved references or
/// insufficient intent structure. CLEAR otherwise.
pub fn evaluate_ambiguity(input: &str) -> AmbiguityState {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return AmbiguityState::Ambiguous;
    }

    let signal = evaluate_integrity(trimmed);
    if signal.proceed {
        AmbiguityState::Clear
    } else {
        AmbiguityState::Ambiguous
    }
}
