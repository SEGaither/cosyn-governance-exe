pub mod bootstrap;
use crate::core::errors::{CosynError, CosynResult};
use crate::core::stage::Stage;
use crate::core::types::LockedOutput;
use crate::dcc::types::{
    BlockReasonCode, DccPipelinePhase, EvidenceScope, RuntimeControl, SubjectSource,
};

/// DCC-enforced pipeline. Supersedes the previous ad-hoc orchestrator.
/// input_gate::accept() is deliberately not called here — its checks
/// (empty input, integrity evaluation) are subsumed by DCC subject binding,
/// evidence evaluation, and ambiguity evaluation. The original accept()
/// remains available for backward-compatible callers.
pub fn run(input: &str) -> CosynResult<LockedOutput> {
    let mut ctrl = RuntimeControl::new();
    crate::telemetry::log_event("input_received", input);

    // ── INGESTED ──
    ctrl.phase = DccPipelinePhase::Ingested;

    // ── BINDING ──
    ctrl.phase = DccPipelinePhase::Binding;
    let binding = crate::dcc::subject::bind_subject(input);
    ctrl.canonical_subject = binding.canonical_subject;
    ctrl.subject_source = binding.source;

    if ctrl.subject_source == SubjectSource::Unknown {
        ctrl.block_reason_code = Some(BlockReasonCode::BrSubjectUnknown);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Input, false, "BR-SUBJECT-UNKNOWN");
        crate::telemetry::log_event("input_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Input(format!(
            "{}: subject could not be resolved",
            BlockReasonCode::BrSubjectUnknown.code()
        )));
    }
    if ctrl.subject_source == SubjectSource::Recognized {
        crate::telemetry::log_stage(
            Stage::Input,
            true,
            "subject recognized (unbound) — cooperative mode",
        );
    } else {
        crate::telemetry::log_stage(Stage::Input, true, "subject bound");
    }

    // ── GATE_CHECK ──
    ctrl.phase = DccPipelinePhase::GateCheck;

    // Evidence
    ctrl.evidence_scope = crate::dcc::evidence::evaluate_evidence(input);
    if ctrl.evidence_scope == EvidenceScope::Unsatisfied {
        ctrl.block_reason_code = Some(BlockReasonCode::BrEvidenceUnsat);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Input, false, "BR-EVIDENCE-UNSAT");
        crate::telemetry::log_event("input_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Input(format!(
            "{}: evidence not satisfied",
            BlockReasonCode::BrEvidenceUnsat.code()
        )));
    }

    // Ambiguity
    ctrl.ambiguity_state = crate::dcc::ambiguity::evaluate_ambiguity(input);
    if ctrl.ambiguity_state == crate::dcc::types::AmbiguityState::Ambiguous {
        ctrl.block_reason_code = Some(BlockReasonCode::BrAmbiguity);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Input, false, "BR-AMBIGUITY");
        crate::telemetry::log_event("input_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Input(format!(
            "{}: ambiguity detected in input",
            BlockReasonCode::BrAmbiguity.code()
        )));
    }

    // Version truth
    let ui_version = crate::ui_runtime::APP_VERSION;
    ctrl.version_truth = crate::dcc::version::evaluate_version_truth(
        crate::dcc::version::RUNTIME_VERSION,
        ui_version,
    );
    if ctrl.version_truth == crate::dcc::types::VersionTruth::Conflict {
        ctrl.block_reason_code = Some(BlockReasonCode::BrVersionConflict);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Validation, false, "BR-VERSION-CONFLICT");
        crate::telemetry::log_event("input_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Orchestration(format!(
            "{}: runtime/UI version mismatch",
            BlockReasonCode::BrVersionConflict.code()
        )));
    }
    if ctrl.version_truth == crate::dcc::types::VersionTruth::Undefined {
        ctrl.block_reason_code = Some(BlockReasonCode::BrVersionUndefined);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Validation, false, "BR-VERSION-UNDEFINED");
        crate::telemetry::log_event("input_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Orchestration(format!(
            "{}: runtime version undefined",
            BlockReasonCode::BrVersionUndefined.code()
        )));
    }

    crate::telemetry::log_stage(Stage::Input, true, "gate checks passed");
    crate::telemetry::log_event("input_validation_result", "allow");

    // ── Pre-reasoning derivation: check if reasoning is permitted ──
    // Before calling LLM, we need structural + semantic on the INPUT side.
    // Structural pass on input: input accepted (already passed binding + evidence + ambiguity)
    ctrl.structural_pass = true;
    // Semantic grounding on input: subject is bound + integrity passed
    ctrl.semantic_grounding_pass = ctrl.subject_source != SubjectSource::Unknown;

    ctrl.derive_reasoning_permitted();
    if !ctrl.reasoning_permitted {
        ctrl.block_reason_code = Some(BlockReasonCode::BrReleaseDenied);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(
            Stage::Draft,
            false,
            "BR-RELEASE-DENIED: reasoning not permitted",
        );
        crate::telemetry::log_event("input_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Orchestration(format!(
            "{}: reasoning not permitted before DCC satisfaction",
            BlockReasonCode::BrReleaseDenied.code()
        )));
    }

    // ── DRAFT + GROUNDING with revision loop ──
    const MAX_ATTEMPTS: usize = 3;
    let mut draft = crate::core::types::DraftOutput { text: String::new() };
    let mut last_failure: Option<CosynError>;

    for attempt in 1..=MAX_ATTEMPTS {
        // Build the prompt: original on first attempt, revision prompt on retries
        let prompt = if attempt == 1 {
            ctrl.canonical_subject.as_deref().unwrap_or(input).to_string()
        } else {
            let verdicts = crate::governance_layer::evaluate_all(input, &draft);
            let failures: Vec<String> = verdicts
                .iter()
                .filter(|v| !v.passed)
                .map(|v| format!("- rule '{}': {}", v.rule, v.detail))
                .collect();
            let failure_list = failures.join("\n");
            format!(
                "Original request: {}\n\nYour previous response was blocked for the following reasons:\n{}\n\nRevise your response to address these issues. Do not include placeholder text, filler, or speculative content.",
                input, failure_list
            )
        };

        crate::telemetry::log_event(
            "llm_call_start",
            &format!("attempt {}/{}", attempt, MAX_ATTEMPTS),
        );

        let draft_result = crate::llm_client::draft(&prompt);
        match draft_result {
            Ok(d) => {
                crate::telemetry::log_event("llm_call_end", "success");
                crate::telemetry::log_stage(
                    Stage::Draft,
                    true,
                    &format!("draft produced (attempt {})", attempt),
                );
                draft = d;
            }
            Err(e) => {
                crate::telemetry::log_event("llm_call_end", "error");
                crate::telemetry::log_event("final_release_decision", "deny");
                return Err(e);
            }
        }

        // ── GROUNDING ──
        ctrl.phase = DccPipelinePhase::Grounding;

        ctrl.structural_pass = crate::dcc::grounding::evaluate_structural(input, &draft);
        if !ctrl.structural_pass {
            crate::telemetry::log_stage(
                Stage::Validation,
                false,
                &format!("BR-STRUCTURAL-FAIL (attempt {})", attempt),
            );
            last_failure = Some(CosynError::Validation(format!(
                "{}: draft failed structural checks",
                BlockReasonCode::BrStructuralFail.code()
            )));
            if attempt < MAX_ATTEMPTS {
                crate::telemetry::log_event("revision", &format!("retrying ({}/{})", attempt + 1, MAX_ATTEMPTS));
                continue;
            }
            ctrl.block_reason_code = Some(BlockReasonCode::BrStructuralFail);
            ctrl.phase = DccPipelinePhase::Blocked;
            crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
            crate::telemetry::log_event("output_validation_result", "deny");
            crate::telemetry::log_event("final_release_decision", "deny");
            return Err(last_failure.unwrap());
        }
        crate::telemetry::log_stage(Stage::Validation, true, "structural_pass = true");

        ctrl.semantic_grounding_pass = crate::dcc::grounding::evaluate_semantic_grounding(
            input,
            &draft,
            ctrl.subject_source,
        );
        if !ctrl.semantic_grounding_pass {
            crate::telemetry::log_stage(
                Stage::Validation,
                false,
                &format!("BR-GROUNDING-FAIL (attempt {})", attempt),
            );
            last_failure = Some(CosynError::Governance(format!(
                "{}: draft failed semantic grounding",
                BlockReasonCode::BrGroundingFail.code()
            )));
            if attempt < MAX_ATTEMPTS {
                crate::telemetry::log_event("revision", &format!("retrying ({}/{})", attempt + 1, MAX_ATTEMPTS));
                continue;
            }
            ctrl.block_reason_code = Some(BlockReasonCode::BrGroundingFail);
            ctrl.phase = DccPipelinePhase::Blocked;
            crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
            crate::telemetry::log_event("output_validation_result", "deny");
            crate::telemetry::log_event("final_release_decision", "deny");
            return Err(last_failure.unwrap());
        }
        crate::telemetry::log_stage(Stage::Validation, true, "semantic_grounding_pass = true");

        // Both checks passed — exit the revision loop
        break;
    }

    // ── READY → FINALIZING ──
    ctrl.phase = DccPipelinePhase::Ready;

    // Final release derivation
    let released = crate::dcc::release::derive_release(&mut ctrl);

    // Block evaluation — catches any remaining gate failure
    if let Some(block_code) = crate::dcc::block::evaluate_block(&ctrl) {
        ctrl.block_reason_code = Some(block_code);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Lock, false, block_code.code());
        crate::telemetry::log_event("output_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Lock(format!(
            "{}: release blocked",
            block_code.code()
        )));
    }

    if !released {
        ctrl.block_reason_code = Some(BlockReasonCode::BrReleaseDenied);
        ctrl.phase = DccPipelinePhase::Blocked;
        crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
        crate::telemetry::log_stage(Stage::Lock, false, "BR-RELEASE-DENIED");
        crate::telemetry::log_event("output_validation_result", "deny");
        crate::telemetry::log_event("final_release_decision", "deny");
        return Err(CosynError::Lock(format!(
            "{}: release_pass = false",
            BlockReasonCode::BrReleaseDenied.code()
        )));
    }

    crate::telemetry::log_event("output_validation_result", "allow");

    // ── FINALIZING → RELEASED ──
    ctrl.phase = DccPipelinePhase::Finalizing;
    crate::telemetry::log_stage(Stage::Lock, true, "artifact locked");

    ctrl.phase = DccPipelinePhase::Released;
    crate::dcc::telemetry::emit_dcc_telemetry(&ctrl);
    crate::telemetry::log_stage(Stage::Output, true, "output released");

    crate::telemetry::log_event("final_release_decision", "allow");
    Ok(LockedOutput {
        text: draft.text,
        locked: true,
        block_reason_code: None,
    })
}

