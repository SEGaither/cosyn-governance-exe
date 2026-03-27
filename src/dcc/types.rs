#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectSource {
    Crs,
    UserText,
    AttachedFile,
    /// Subject is recognizable (structurally resolvable input with entity
    /// references) but not CRS-bound or inline-defined. Model C cooperative
    /// handling applies — recognition does not grant fabrication permission.
    Recognized,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceScope {
    Satisfied,
    Unsatisfied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbiguityState {
    Clear,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionTruth {
    Ok,
    Conflict,
    Undefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncertaintyMode {
    Halt,
    Insufficiency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassBasis {
    Full,
    StructuralOnly,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReasonCode {
    BrSubjectUnknown,
    BrEvidenceUnsat,
    BrAmbiguity,
    BrStructuralFail,
    BrGroundingFail,
    BrVersionConflict,
    BrVersionUndefined,
    BrReleaseDenied,
}

impl BlockReasonCode {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BrSubjectUnknown => "BR-SUBJECT-UNKNOWN",
            Self::BrEvidenceUnsat => "BR-EVIDENCE-UNSAT",
            Self::BrAmbiguity => "BR-AMBIGUITY",
            Self::BrStructuralFail => "BR-STRUCTURAL-FAIL",
            Self::BrGroundingFail => "BR-GROUNDING-FAIL",
            Self::BrVersionConflict => "BR-VERSION-CONFLICT",
            Self::BrVersionUndefined => "BR-VERSION-UNDEFINED",
            Self::BrReleaseDenied => "BR-RELEASE-DENIED",
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self {
            Self::BrSubjectUnknown => "Your input could not be understood. Try adding more detail or rephrasing.",
            Self::BrEvidenceUnsat => "Your input doesn't contain enough substance to process. Add context or a clear question.",
            Self::BrAmbiguity => "Your input is ambiguous. Clarify what you are referring to.",
            Self::BrStructuralFail => "The response failed quality checks. Please try a different prompt.",
            Self::BrGroundingFail => "The response did not meet governance standards. Please try a different prompt.",
            Self::BrVersionConflict => "Internal version mismatch detected. Please reinstall or contact support.",
            Self::BrVersionUndefined => "Internal version error. Please reinstall or contact support.",
            Self::BrReleaseDenied => "The response was blocked by governance review. Try rephrasing your request.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DccPipelinePhase {
    Ingested,
    Binding,
    GateCheck,
    Grounding,
    Ready,
    Finalizing,
    Released,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct RuntimeControl {
    pub canonical_subject: Option<String>,
    pub subject_source: SubjectSource,
    pub evidence_scope: EvidenceScope,
    pub ambiguity_state: AmbiguityState,
    pub structural_pass: bool,
    pub semantic_grounding_pass: bool,
    pub version_truth: VersionTruth,
    pub release_pass: bool,
    pub block_reason_code: Option<BlockReasonCode>,
    pub reasoning_permitted: bool,
    pub uncertainty_mode: UncertaintyMode,
    pub pass_basis: PassBasis,
    pub phase: DccPipelinePhase,
}

impl RuntimeControl {
    /// Create a new RuntimeControl in initial state. All gates default to
    /// blocking — each must be explicitly satisfied.
    pub fn new() -> Self {
        Self {
            canonical_subject: None,
            subject_source: SubjectSource::Unknown,
            evidence_scope: EvidenceScope::Unsatisfied,
            ambiguity_state: AmbiguityState::Ambiguous,
            structural_pass: false,
            semantic_grounding_pass: false,
            version_truth: VersionTruth::Undefined,
            release_pass: false,
            block_reason_code: None,
            reasoning_permitted: false,
            uncertainty_mode: UncertaintyMode::Halt,
            pass_basis: PassBasis::None,
            phase: DccPipelinePhase::Ingested,
        }
    }

    /// Mechanically derive reasoning_permitted from DCC fields.
    /// No other component may override this.
    pub fn derive_reasoning_permitted(&mut self) {
        self.reasoning_permitted = self.subject_source != SubjectSource::Unknown
            && self.evidence_scope == EvidenceScope::Satisfied
            && self.ambiguity_state == AmbiguityState::Clear
            && self.structural_pass
            && self.semantic_grounding_pass
            && self.version_truth == VersionTruth::Ok;
    }

    /// Mechanically derive release_pass from reasoning_permitted.
    /// No other component may override this.
    pub fn derive_release_pass(&mut self) {
        self.derive_reasoning_permitted();
        self.release_pass = self.reasoning_permitted;
    }

    /// Derive pass_basis from current structural/semantic state.
    pub fn derive_pass_basis(&mut self) {
        self.pass_basis = match (self.structural_pass, self.semantic_grounding_pass) {
            (true, true) => PassBasis::Full,
            (true, false) => PassBasis::StructuralOnly,
            _ => PassBasis::None,
        };
    }
}
