use crate::dcc::types::{EvidenceScope, PriorMessage};
use crate::dcc::continuation;

/// Function words and vague placeholders that don't constitute evidence.
const NON_SUBSTANTIVE: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "do", "does", "did", "have", "has", "had", "will", "would", "could",
    "should", "can", "may", "might", "shall", "must",
    "i", "me", "my", "you", "your", "he", "she", "it", "we", "they",
    "this", "that", "these", "those",
    "in", "on", "at", "to", "for", "of", "with", "from", "by", "about",
    "into", "through", "during", "before", "after", "above", "below",
    "between", "under", "over",
    "and", "or", "but", "not", "no", "so", "if", "then", "just",
    "thing", "things", "stuff", "something", "whatever", "anything",
    "everything", "somewhere", "somehow", "someone", "anybody",
];

/// Evaluate evidence scope with optional conversation context.
///
/// Continuation phrases can inherit substance from prior context.
/// When prior_messages is None, behavior is identical to v5.1.0.
pub fn evaluate_evidence(input: &str, prior_messages: Option<&[PriorMessage]>) -> EvidenceScope {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return EvidenceScope::Unsatisfied;
    }

    let all_structural = trimmed.lines().all(|line| {
        let t = line.trim();
        t.is_empty()
            || t.starts_with('#')
            || t.starts_with("---")
            || t.starts_with("```")
            || t == ">"
            || t == "-"
            || t == "*"
    });

    if all_structural {
        return EvidenceScope::Unsatisfied;
    }

    let has_substance = trimmed
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty())
        .any(|t| !NON_SUBSTANTIVE.contains(&t.as_str()));

    if has_substance {
        return EvidenceScope::Satisfied;
    }

    // ── Context expansion (v5.2.0) ──
    if let Some(msgs) = prior_messages {
        if !msgs.is_empty()
            && continuation::is_continuation(trimmed)
            && continuation::prior_has_substance(msgs)
        {
            return EvidenceScope::Satisfied;
        }
    }

    EvidenceScope::Unsatisfied
}
