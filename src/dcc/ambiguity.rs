use crate::dcc::types::AmbiguityState;
use crate::input_gate::integrity::evaluate_integrity;

/// Words that carry no specific meaning on their own.
const VAGUE_NOUNS: &[&str] = &[
    "thing", "things", "stuff", "something", "whatever", "anything",
    "everything", "somewhere", "somehow", "someone", "anybody",
];

/// Function words (articles, prepositions, pronouns, auxiliaries) that don't
/// contribute substantive content on their own.
const FUNCTION_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "do", "does", "did", "have", "has", "had", "will", "would", "could",
    "should", "can", "may", "might", "shall", "must",
    "i", "me", "my", "you", "your", "he", "she", "it", "we", "they",
    "this", "that", "these", "those",
    "in", "on", "at", "to", "for", "of", "with", "from", "by", "about",
    "into", "through", "during", "before", "after", "above", "below",
    "between", "under", "over",
    "and", "or", "but", "not", "no", "so", "if", "then", "just",
];

/// Evaluate ambiguity state from input.
/// AMBIGUOUS when:
/// - integrity check detects unresolved references or insufficient intent
/// - all content words are vague placeholders
/// - no substantive content words exist
/// CLEAR otherwise.
pub fn evaluate_ambiguity(input: &str) -> AmbiguityState {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return AmbiguityState::Ambiguous;
    }

    // Extract content words (not function words)
    let tokens: Vec<String> = trimmed
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();

    let content_words: Vec<&String> = tokens
        .iter()
        .filter(|t| !FUNCTION_WORDS.contains(&t.as_str()))
        .collect();

    // No content words at all — pure function words
    if content_words.is_empty() {
        return AmbiguityState::Ambiguous;
    }

    // All content words are vague placeholders
    let all_vague = content_words
        .iter()
        .all(|w| VAGUE_NOUNS.contains(&w.as_str()));
    if all_vague {
        return AmbiguityState::Ambiguous;
    }

    // Delegate to integrity check for reference resolution and intent
    let signal = evaluate_integrity(trimmed);
    if signal.proceed {
        AmbiguityState::Clear
    } else {
        AmbiguityState::Ambiguous
    }
}
