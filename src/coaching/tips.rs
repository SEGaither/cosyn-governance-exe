use crate::dcc::types::BlockReasonCode;

pub struct CoachingTip {
    pub name: &'static str,
    pub content: &'static str,
}

const TIP_DEICTIC_ANCHORING: CoachingTip = CoachingTip {
    name: "Deictic Anchoring",
    content: "Ground your prompt with specific context. Instead of vague references like \
        'the thing' or 'it', name what you're talking about. Include who, what, and where. \
        Example: instead of 'fix it', say 'fix the login timeout bug in auth.rs'.",
};

const TIP_MODES_EVIDENCE: CoachingTip = CoachingTip {
    name: "Modes and Evidence",
    content: "Provide evidence or context the model can work with. State facts, paste data, \
        or reference specific documents. The governance system needs substance — not just \
        instructions — to verify the response is grounded.",
};

const TIP_DETERMINISTIC_DATA: CoachingTip = CoachingTip {
    name: "Deterministic Data Handling",
    content: "When working with data or claims, provide the source. Paste the actual numbers, \
        link the document, or quote the passage. The system cannot verify claims it cannot trace \
        back to evidence you supply.",
};

const TIP_CRS_DIRECTIVES: CoachingTip = CoachingTip {
    name: "CRS Directives",
    content: "Be explicit about what you want the model to do. Use clear directives: \
        'summarize', 'compare', 'list', 'explain'. Ambiguous requests like 'do the thing' \
        or 'help with stuff' cannot be resolved into a governed action.",
};

const TIP_FORMATTING_TAX: CoachingTip = CoachingTip {
    name: "Formatting Tax",
    content: "Keep formatting requests minimal. Heavy formatting constraints (tables, \
        specific markdown structures) consume model attention that could go toward content \
        quality. State your format need simply and let the governance layer handle structure.",
};

/// Return the 1-2 most relevant coaching tips for a given block reason.
pub fn tips_for_gate(code: BlockReasonCode) -> Vec<&'static CoachingTip> {
    match code {
        BlockReasonCode::BrSubjectUnknown => {
            vec![&TIP_DEICTIC_ANCHORING, &TIP_MODES_EVIDENCE]
        }
        BlockReasonCode::BrEvidenceUnsat => {
            vec![&TIP_DETERMINISTIC_DATA, &TIP_MODES_EVIDENCE]
        }
        BlockReasonCode::BrAmbiguity => {
            vec![&TIP_DEICTIC_ANCHORING, &TIP_CRS_DIRECTIVES]
        }
        BlockReasonCode::BrStructuralFail | BlockReasonCode::BrGroundingFail => {
            vec![&TIP_FORMATTING_TAX]
        }
        // Version/release gates are system errors — no user coaching applies
        BlockReasonCode::BrVersionConflict
        | BlockReasonCode::BrVersionUndefined
        | BlockReasonCode::BrReleaseDenied => {
            vec![]
        }
    }
}
