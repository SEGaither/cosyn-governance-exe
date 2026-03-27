use crate::dcc::types::SubjectSource;
use crate::input_gate::integrity::{evaluate_integrity, CANONICAL_IDENTITY};

pub struct SubjectBinding {
    pub canonical_subject: Option<String>,
    pub source: SubjectSource,
}

/// Resolve subject binding. Returns exactly one of:
/// - CRS entity id (canonical_subject = CANONICAL_IDENTITY, source = Crs)
/// - User entity id (canonical_subject = extracted entity, source = UserText)
/// - UNKNOWN (canonical_subject = None, source = Unknown)
///
/// No fuzzy matching. No fallback completion. Unresolved = UNKNOWN.
pub fn bind_subject(input: &str) -> SubjectBinding {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return SubjectBinding {
            canonical_subject: None,
            source: SubjectSource::Unknown,
        };
    }

    let lower = trimmed.to_lowercase();

    // CRS entity: input references the system's own canonical identity
    if lower.contains(CANONICAL_IDENTITY) {
        return SubjectBinding {
            canonical_subject: Some(CANONICAL_IDENTITY.to_string()),
            source: SubjectSource::Crs,
        };
    }

    // User text entity: integrity check confirms grounding
    let signal = evaluate_integrity(trimmed);
    if signal.proceed {
        if signal.recognized_unbound {
            // Model C: subject is structurally resolvable but not inline-grounded.
            // Classified as Recognized — not Unknown, not fully UserText-bound.
            // Downstream gates (evidence, grounding, governance) still apply.
            return SubjectBinding {
                canonical_subject: Some(trimmed.to_string()),
                source: SubjectSource::Recognized,
            };
        }
        return SubjectBinding {
            canonical_subject: Some(trimmed.to_string()),
            source: SubjectSource::UserText,
        };
    }

    // Unresolved: UNKNOWN — hard block
    SubjectBinding {
        canonical_subject: None,
        source: SubjectSource::Unknown,
    }
}
