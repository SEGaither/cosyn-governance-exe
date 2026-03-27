use crate::core::types::DraftOutput;
use crate::dcc::types::SubjectSource;

/// Evaluate structural pass. Uses existing validator logic.
/// structural_pass = true means the draft passes all format/length/sentinel checks.
pub fn evaluate_structural(_input: &str, draft: &DraftOutput) -> bool {
    crate::validator::validate(draft).is_ok()
}

/// Evaluate semantic grounding pass.
/// This checks that the draft is constitutionally grounded:
/// - passes all governance rules
/// - subject is resolved (not Unknown)
///
/// If subject is Unknown, semantic grounding always fails.
/// If governance enforcement fails, semantic grounding fails.
pub fn evaluate_semantic_grounding(
    input: &str,
    draft: &DraftOutput,
    subject_source: SubjectSource,
) -> bool {
    if subject_source == SubjectSource::Unknown {
        return false;
    }
    crate::governance_layer::enforce(input, draft).is_ok()
}
