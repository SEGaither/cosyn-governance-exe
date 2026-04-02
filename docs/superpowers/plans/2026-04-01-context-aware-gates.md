# Context-Aware Gates (v5.2.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add conversation-history awareness to input gates (subject, ambiguity, evidence) so continuation prompts like "elaborate on that" pass when prior context justifies them, while preserving fail-closed cold-start behavior.

**Architecture:** Prior messages are extracted from the existing OpenAI-compatible `messages` array in the handler, converted to a `PriorMessage` type owned by the DCC layer, and passed through the orchestrator to each input gate as `Option<&[PriorMessage]>`. A shared `continuation.rs` module provides deictic/continuation detection utilities. No new dependencies, no server-side session state.

**Tech Stack:** Rust, axum, tokio, async-trait. Existing `LlmProvider` trait for mock provider in coaching tests.

**Spec:** `docs/superpowers/specs/2026-04-01-session-context-gates-design.md`

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src/dcc/types.rs` | Add `PriorMessage` struct | Modify |
| `src/dcc/continuation.rs` | Continuation/deictic detection utilities | Create |
| `src/dcc/mod.rs` | Register `continuation` module | Modify |
| `src/dcc/subject.rs` | Add context fallback to `bind_subject()` | Modify |
| `src/dcc/ambiguity.rs` | Add deictic resolution to `evaluate_ambiguity()` | Modify |
| `src/dcc/evidence.rs` | Add continuation expansion to `evaluate_evidence()` | Modify |
| `src/orchestrator/mod.rs` | Thread `Option<&[PriorMessage]>` through both pipelines | Modify |
| `src/proxy/handlers.rs` | Extract prior messages from request | Modify |
| `Cargo.toml` | Version bump to 5.2.0 | Modify |
| `tests/gate_tests.rs` | Cold-start gate integration tests (7 cases) | Create |
| `tests/coaching_tests.rs` | Mock-provider coaching tests (5 cases) | Create |
| `tests/context_gate_tests.rs` | Context-aware gate tests (8 cases) | Create |

---

### Task 1: Add PriorMessage type

**Files:**
- Modify: `src/dcc/types.rs`

- [ ] **Step 1: Add PriorMessage struct to types.rs**

Add after the existing `SubjectSource` enum (before `EvidenceScope`):

```rust
#[derive(Debug, Clone)]
pub struct PriorMessage {
    pub role: String,
    pub content: String,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: compiles (new type is unused — that's fine, it gets used in Task 2)

- [ ] **Step 3: Commit**

```bash
git add src/dcc/types.rs
git commit -m "feat(dcc): add PriorMessage type for context-aware gates"
```

---

### Task 2: Create continuation.rs module

**Files:**
- Create: `src/dcc/continuation.rs`
- Modify: `src/dcc/mod.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of the new `src/dcc/continuation.rs` file:

```rust
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
        // More than 6 content words — standalone query, not a continuation
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
        // "it's" trims to "it" which matches
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
```

- [ ] **Step 2: Write the implementation above the tests**

```rust
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
/// Same list as ambiguity.rs FUNCTION_WORDS.
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
/// Returns true if input has ≤6 content words AND contains a continuation phrase.
pub fn is_continuation(input: &str) -> bool {
    if content_word_count(input) > 6 {
        return false;
    }
    let lower = input.to_lowercase();
    CONTINUATION_PHRASES.iter().any(|phrase| lower.contains(phrase))
}

/// Detect whether the input contains deictic references (it, that, this, etc.).
pub fn has_deictic_reference(input: &str) -> bool {
    tokenize(input)
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
```

- [ ] **Step 3: Register module in dcc/mod.rs**

Add `pub mod continuation;` after the existing `pub mod telemetry;` line.

- [ ] **Step 4: Run tests**

Run: `cargo test continuation -- --nocapture`
Expected: all 13 continuation tests pass

- [ ] **Step 5: Commit**

```bash
git add src/dcc/continuation.rs src/dcc/mod.rs
git commit -m "feat(dcc): add continuation/deictic detection module"
```

---

### Task 3: Gate integration tests (cold-start, Deliverable 1)

**Files:**
- Create: `tests/gate_tests.rs`

- [ ] **Step 1: Write the test file**

```rust
//! Cold-start gate integration tests.
//! All gates receive None for prior messages (v5.1.0 behavior).

use cosyn::dcc::subject::bind_subject;
use cosyn::dcc::ambiguity::evaluate_ambiguity;
use cosyn::dcc::evidence::evaluate_evidence;
use cosyn::dcc::types::{SubjectSource, AmbiguityState, EvidenceScope};

/// Helper: run all three input gates with no context. Returns the first block reason or Ok.
fn run_input_gates(input: &str) -> Result<(), &'static str> {
    let binding = bind_subject(input, None);
    if binding.source == SubjectSource::Unknown {
        return Err("BR-SUBJECT-UNKNOWN");
    }

    let evidence = evaluate_evidence(input, None);
    if evidence == EvidenceScope::Unsatisfied {
        return Err("BR-EVIDENCE-UNSAT");
    }

    let ambiguity = evaluate_ambiguity(input, None);
    if ambiguity == AmbiguityState::Ambiguous {
        return Err("BR-AMBIGUITY");
    }

    Ok(())
}

#[test]
fn unresolvable_subject() {
    assert_eq!(run_input_gates("asdfghjkl"), Err("BR-SUBJECT-UNKNOWN"));
}

#[test]
fn max_ambiguity() {
    assert_eq!(run_input_gates("do the thing with the stuff"), Err("BR-AMBIGUITY"));
}

#[test]
fn all_function_words() {
    // "is the a" — no substantive words → hits evidence before ambiguity
    assert_eq!(run_input_gates("is the a"), Err("BR-EVIDENCE-UNSAT"));
}

#[test]
fn all_vague_nouns() {
    assert_eq!(run_input_gates("something whatever"), Err("BR-AMBIGUITY"));
}

#[test]
fn empty_input() {
    assert_eq!(run_input_gates(""), Err("BR-SUBJECT-UNKNOWN"));
}

#[test]
fn clean_factual() {
    assert!(run_input_gates("What is the capital of France?").is_ok());
}

#[test]
fn clean_general() {
    assert!(run_input_gates("What are the health benefits of exercise?").is_ok());
}
```

**IMPORTANT:** This test file calls `bind_subject(input, None)`, `evaluate_evidence(input, None)`, and `evaluate_ambiguity(input, None)` — these new signatures don't exist yet. This file will NOT compile until Tasks 4-6 are done. Create it now but don't expect `cargo test` to pass until after Task 6.

- [ ] **Step 2: Commit (tests written, not yet compiling)**

```bash
git add tests/gate_tests.rs
git commit -m "test: add cold-start gate integration tests (pending signature changes)"
```

---

### Task 4: Context-aware subject binding

**Files:**
- Modify: `src/dcc/subject.rs`

- [ ] **Step 1: Update bind_subject signature and add context fallback**

Replace the entire `src/dcc/subject.rs` with:

```rust
use crate::dcc::types::{PriorMessage, SubjectSource};
use crate::dcc::continuation;
use crate::input_gate::integrity::{evaluate_integrity, CANONICAL_IDENTITY};

pub struct SubjectBinding {
    pub canonical_subject: Option<String>,
    pub source: SubjectSource,
}

/// Resolve subject binding. Returns exactly one of:
/// - CRS entity id (canonical_subject = CANONICAL_IDENTITY, source = Crs)
/// - User entity id (canonical_subject = extracted entity, source = UserText)
/// - Context-inferred entity (source = Recognized) — cooperative handling
/// - UNKNOWN (canonical_subject = None, source = Unknown)
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
    // If input failed binding AND is a continuation/deictic reference,
    // try re-binding against the last assistant message.
    if let Some(msgs) = prior_messages {
        if !msgs.is_empty()
            && (continuation::is_continuation(trimmed) || continuation::has_deictic_reference(trimmed))
        {
            if let Some(assistant_content) = continuation::last_assistant_content(msgs) {
                if continuation::prior_has_substance(msgs) {
                    // Re-run integrity against the assistant content to extract a subject
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
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Errors in orchestrator/mod.rs because `bind_subject` now requires 2 args. That's expected — Task 7 fixes those call sites.

- [ ] **Step 3: Commit**

```bash
git add src/dcc/subject.rs
git commit -m "feat(dcc): add context fallback to bind_subject"
```

---

### Task 5: Context-aware ambiguity gate

**Files:**
- Modify: `src/dcc/ambiguity.rs`

- [ ] **Step 1: Update evaluate_ambiguity with context parameter**

Replace the entire `src/dcc/ambiguity.rs` with:

```rust
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

/// Evaluate ambiguity state from input.
///
/// When prior_messages is Some and the input contains deictic references,
/// prior context with substance can resolve the ambiguity to Clear.
/// Vague nouns remain Ambiguous regardless of context — they are genuinely
/// empty, not deictic references.
///
/// When prior_messages is None, behavior is identical to v5.1.0.
pub fn evaluate_ambiguity(input: &str, prior_messages: Option<&[PriorMessage]>) -> AmbiguityState {
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

    // All content words are vague placeholders — blocked even with context
    let all_vague = content_words
        .iter()
        .all(|w| VAGUE_NOUNS.contains(&w.as_str()));
    if all_vague {
        return AmbiguityState::Ambiguous;
    }

    // Delegate to integrity check for reference resolution and intent
    let signal = evaluate_integrity(trimmed);
    if signal.proceed {
        return AmbiguityState::Clear;
    }

    // ── Context resolution (v5.2.0) ──
    // If input has deictic references and would otherwise be ambiguous,
    // check if prior context has substance to resolve the reference.
    if let Some(msgs) = prior_messages {
        if !msgs.is_empty() && continuation::has_deictic_reference(trimmed) {
            if continuation::prior_has_substance(msgs) {
                return AmbiguityState::Clear;
            }
        }
    }

    AmbiguityState::Ambiguous
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Still errors from orchestrator call sites — expected, fixed in Task 7.

- [ ] **Step 3: Commit**

```bash
git add src/dcc/ambiguity.rs
git commit -m "feat(dcc): add context resolution to evaluate_ambiguity"
```

---

### Task 6: Context-aware evidence gate

**Files:**
- Modify: `src/dcc/evidence.rs`

- [ ] **Step 1: Update evaluate_evidence with context parameter**

Replace the entire `src/dcc/evidence.rs` with:

```rust
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

/// Evaluate evidence scope.
///
/// When prior_messages is Some and the input is a continuation phrase,
/// prior context with substance satisfies the evidence requirement.
///
/// When prior_messages is None, behavior is identical to v5.1.0.
pub fn evaluate_evidence(input: &str, prior_messages: Option<&[PriorMessage]>) -> EvidenceScope {
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

    if has_substance {
        return EvidenceScope::Satisfied;
    }

    // ── Context expansion (v5.2.0) ──
    // If input has no substance AND is a continuation phrase,
    // check if prior context has substance to inherit from.
    if let Some(msgs) = prior_messages {
        if !msgs.is_empty() && continuation::is_continuation(trimmed) {
            if continuation::prior_has_substance(msgs) {
                return EvidenceScope::Satisfied;
            }
        }
    }

    EvidenceScope::Unsatisfied
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Still errors from orchestrator — that's next.

- [ ] **Step 3: Commit**

```bash
git add src/dcc/evidence.rs
git commit -m "feat(dcc): add continuation expansion to evaluate_evidence"
```

---

### Task 7: Update orchestrator to thread prior messages

**Files:**
- Modify: `src/orchestrator/mod.rs`

- [ ] **Step 1: Update run() — all gate calls pass None**

In `run()`, change these three lines:

```rust
// OLD:
let binding = crate::dcc::subject::bind_subject(input);
// NEW:
let binding = crate::dcc::subject::bind_subject(input, None);
```

```rust
// OLD:
ctrl.evidence_scope = crate::dcc::evidence::evaluate_evidence(input);
// NEW:
ctrl.evidence_scope = crate::dcc::evidence::evaluate_evidence(input, None);
```

```rust
// OLD:
ctrl.ambiguity_state = crate::dcc::ambiguity::evaluate_ambiguity(input);
// NEW:
ctrl.ambiguity_state = crate::dcc::ambiguity::evaluate_ambiguity(input, None);
```

- [ ] **Step 2: Update run_governed() — add parameter and pass to gates**

Change the signature:

```rust
// OLD:
pub async fn run_governed(
    input: &str,
    provider: &dyn crate::provider::LlmProvider,
) -> CosynResult<LockedOutput> {
// NEW:
pub async fn run_governed(
    input: &str,
    provider: &dyn crate::provider::LlmProvider,
    prior_messages: Option<&[crate::dcc::types::PriorMessage]>,
) -> CosynResult<LockedOutput> {
```

Then update the three gate calls inside `run_governed()`:

```rust
// OLD:
let binding = crate::dcc::subject::bind_subject(input);
// NEW:
let binding = crate::dcc::subject::bind_subject(input, prior_messages);
```

```rust
// OLD:
ctrl.evidence_scope = crate::dcc::evidence::evaluate_evidence(input);
// NEW:
ctrl.evidence_scope = crate::dcc::evidence::evaluate_evidence(input, prior_messages);
```

```rust
// OLD:
ctrl.ambiguity_state = crate::dcc::ambiguity::evaluate_ambiguity(input);
// NEW:
ctrl.ambiguity_state = crate::dcc::ambiguity::evaluate_ambiguity(input, prior_messages);
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Error in `handlers.rs` — `run_governed` now requires 3 args. Fixed in Task 8.

- [ ] **Step 4: Commit**

```bash
git add src/orchestrator/mod.rs
git commit -m "feat(orchestrator): thread prior_messages through gate calls"
```

---

### Task 8: Update handler to extract prior messages

**Files:**
- Modify: `src/proxy/handlers.rs`

- [ ] **Step 1: Add prior message extraction and pass to run_governed**

In `chat_completions()`, after the `user_message` extraction and before the `run_governed` call, add:

```rust
    // Extract prior messages (everything before the last message), filtering to user/assistant only
    let prior: Option<Vec<crate::dcc::types::PriorMessage>> = if request.messages.len() > 1 {
        let msgs: Vec<crate::dcc::types::PriorMessage> = request.messages[..request.messages.len()-1]
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
            .map(|m| crate::dcc::types::PriorMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        if msgs.is_empty() { None } else { Some(msgs) }
    } else {
        None
    };
```

Then update the `run_governed` call:

```rust
// OLD:
match crate::orchestrator::run_governed(&user_message, state.provider.as_ref()).await {
// NEW:
match crate::orchestrator::run_governed(&user_message, state.provider.as_ref(), prior.as_deref()).await {
```

- [ ] **Step 2: Verify full compilation**

Run: `cargo check`
Expected: Clean compilation — all call sites updated.

- [ ] **Step 3: Run existing tests**

Run: `cargo test`
Expected: All existing tests pass (no regressions — CLI path always passes None).

- [ ] **Step 4: Commit**

```bash
git add src/proxy/handlers.rs
git commit -m "feat(proxy): extract prior messages and pass to governed pipeline"
```

---

### Task 9: Coaching integration tests (Deliverable 2)

**Files:**
- Create: `tests/coaching_tests.rs`

- [ ] **Step 1: Write the test file**

```rust
//! Coaching integration tests with a mock LLM provider.

use cosyn::coaching::generate_coaching;
use cosyn::core::errors::{CosynError, CosynResult};
use cosyn::dcc::types::BlockReasonCode;
use cosyn::provider::{LlmProvider, LlmRequest, LlmResponse};

/// Mock provider that returns a canned coaching response.
struct MockCoachingProvider;

#[async_trait::async_trait]
impl LlmProvider for MockCoachingProvider {
    fn name(&self) -> &str {
        "mock-coaching"
    }

    async fn complete(&self, _request: &LlmRequest) -> CosynResult<LlmResponse> {
        Ok(LlmResponse {
            content: "Try being more specific about what you want to discuss. For example, instead of 'do the thing', say 'explain the authentication flow in auth.rs'.".into(),
            model: "mock".into(),
            input_tokens: 0,
            output_tokens: 0,
        })
    }
}

/// Mock provider that always fails.
struct FailingProvider;

#[async_trait::async_trait]
impl LlmProvider for FailingProvider {
    fn name(&self) -> &str {
        "mock-failing"
    }

    async fn complete(&self, _request: &LlmRequest) -> CosynResult<LlmResponse> {
        Err(CosynError::Provider("mock failure".into()))
    }
}

#[tokio::test]
async fn coaching_on_subject_block() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("asdfghjkl", BlockReasonCode::BrSubjectUnknown, &provider).await;
    assert!(!result.is_empty());
    assert!(result.contains("specific"));
}

#[tokio::test]
async fn coaching_on_evidence_block() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("is the a", BlockReasonCode::BrEvidenceUnsat, &provider).await;
    assert!(!result.is_empty());
}

#[tokio::test]
async fn coaching_on_ambiguity_block() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("do the thing with the stuff", BlockReasonCode::BrAmbiguity, &provider).await;
    assert!(!result.is_empty());
}

#[tokio::test]
async fn coaching_fallback_on_llm_failure() {
    let provider = FailingProvider;
    let result = generate_coaching("asdfghjkl", BlockReasonCode::BrSubjectUnknown, &provider).await;
    // Should fall back to static user_message()
    assert_eq!(result, BlockReasonCode::BrSubjectUnknown.user_message());
}

#[tokio::test]
async fn no_coaching_on_output_gates() {
    let provider = MockCoachingProvider;
    let result = generate_coaching("anything", BlockReasonCode::BrStructuralFail, &provider).await;
    // BrStructuralFail has tips (TIP_FORMATTING_TAX), so it WILL call the LLM.
    // But BrVersionConflict/BrVersionUndefined/BrReleaseDenied have empty tips → static message.
    // Use BrReleaseDenied instead for "no coaching" test:
    let result2 = generate_coaching("anything", BlockReasonCode::BrReleaseDenied, &provider).await;
    assert_eq!(result2, BlockReasonCode::BrReleaseDenied.user_message());
}
```

**NOTE:** The spec says `BrStructuralFail` should have "no coaching generated", but examining `tips.rs`, `BrStructuralFail` maps to `TIP_FORMATTING_TAX` (non-empty). The test above adjusts: it uses `BrReleaseDenied` (which truly has empty tips) for the "no coaching" assertion. The `BrStructuralFail` case still works — it just calls the LLM. Flag this spec discrepancy to the user if asked.

- [ ] **Step 2: Verify compilation and run**

Run: `cargo test coaching -- --nocapture`
Expected: All 5 coaching tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/coaching_tests.rs
git commit -m "test: add coaching integration tests with mock provider"
```

---

### Task 10: Context-aware gate integration tests (Deliverable 3)

**Files:**
- Create: `tests/context_gate_tests.rs`

- [ ] **Step 1: Write the test file**

```rust
//! Context-aware gate integration tests.
//! Tests that continuation/deictic inputs pass when prior context justifies them,
//! and block when it doesn't.

use cosyn::dcc::subject::bind_subject;
use cosyn::dcc::ambiguity::evaluate_ambiguity;
use cosyn::dcc::evidence::evaluate_evidence;
use cosyn::dcc::types::{PriorMessage, SubjectSource, AmbiguityState, EvidenceScope};

/// Helper: run all three input gates with optional context. Returns first block reason or Ok.
fn run_input_gates_with_context(
    input: &str,
    prior: Option<&[PriorMessage]>,
) -> Result<(), &'static str> {
    let binding = bind_subject(input, prior);
    if binding.source == SubjectSource::Unknown {
        return Err("BR-SUBJECT-UNKNOWN");
    }

    let evidence = evaluate_evidence(input, prior);
    if evidence == EvidenceScope::Unsatisfied {
        return Err("BR-EVIDENCE-UNSAT");
    }

    let ambiguity = evaluate_ambiguity(input, prior);
    if ambiguity == AmbiguityState::Ambiguous {
        return Err("BR-AMBIGUITY");
    }

    Ok(())
}

fn france_context() -> Vec<PriorMessage> {
    vec![
        PriorMessage { role: "user".into(), content: "What is the capital of France?".into() },
        PriorMessage { role: "assistant".into(), content: "The capital of France is Paris.".into() },
    ]
}

fn exercise_context() -> Vec<PriorMessage> {
    vec![
        PriorMessage { role: "user".into(), content: "What are the health benefits of exercise?".into() },
        PriorMessage { role: "assistant".into(), content: "Exercise improves cardiovascular health, strengthens muscles, and boosts mental well-being.".into() },
    ]
}

#[test]
fn continuation_resolves_with_context() {
    let ctx = france_context();
    assert!(run_input_gates_with_context("elaborate on that", Some(&ctx)).is_ok());
}

#[test]
fn continuation_blocks_without_context() {
    assert_eq!(
        run_input_gates_with_context("elaborate on that", None),
        Err("BR-SUBJECT-UNKNOWN")
    );
}

#[test]
fn tell_me_more_with_context() {
    let ctx = exercise_context();
    assert!(run_input_gates_with_context("tell me more about it", Some(&ctx)).is_ok());
}

#[test]
fn vague_nouns_block_even_with_context() {
    let ctx = france_context();
    assert_eq!(
        run_input_gates_with_context("do the thing with the stuff", Some(&ctx)),
        Err("BR-AMBIGUITY")
    );
}

#[test]
fn empty_history_is_cold_start() {
    let empty: Vec<PriorMessage> = vec![];
    assert_eq!(
        run_input_gates_with_context("elaborate on that", Some(&empty)),
        Err("BR-SUBJECT-UNKNOWN")
    );
}

#[test]
fn system_messages_filtered() {
    // Only system messages — no user/assistant context after filtering
    // The handler filters these, but if they somehow reach gates, no substance exists
    let msgs = vec![
        PriorMessage { role: "system".into(), content: "You are helpful".into() },
        PriorMessage { role: "system".into(), content: "Be concise".into() },
    ];
    assert_eq!(
        run_input_gates_with_context("elaborate on that", Some(&msgs)),
        Err("BR-SUBJECT-UNKNOWN")
    );
}

#[test]
fn cold_start_clean_input_unaffected() {
    assert!(run_input_gates_with_context("What is the capital of France?", None).is_ok());
}

#[test]
fn context_clean_input_unaffected() {
    let ctx = vec![
        PriorMessage { role: "user".into(), content: "hi".into() },
        PriorMessage { role: "assistant".into(), content: "hello".into() },
    ];
    assert!(run_input_gates_with_context("What is the capital of France?", Some(&ctx)).is_ok());
}
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass — gate_tests (7), coaching_tests (5), context_gate_tests (8), plus all existing tests.

- [ ] **Step 3: Commit**

```bash
git add tests/context_gate_tests.rs
git commit -m "test: add context-aware gate integration tests"
```

---

### Task 11: Version bump and final verification

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Bump version**

In `Cargo.toml`, change:
```toml
version = "5.1.0"
```
to:
```toml
version = "5.2.0"
```

- [ ] **Step 2: Run full test suite**

Run: `cargo test`
Expected: All tests pass. Record the count.

- [ ] **Step 3: Run cargo clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore: bump version to 5.2.0"
```

- [ ] **Step 5: Final verification — run full suite one more time**

Run: `cargo test 2>&1 | tail -5`
Expected: `test result: ok. N passed; 0 failed; ...`
