//! DCC enforcement integration tests.
//! These test the RuntimeControl model and its enforcement behavior
//! without requiring an API key (no LLM calls).

use cosyn::core::types::DraftOutput;
use cosyn::dcc::ambiguity::evaluate_ambiguity;
use cosyn::dcc::block::evaluate_block;
use cosyn::dcc::evidence::evaluate_evidence;
use cosyn::dcc::grounding::{evaluate_semantic_grounding, evaluate_structural};
use cosyn::dcc::release::derive_release;
use cosyn::dcc::subject::bind_subject;
use cosyn::dcc::types::*;
use cosyn::dcc::version::evaluate_version_truth;

// ── T-01: Unresolved Subject Blocks ──

#[test]
fn t01_unresolved_subject_blocks() {
    let binding = bind_subject("", None);
    assert_eq!(binding.source, SubjectSource::Unknown);
    assert!(binding.canonical_subject.is_none());

    let mut ctrl = RuntimeControl::new();
    ctrl.subject_source = SubjectSource::Unknown;
    ctrl.canonical_subject = None;

    let block = evaluate_block(&ctrl);
    assert_eq!(block, Some(BlockReasonCode::BrSubjectUnknown));
}

#[test]
fn t01_unknown_subject_prevents_reasoning() {
    let mut ctrl = RuntimeControl::new();
    ctrl.subject_source = SubjectSource::Unknown;
    // Set everything else to passing
    ctrl.evidence_scope = EvidenceScope::Satisfied;
    ctrl.ambiguity_state = AmbiguityState::Clear;
    ctrl.structural_pass = true;
    ctrl.semantic_grounding_pass = true;
    ctrl.version_truth = VersionTruth::Ok;

    ctrl.derive_reasoning_permitted();
    assert!(
        !ctrl.reasoning_permitted,
        "UNKNOWN subject must block reasoning"
    );

    ctrl.derive_release_pass();
    assert!(!ctrl.release_pass, "UNKNOWN subject must block release");
}

// ── T-02: Structural Pass Does Not Release ──

#[test]
fn t02_structural_only_does_not_release() {
    let mut ctrl = RuntimeControl::new();
    ctrl.canonical_subject = Some("test".into());
    ctrl.subject_source = SubjectSource::UserText;
    ctrl.evidence_scope = EvidenceScope::Satisfied;
    ctrl.ambiguity_state = AmbiguityState::Clear;
    ctrl.version_truth = VersionTruth::Ok;
    ctrl.structural_pass = true;
    ctrl.semantic_grounding_pass = false; // structural only

    ctrl.derive_pass_basis();
    assert_eq!(ctrl.pass_basis, PassBasis::StructuralOnly);

    let released = derive_release(&mut ctrl);
    assert!(!released, "structural_pass alone must not permit release");
    assert!(!ctrl.reasoning_permitted);

    let block = evaluate_block(&ctrl);
    assert_eq!(block, Some(BlockReasonCode::BrGroundingFail));
}

#[test]
fn t02_structural_and_semantic_independence() {
    // structural can be true while semantic is false
    let draft = DraftOutput {
        text: "This is a sufficiently long draft output for testing structural pass.".into(),
    };
    let structural = evaluate_structural("some input", &draft);
    let semantic = evaluate_semantic_grounding("some input", &draft, SubjectSource::Unknown);

    assert!(structural, "structural should pass on valid draft");
    assert!(
        !semantic,
        "semantic must fail when subject is UNKNOWN, even if draft is valid"
    );
}

// ── T-03: Version Conflict Blocks ──

#[test]
fn t03_version_conflict_blocks() {
    let truth = evaluate_version_truth("2.2.0", "2.1.4");
    assert_eq!(truth, VersionTruth::Conflict);

    let mut ctrl = RuntimeControl::new();
    ctrl.canonical_subject = Some("test".into());
    ctrl.subject_source = SubjectSource::UserText;
    ctrl.evidence_scope = EvidenceScope::Satisfied;
    ctrl.ambiguity_state = AmbiguityState::Clear;
    ctrl.structural_pass = true;
    ctrl.semantic_grounding_pass = true;
    ctrl.version_truth = VersionTruth::Conflict;

    ctrl.derive_reasoning_permitted();
    assert!(
        !ctrl.reasoning_permitted,
        "version conflict must block reasoning"
    );

    let block = evaluate_block(&ctrl);
    assert_eq!(block, Some(BlockReasonCode::BrVersionConflict));
}

