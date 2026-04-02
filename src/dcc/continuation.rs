use crate::dcc::types::PriorMessage;

/// Hardcoded continuation phrases. Matched against the lowercased input,
/// but ONLY when the input has 6 or fewer content words (after removing
/// function words). This prevents false positives on longer standalone
/// queries that happen to contain "expand" or "go on".
const CONTINUATION_PHRASES: &[&str] = &[
    "elaborate", "expand", "tell me more", "explain further",
    "go on", "continue", "more detail", "keep going",
    "what about", "how about", "and also",
];

/// Pronouns/deictic references that indicate back-reference.
const DEICTIC_WORDS: &[&str] = &[
    "it", "that", "this", "those", "these", "them",
];

/// Function words excluded when counting content words for continuation detection.
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

/// Non-substantive words — same list as evidence.rs NON_SUBSTANTIVE.
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

/// Tokenize using the same strategy as existing gates: split_whitespace + trim punctuation + lowercase.
fn tokenize(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Count content words (non-function-words) in input.
fn content_word_count(input: &str) -> usize {
    tokenize(input)
        .iter()
        .filter(|t| !FUNCTION_WORDS.contains(&t.as_str()))
        .count()
}

/// Detect whether an input is a conversational continuation.
/// Returns true if input has <6 content words AND contains a continuation phrase.
pub fn is_continuation(input: &str) -> bool {
    if content_word_count(input) >= 6 {
        return false;
    }
    let lower = input.to_lowercase();
    CONTINUATION_PHRASES.iter().any(|phrase| lower.contains(phrase))
}

/// Tokenize splitting on any non-alphanumeric boundary (handles contractions like "it's").
fn tokenize_words(input: &str) -> Vec<String> {
    input
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Detect whether the input contains deictic references (it, that, this, etc.).
pub fn has_deictic_reference(input: &str) -> bool {
    tokenize_words(input)
        .iter()
        .any(|t| DEICTIC_WORDS.contains(&t.as_str()))
}

/// Check whether the last assistant message has at least one substantive word.
pub fn prior_has_substance(messages: &[PriorMessage]) -> bool {
    match last_assistant_content(messages) {
        None => false,
        Some(content) => {
            content
                .split_whitespace()
                .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|t| !t.is_empty())
                .any(|t| !NON_SUBSTANTIVE.contains(&t.as_str()))
        }
    }
}

/// Return the content of the last assistant message, or None.
pub fn last_assistant_content(messages: &[PriorMessage]) -> Option<&str> {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| m.content.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dcc::types::PriorMessage;

    #[test]
    fn test_is_continuation_elaborate() {
        assert!(is_continuation("elaborate on that"));
    }

    #[test]
    fn test_is_continuation_tell_me_more() {
        assert!(is_continuation("tell me more"));
    }

    #[test]
    fn test_is_continuation_long_input_not_continuation() {
        assert!(!is_continuation("I want to expand my business into new international markets"));
    }

    #[test]
    fn test_has_deictic_it() {
        assert!(has_deictic_reference("tell me about it"));
    }

    #[test]
    fn test_has_deictic_that() {
        assert!(has_deictic_reference("explain that"));
    }

    #[test]
    fn test_no_deictic_in_normal_sentence() {
        assert!(!has_deictic_reference("What is the capital of France?"));
    }

    #[test]
    fn test_has_deictic_its_trimmed() {
        assert!(has_deictic_reference("what's it's purpose"));
    }

    #[test]
    fn test_prior_has_substance_with_content() {
        let msgs = vec![
            PriorMessage { role: "user".into(), content: "capital of France?".into() },
            PriorMessage { role: "assistant".into(), content: "The capital of France is Paris.".into() },
        ];
        assert!(prior_has_substance(&msgs));
    }

    #[test]
    fn test_prior_has_substance_empty() {
        let msgs: Vec<PriorMessage> = vec![];
        assert!(!prior_has_substance(&msgs));
    }

    #[test]
    fn test_prior_has_substance_no_assistant() {
        let msgs = vec![
            PriorMessage { role: "user".into(), content: "hello".into() },
        ];
        assert!(!prior_has_substance(&msgs));
    }

    #[test]
    fn test_last_assistant_content() {
        let msgs = vec![
            PriorMessage { role: "user".into(), content: "first".into() },
            PriorMessage { role: "assistant".into(), content: "response one".into() },
            PriorMessage { role: "user".into(), content: "second".into() },
            PriorMessage { role: "assistant".into(), content: "response two".into() },
        ];
        assert_eq!(last_assistant_content(&msgs), Some("response two"));
    }

    #[test]
    fn test_last_assistant_content_none() {
        let msgs = vec![
            PriorMessage { role: "user".into(), content: "hello".into() },
        ];
        assert_eq!(last_assistant_content(&msgs), None);
    }
}
