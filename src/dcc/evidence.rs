use crate::dcc::types::EvidenceScope;

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

/// Evaluate evidence scope. Evidence is SATISFIED when:
/// - input is non-empty
/// - input contains resolvable content (not just structural markers)
/// - input contains at least one substantive content word
/// Returns UNSATISFIED otherwise. No fallback.
pub fn evaluate_evidence(input: &str) -> EvidenceScope {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return EvidenceScope::Unsatisfied;
    }

    // All-structural content (only markdown markers, no substance)
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

    // Check for at least one substantive content word
    let has_substance = trimmed
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty())
        .any(|t| !NON_SUBSTANTIVE.contains(&t.as_str()));

    if !has_substance {
        return EvidenceScope::Unsatisfied;
    }

    EvidenceScope::Satisfied
}
