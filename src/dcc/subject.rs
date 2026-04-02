use crate::dcc::types::{PriorMessage, SubjectSource};
use crate::dcc::continuation;
use crate::input_gate::integrity::{evaluate_integrity, CANONICAL_IDENTITY};

pub struct SubjectBinding {
    pub canonical_subject: Option<String>,
    pub source: SubjectSource,
}

/// Resolve subject binding with optional conversation context.
///
/// When prior_messages is Some, continuation/deictic inputs attempt re-binding
/// against the last assistant message before returning Unknown.
/// When prior_messages is None, behavior is identical to v5.1.0.
pub fn bind_subject(input: &str, prior_messages: Option<&[PriorMessage]>) -> SubjectBinding {
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

    // ── Context fallback (v5.2.0) ──
    if let Some(msgs) = prior_messages {
        if !msgs.is_empty()
            && (continuation::is_continuation(trimmed) || continuation::has_deictic_reference(trimmed))
        {
            if let Some(assistant_content) = continuation::last_assistant_content(msgs) {
                if continuation::prior_has_substance(msgs) {
                    let context_signal = evaluate_integrity(assistant_content);
                    if context_signal.proceed {
                        return SubjectBinding {
                            canonical_subject: Some(assistant_content.to_string()),
                            source: SubjectSource::Recognized,
                        };
                    }
                }
            }
        }
    }

    // Unresolved: UNKNOWN — hard block
    SubjectBinding {
        canonical_subject: None,
        source: SubjectSource::Unknown,
    }
}
