use crate::dcc::types::{AmbiguityState, PriorMessage};
use crate::dcc::continuation;
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

/// Evaluate ambiguity state from input with optional conversation context.
///
/// Vague nouns remain Ambiguous regardless of context.
/// Deictic references can resolve to Clear if prior context has substance.
/// When prior_messages is None, behavior is identical to v5.1.0.
pub fn evaluate_ambiguity(input: &str, prior_messages: Option<&[PriorMessage]>) -> AmbiguityState {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return AmbiguityState::Ambiguous;
    }

    let tokens: Vec<String> = trimmed
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();

    let content_words: Vec<&String> = tokens
        .iter()
        .filter(|t| !FUNCTION_WORDS.contains(&t.as_str()))
        .collect();

    if content_words.is_empty() {
        return AmbiguityState::Ambiguous;
    }

    let all_vague = content_words
        .iter()
        .all(|w| VAGUE_NOUNS.contains(&w.as_str()));
    if all_vague {
        return AmbiguityState::Ambiguous;
    }

    let signal = evaluate_integrity(trimmed);
    if signal.proceed {
        return AmbiguityState::Clear;
    }

    // ── Context resolution (v5.2.0) ──
    if let Some(msgs) = prior_messages {
        if !msgs.is_empty()
            && continuation::has_deictic_reference(trimmed)
            && continuation::prior_has_substance(msgs)
        {
            return AmbiguityState::Clear;
        }
    }

    AmbiguityState::Ambiguous
}
