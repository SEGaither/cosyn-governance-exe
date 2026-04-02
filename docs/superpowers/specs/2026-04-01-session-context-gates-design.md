# CoSyn v5.2.0 — Context-Aware Gates + Gate/Coaching Integration Tests

**Date:** 2026-04-01
**Status:** Revised (post-review)
**Version target:** 5.2.0
**Principle:** "No rules" — zero additional friction for users

---

## Summary

Three deliverables:

1. **Gate integration tests** — Rust tests proving v5.1.0 input gates work correctly
2. **Coaching integration tests** — Rust tests proving coaching fires on input blocks with a mock LLM provider
3. **Context-aware gates** — subject binding, ambiguity, and evidence gates read prior messages from the existing `messages` array in the OpenAI-compatible request, resolving continuation references before blocking

No new headers, no server-side session state, no new dependencies.

---

## Deliverable 1: Gate Integration Tests

**File:** `tests/gate_tests.rs`

Tests call gate functions directly (no LLM, no proxy, no async). All gate functions receive `None` for prior messages (cold-start behavior).

| Test | Input | Expected Gate Result |
|------|-------|---------------------|
| unresolvable_subject | "asdfghjkl" | BR-SUBJECT-UNKNOWN |
| max_ambiguity | "do the thing with the stuff" | BR-AMBIGUITY |
| all_function_words | "is the a" | BR-EVIDENCE-UNSAT (hits evidence before ambiguity) |
| all_vague_nouns | "something whatever" | BR-AMBIGUITY |
| empty_input | "" | BR-SUBJECT-UNKNOWN |
| clean_factual | "What is the capital of France?" | passes all input gates |
| clean_general | "What are the health benefits of exercise?" | passes all input gates |

Tests assert on the specific `BlockReasonCode` or on successful passage through the input gate phase of the pipeline.

---

## Deliverable 2: Coaching Integration Tests

**File:** `tests/coaching_tests.rs`

**Mock provider:** Implement `LlmProvider` with a struct that returns a canned coaching string. No API key needed.

| Test | Block Code | Expected Behavior |
|------|-----------|-------------------|
| coaching_on_subject_block | BrSubjectUnknown | Coaching returned, contains guidance |
| coaching_on_evidence_block | BrEvidenceUnsat | Coaching returned, contains guidance |
| coaching_on_ambiguity_block | BrAmbiguity | Coaching returned, contains guidance |
| coaching_fallback_on_llm_failure | any | Mock returns error → static `user_message()` returned |
| no_coaching_on_output_gates | BrStructuralFail | No coaching generated (tips list is empty) |

---

## Deliverable 3: Context-Aware Gates

### Principle

The `ChatCompletionRequest.messages` array already contains conversation history in multi-turn clients. The proxy passes `messages[0..n-1]` (everything before the current user message) to input gates as prior context. No new headers, no server state, no new dependencies.

### New Type: PriorMessage

Defined in `dcc::types` to avoid gates depending on proxy types:

```rust
// dcc/types.rs
#[derive(Debug, Clone)]
pub struct PriorMessage {
    pub role: String,
    pub content: String,
}
```

The handler converts `ChatMessage` → `PriorMessage` before passing to the orchestrator. Only messages with `role: "user"` or `role: "assistant"` are included — system messages are filtered out.

### Gate Signature Changes

All three input-side functions gain an optional context parameter:

```rust
// dcc/subject.rs
pub fn bind_subject(input: &str, prior_messages: Option<&[PriorMessage]>) -> SubjectBinding

// dcc/ambiguity.rs
pub fn evaluate_ambiguity(input: &str, prior_messages: Option<&[PriorMessage]>) -> AmbiguityState

// dcc/evidence.rs
pub fn evaluate_evidence(input: &str, prior_messages: Option<&[PriorMessage]>) -> EvidenceScope
```

### Continuation Detection (shared utility)

New file: `dcc/continuation.rs` (add `pub mod continuation;` to `dcc/mod.rs`)

Detects whether an input is a conversational continuation rather than a standalone prompt.

