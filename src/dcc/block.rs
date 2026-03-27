use crate::dcc::types::{
    AmbiguityState, BlockReasonCode, EvidenceScope, RuntimeControl, SubjectSource, VersionTruth,
};

/// Evaluate the RuntimeControl and return the first applicable block reason code.
/// Returns the blocking code if any gate fails, or None if all pass.
/// Evaluation order matches DCC priority: subject > evidence > ambiguity >
/// structural > grounding > version > release.
pub fn evaluate_block(ctrl: &RuntimeControl) -> Option<BlockReasonCode> {
    if ctrl.subject_source == SubjectSource::Unknown {
        return Some(BlockReasonCode::BrSubjectUnknown);
    }
    if ctrl.evidence_scope == EvidenceScope::Unsatisfied {
        return Some(BlockReasonCode::BrEvidenceUnsat);
    }
    if ctrl.ambiguity_state == AmbiguityState::Ambiguous {
        return Some(BlockReasonCode::BrAmbiguity);
    }
    if !ctrl.structural_pass {
        return Some(BlockReasonCode::BrStructuralFail);
    }
    if !ctrl.semantic_grounding_pass {
        return Some(BlockReasonCode::BrGroundingFail);
    }
    if ctrl.version_truth == VersionTruth::Conflict {
        return Some(BlockReasonCode::BrVersionConflict);
    }
    if ctrl.version_truth == VersionTruth::Undefined {
        return Some(BlockReasonCode::BrVersionUndefined);
    }
    if !ctrl.release_pass {
        return Some(BlockReasonCode::BrReleaseDenied);
    }
    None
}
