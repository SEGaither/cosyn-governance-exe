pub struct IntegritySignal {
    pub proceed: bool,
    pub reason: Option<String>,
    /// True when the input references a recognizable domain that is not
    /// CoSyn-canonical and not inline-defined by the user. The subject is
    /// known but unbound — Model C cooperative handling applies.
    pub recognized_unbound: bool,
}

/// Canonical self-identity for the runtime. Used as a grounding source
/// for the system's own name only. Not a general exception mechanism.
pub const CANONICAL_IDENTITY: &str = "cosyn";

/// Tokens that function as determiners/articles, indicating the following
/// content is an object rather than an entity requiring grounding.
const STRUCTURAL_FOLLOWERS: &[&str] = &[
    "a", "an", "the", "some", "all", "each", "every", "my", "your", "this", "that",
];

/// Pronouns and references that require a resolvable referent.
/// "that" excluded: too ambiguous — functions as relative pronoun ("a letter
/// that is polite") far more often than as a standalone demonstrative reference
/// in cold-start prompts. Including it causes false blocks on legitimate input.
const REFERENCE_TOKENS: &[&str] = &["it", "this"];

fn normalize_tokens(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

fn is_imperative_opening(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }

    let first = &tokens[0];

    // A capitalized first token is an imperative opener when the second token
    // is a structural follower (determiner/article) — this indicates
    // "Verb + article + object" clause structure, not "Entity + ..."
    if first.chars().next().unwrap_or('a').is_uppercase() && first.len() > 1 {
        let second_lower = tokens[1].to_lowercase();
        if STRUCTURAL_FOLLOWERS.contains(&second_lower.as_str()) {
            return true;
        }
    }

    false
}

fn collect_entity_candidates(tokens: &[String], imperative_opening: bool) -> Vec<String> {
    let skip_first = imperative_opening;
    let mut candidates = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        if skip_first && i == 0 {
            continue;
        }

        let clean = token.trim_matches(|c: char| !c.is_alphanumeric());
        if clean.len() <= 2 {
            continue;
        }

        // Must start with uppercase to be a candidate
        if !clean.chars().next().unwrap_or('a').is_uppercase() {
            continue;
        }

        // Skip if token is a structural follower
        if STRUCTURAL_FOLLOWERS.contains(&clean.to_lowercase().as_str()) {
            continue;
        }

        // Skip reference tokens
        if REFERENCE_TOKENS.contains(&clean.to_lowercase().as_str()) {
            continue;
        }

        candidates.push(clean.to_string());
    }

    candidates
}

fn is_entity_grounded(entity: &str, normalized_input: &str) -> bool {
    let e = entity.to_lowercase();

    // Runtime self-identity: the system's own name is always grounded
    if e == CANONICAL_IDENTITY {
        return true;
    }

    normalized_input.contains(&format!("{} is", e))
        || normalized_input.contains(&format!("{} =", e))
        || normalized_input.contains(&format!("{} refers", e))
        || normalized_input.contains(&format!("{} means", e))
}

fn has_resolvable_intent(tokens: &[String], normalized_input: &str) -> bool {
    if tokens.len() < 2 {
        return false;
    }

    // Imperative structure: directive opener + object content
    if is_imperative_opening(tokens) {
        return true;
    }

    // Interrogative structure: contains question-indicating words
    let has_question = normalized_input.contains("what ")
        || normalized_input.contains("how ")
        || normalized_input.contains("why ")
        || normalized_input.contains("when ")
        || normalized_input.contains("where ")
        || normalized_input.ends_with('?');

    if has_question {
        return true;
    }

    // Declarative with relational content: subject + relationship expressed
    let has_relationship = normalized_input.contains(" is ")
        || normalized_input.contains(" does ")
        || normalized_input.contains(" should ")
        || normalized_input.contains(" can ")
        || normalized_input.contains(" has ")
        || normalized_input.contains(" are ");

    if has_relationship && tokens.len() >= 3 {
        return true;
    }

    // Sufficient structural density to carry meaning even without
    // the above patterns (e.g. complex noun-phrase requests)
    tokens.len() >= 2
}

fn has_unresolved_reference(tokens: &[String], known_entities: &[String]) -> bool {
    // If there are grounded entities in the prompt, references have something to point to
    if !known_entities.is_empty() {
        return false;
    }

    // Check for reference tokens regardless of prompt length.
    // Without a grounded entity, "it", "this", and "that" have no antecedent
    // in a cold-start interaction.
    for token in tokens {
        let t = token.to_lowercase();
        if REFERENCE_TOKENS.contains(&t.as_str()) {
            return true;
        }
    }

    false
}

pub fn evaluate_integrity(input: &str) -> IntegritySignal {
    let trimmed = input.trim();

    // 1. Reject empty input
    if trimmed.is_empty() {
        return IntegritySignal {
            proceed: false,
            reason: Some("No input provided.".to_string()),
            recognized_unbound: false,
        };
    }

    // 2. Normalize tokens
    let tokens = normalize_tokens(trimmed);
    let normalized = trimmed.to_lowercase();

    if tokens.is_empty() {
        return IntegritySignal {
            proceed: false,
            reason: Some("No input provided.".to_string()),
            recognized_unbound: false,
        };
    }

    // 3. Determine whether opening clause is imperative
    let imperative = is_imperative_opening(&tokens);

    // 4. Collect entity candidates after excluding imperative opener
    let entities = collect_entity_candidates(&tokens, imperative);

    // 5. Check unresolved references
    if has_unresolved_reference(&tokens, &entities) {
        return IntegritySignal {
            proceed: false,
            reason: Some(
                "Reference detected without a defined subject. Clarify what is being referenced."
                    .to_string(),
            ),
            recognized_unbound: false,
        };
    }

    // 6. Check grounding for entity candidates.
    // Under Model C (Hybrid Controlled Recognition): entities that fail
    // inline grounding but appear in structurally resolvable input are
    // treated as recognized-but-unbound rather than rejected outright.
    // This preserves the cooperative clarification pathway.
    let mut has_ungrounded_entity = false;
    for entity in &entities {
        if !is_entity_grounded(entity, &normalized) {
            has_ungrounded_entity = true;
        }
    }

    // 7. Check whether intent is structurally resolvable
    if !has_resolvable_intent(&tokens, &normalized) {
        return IntegritySignal {
            proceed: false,
            reason: Some("Intent cannot be determined from input.".to_string()),
            recognized_unbound: false,
        };
    }

    // 8. If artifact mode requested, confirm output form is resolvable
    if normalized.contains("paste-ready") && tokens.len() < 3 {
        return IntegritySignal {
            proceed: false,
            reason: Some(
                "Artifact requested but output form is not sufficiently specified.".to_string(),
            ),
            recognized_unbound: false,
        };
    }

    // 9. Proceed — flag recognized_unbound if any entity lacked inline grounding
    // but intent was structurally resolvable. This does NOT grant evidence
    // satisfaction or reasoning permission — downstream gates still apply.
    IntegritySignal {
        proceed: true,
        reason: None,
        recognized_unbound: has_ungrounded_entity,
    }
}