```rust
/// Hardcoded continuation phrases. Matched against the lowercased input,
/// but ONLY when the input has 6 or fewer content words (after removing
/// function words). This prevents false positives on longer standalone
/// queries that happen to contain "expand" or "go on" as part of a
/// different intent.
const CONTINUATION_PHRASES: &[&str] = &[
    "elaborate", "expand", "tell me more", "explain further",
    "go on", "continue", "more detail", "keep going",
    "what about", "how about", "and also",
];

/// Pronouns/deictic references that indicate back-reference.
const DEICTIC_WORDS: &[&str] = &[
    "it", "that", "this", "those", "these", "them",
];

pub fn is_continuation(input: &str) -> bool { ... }
pub fn has_deictic_reference(input: &str) -> bool { ... }
pub fn prior_has_substance(messages: &[PriorMessage]) -> bool { ... }
pub fn last_assistant_content(messages: &[PriorMessage]) -> Option<&str> { ... }
```

**Tokenization strategy:** All functions in this module use the same tokenization as the existing gates: `split_whitespace()` then `trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()`. This ensures consistent behavior across the gate pipeline.

`is_continuation()` — count content words (non-function-words) in input. If more than 6, return false (input is a standalone query, not a continuation). Otherwise, return true if the lowercased input contains any continuation phrase as a substring.

`has_deictic_reference()` — tokenize input using the standard strategy above. Return true if any token exactly matches a deictic word. "it's" tokenizes to "it's" → trimmed to "it" → matches. "itself" → trimmed to "itself" → no match.

`prior_has_substance()` — calls `last_assistant_content(messages)` internally. If None, returns false. Otherwise applies the existing `NON_SUBSTANTIVE` word list (same list used in `evidence.rs`) to that content. Returns true if at least one substantive word exists.

`last_assistant_content()` — iterates `messages` in reverse, returns `Some(&content)` for the first `role: "assistant"` entry. Returns None if no assistant messages exist.

**Known limitation:** Subject re-binding against the last assistant message may extract a different entity than the one the user intended, since assistant messages are longer and may contain multiple entities. This is acceptable as a v1 heuristic — the alternative (NLP-grade coreference resolution) is out of scope.

### Subject Binding — Context Resolution

Current behavior: `bind_subject()` resolves the canonical subject from the input text alone. Inputs like "elaborate on that" produce `SubjectSource::Unknown` because no subject is found.

New behavior when `prior_messages` is `Some`:

1. First, try to bind subject from input alone (existing logic).
2. If result is `Unknown` AND `is_continuation(input)` or `has_deictic_reference(input)`:
   a. Check `last_assistant_content(prior_messages)` for substance.
   b. If the last assistant message has substance, re-run binding against it to extract a carried-forward subject.
   c. If a subject is found, return it with `SubjectSource::Recognized` (not `Crs` or `UserText` — it's inferred from context, which grants cooperative handling but not full binding).
3. If no subject found even with context → `Unknown` (same as today).

When `prior_messages` is `None` → identical to v5.1.0 behavior.

### Ambiguity Gate — Context Resolution

Current behavior: blocks when all content words are vague, or when integrity check fails.

New behavior when `prior_messages` is `Some`:

1. Run existing checks (vague nouns, function words). If input is all vague nouns → **still blocked** even with context. Vague nouns are genuinely empty, not deictic references.
2. If the input contains deictic references (`has_deictic_reference()`) and would otherwise be flagged as ambiguous:
   a. Check `prior_has_substance(prior_messages)`.
   b. If prior context has substance → `Clear` (the reference has something to point at).
   c. If no substance in prior context → `Ambiguous` (same as today).

When `prior_messages` is `None` → identical to v5.1.0 behavior.

### Evidence Gate — Context Expansion

Current behavior: blocks when input has no substantive content words.

New behavior when `prior_messages` is `Some`:

1. Run existing checks on the input alone. If substance exists → `Satisfied` (no change needed).
2. If input has no substance AND `is_continuation(input)`:
   a. Check `prior_has_substance(prior_messages)`.
   b. If prior context has substance → `Satisfied` (the continuation inherits substance from what it's continuing).
   c. If no prior substance → `Unsatisfied` (same as today).

When `prior_messages` is `None` → identical to v5.1.0 behavior.

### Handler Changes

`handlers.rs` — `chat_completions()`:

1. Extract prior messages, filtering to user/assistant only:
   ```rust
   let prior: Option<Vec<PriorMessage>> = if request.messages.len() > 1 {
       let msgs: Vec<PriorMessage> = request.messages[..request.messages.len()-1]
           .iter()
           .filter(|m| m.role == "user" || m.role == "assistant")
           .map(|m| PriorMessage { role: m.role.clone(), content: m.content.clone() })
           .collect();
       if msgs.is_empty() { None } else { Some(msgs) }
   } else {
       None
   };
   ```
2. Pass `prior.as_deref()` to `run_governed()`.

### Orchestrator Changes

`orchestrator::run_governed()` gains `prior_messages: Option<&[PriorMessage]>` parameter. Passes it to `bind_subject()`, `evaluate_ambiguity()`, and `evaluate_evidence()`.

`orchestrator::run()` (sync CLI path) updates all three call sites to pass `None`:
- `bind_subject(input)` → `bind_subject(input, None)`
- `evaluate_evidence(input)` → `evaluate_evidence(input, None)`
- `evaluate_ambiguity(input)` → `evaluate_ambiguity(input, None)`

No regression — identical to v5.1.0 behavior.

### Coaching Note

Coaching fires after the block decision. If the input gets blocked even with context, the coaching message about the opaque phrase is correct — the system couldn't resolve it. No changes to coaching needed.

### What Does NOT Change

- Version gate — stateless
- Grounding gate — stateless, per-turn (constitutional invariant)
- Structural gate — stateless
- Release gate — stateless
- Fail-closed behavior — no context = strict = current behavior
- CLI path — no regression, always passes None

### Context-Aware Gate Tests

**File:** `tests/context_gate_tests.rs`

| Test | Input | Prior Messages | Expected |
|------|-------|---------------|----------|
| continuation_resolves_with_context | "elaborate on that" | [{user: "capital of France?"}, {assistant: "The capital of France is Paris."}] | passes subject + ambiguity + evidence |
| continuation_blocks_without_context | "elaborate on that" | None | blocked (BR-SUBJECT-UNKNOWN) |
| tell_me_more_with_context | "tell me more about it" | [{user: "health benefits of exercise?"}, {assistant: "Exercise improves cardiovascular health..."}] | passes all input gates |
| vague_nouns_block_even_with_context | "do the thing with the stuff" | [{user: "capital of France?"}, {assistant: "The capital of France is Paris."}] | still blocked (BR-AMBIGUITY) — vague nouns are not deictic |
| empty_history_is_cold_start | "elaborate on that" | Some([]) | blocked (empty = no context) |
| system_messages_filtered | "elaborate on that" | [{system: "You are helpful"}, {system: "Be concise"}] | blocked (no user/assistant context after filtering) |
| cold_start_clean_input_unaffected | "What is the capital of France?" | None | passes (same as v5.1.0) |
| context_clean_input_unaffected | "What is the capital of France?" | [{user: "hi"}, {assistant: "hello"}] | passes (standalone input needs no context help) |

---

## Version and Cargo.toml

- Bump version to `5.2.0`
- No new dependencies

---

## Constitutional Invariants Preserved

1. No context = cold start = current behavior (fail-closed)
2. Context resolves references but never weakens evidence requirements — prior substance must exist for continuation to pass
3. Output grounding is per-turn, always — prior messages never reach grounding phase
4. Vague nouns remain blocked regardless of context — they're not references, they're empty
5. Context-inferred subjects get `Recognized` status (cooperative handling), not full binding — context doesn't grant fabrication permission

---

## Files Changed (Summary)

| File | Change |
|------|--------|
| `dcc/types.rs` | Add `PriorMessage` struct |
| `dcc/continuation.rs` | New file — continuation/deictic detection utilities |
| `dcc/mod.rs` | Add `pub mod continuation;` |
| `dcc/subject.rs` | `bind_subject()` gains `Option<&[PriorMessage]>`, context fallback logic |
| `dcc/ambiguity.rs` | `evaluate_ambiguity()` gains `Option<&[PriorMessage]>`, deictic resolution |
| `dcc/evidence.rs` | `evaluate_evidence()` gains `Option<&[PriorMessage]>`, continuation expansion |
| `orchestrator/mod.rs` | `run_governed()` gains parameter, passes to gates. `run()` passes `None`. |
| `proxy/handlers.rs` | Extract + filter prior messages, pass to `run_governed()` |
| `Cargo.toml` | Version bump to 5.2.0 |
| `tests/gate_tests.rs` | New — cold-start gate verification |
| `tests/coaching_tests.rs` | New — mock-provider coaching verification |
| `tests/context_gate_tests.rs` | New — context-aware gate verification |