#[test]
fn t03_version_undefined_blocks() {
    let truth = evaluate_version_truth("", "2.2.0");
    assert_eq!(truth, VersionTruth::Undefined);

    let mut ctrl = RuntimeControl::new();
    ctrl.canonical_subject = Some("test".into());
    ctrl.subject_source = SubjectSource::UserText;
    ctrl.evidence_scope = EvidenceScope::Satisfied;
    ctrl.ambiguity_state = AmbiguityState::Clear;
    ctrl.structural_pass = true;
    ctrl.semantic_grounding_pass = true;
    ctrl.version_truth = VersionTruth::Undefined;

    let block = evaluate_block(&ctrl);
    assert_eq!(block, Some(BlockReasonCode::BrVersionUndefined));
}

// ── T-04: Evidence Unsatisfied Blocks ──

#[test]
fn t04_evidence_unsatisfied_blocks() {
    let scope = evaluate_evidence("", None);
    assert_eq!(scope, EvidenceScope::Unsatisfied);

    let scope = evaluate_evidence("# Header\n---\n```\n```", None);
    assert_eq!(scope, EvidenceScope::Unsatisfied);

    let mut ctrl = RuntimeControl::new();
    ctrl.canonical_subject = Some("test".into());
    ctrl.subject_source = SubjectSource::UserText;
    ctrl.evidence_scope = EvidenceScope::Unsatisfied;

    let block = evaluate_block(&ctrl);
    assert_eq!(block, Some(BlockReasonCode::BrEvidenceUnsat));

    ctrl.derive_reasoning_permitted();
    assert!(
        !ctrl.reasoning_permitted,
        "unsatisfied evidence must block reasoning"
    );
}

#[test]
fn t04_ambiguity_blocks() {
    let amb = evaluate_ambiguity("", None);
    assert_eq!(amb, AmbiguityState::Ambiguous);

    let mut ctrl = RuntimeControl::new();
    ctrl.canonical_subject = Some("test".into());
    ctrl.subject_source = SubjectSource::UserText;
    ctrl.evidence_scope = EvidenceScope::Satisfied;
    ctrl.ambiguity_state = AmbiguityState::Ambiguous;

    let block = evaluate_block(&ctrl);
    assert_eq!(block, Some(BlockReasonCode::BrAmbiguity));
}

// ── T-05: Happy Path Releases ──

#[test]
fn t05_happy_path_releases() {
    let mut ctrl = RuntimeControl::new();
    ctrl.canonical_subject = Some("cosyn".into());
    ctrl.subject_source = SubjectSource::Crs;
    ctrl.evidence_scope = EvidenceScope::Satisfied;
    ctrl.ambiguity_state = AmbiguityState::Clear;
    ctrl.structural_pass = true;
    ctrl.semantic_grounding_pass = true;
    ctrl.version_truth = VersionTruth::Ok;

    let released = derive_release(&mut ctrl);
    assert!(released, "all DCC fields satisfied must release");
    assert!(ctrl.reasoning_permitted);
    assert_eq!(ctrl.pass_basis, PassBasis::Full);

    let block = evaluate_block(&ctrl);
    assert!(block.is_none(), "no block code when all pass");
}

// ── Block reason codes always present ──

#[test]
fn block_codes_are_machine_emitted() {
    let codes = [
        BlockReasonCode::BrSubjectUnknown,
        BlockReasonCode::BrEvidenceUnsat,
        BlockReasonCode::BrAmbiguity,
        BlockReasonCode::BrStructuralFail,
        BlockReasonCode::BrGroundingFail,
        BlockReasonCode::BrVersionConflict,
        BlockReasonCode::BrVersionUndefined,
        BlockReasonCode::BrReleaseDenied,
    ];
    for code in &codes {
        assert!(
            code.code().starts_with("BR-"),
            "code {:?} must start with BR-",
            code
        );
    }
}

// ── Derivation cannot be overridden ──

#[test]
fn reasoning_permitted_is_mechanically_derived() {
    let mut ctrl = RuntimeControl::new();
    // Manually set reasoning_permitted — derivation must overwrite
    ctrl.reasoning_permitted = true;
    ctrl.subject_source = SubjectSource::Unknown;
    ctrl.derive_reasoning_permitted();
    assert!(
        !ctrl.reasoning_permitted,
        "manual override must not survive derivation"
    );
}

#[test]
fn release_pass_is_mechanically_derived() {
    let mut ctrl = RuntimeControl::new();
    ctrl.release_pass = true;
    ctrl.subject_source = SubjectSource::Unknown;
    ctrl.derive_release_pass();
    assert!(
        !ctrl.release_pass,
        "manual override must not survive derivation"
    );
}
