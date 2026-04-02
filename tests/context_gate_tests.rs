//! Context-aware gate integration tests.
//! Tests that continuation/deictic inputs pass when prior context justifies them,
//! and block when it doesn't.
//!
//! Key design note: "that" is deliberately excluded from REFERENCE_TOKENS in
//! integrity.rs (it functions as a relative pronoun too often), so inputs like
//! "elaborate on that" pass cold-start without needing context. Tests here use
//! "it"-based references which DO trigger cold-start failure, properly exercising
//! the v5.2.0 context fallback.

use cosyn::dcc::subject::bind_subject;
use cosyn::dcc::ambiguity::evaluate_ambiguity;
use cosyn::dcc::evidence::evaluate_evidence;
use cosyn::dcc::types::{PriorMessage, SubjectSource, AmbiguityState, EvidenceScope};

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

// ── Context resolution tests (v5.2.0 feature) ──

#[test]
fn it_reference_resolves_with_context() {
    // "it" is a REFERENCE_TOKEN — fails integrity cold-start, needs context
    let ctx = france_context();
    assert!(run_input_gates_with_context("tell me more about it", Some(&ctx)).is_ok());
}

#[test]
fn it_reference_blocks_without_context() {
    // Without context, "it" has no antecedent — BR-SUBJECT-UNKNOWN
    assert_eq!(
        run_input_gates_with_context("tell me more about it", None),
        Err("BR-SUBJECT-UNKNOWN")
    );
}

#[test]
fn explain_it_further_with_context() {
    let ctx = exercise_context();
    assert!(run_input_gates_with_context("explain it further", Some(&ctx)).is_ok());
}

#[test]
fn vague_nouns_block_even_with_context() {
    // "do the thing with the stuff" — all tokens non-substantive.
    // Evidence gate fires first (before ambiguity). Context cannot rescue
    // all-non-substantive input because it's not a continuation phrase.
    let ctx = france_context();
    assert_eq!(
        run_input_gates_with_context("do the thing with the stuff", Some(&ctx)),
        Err("BR-EVIDENCE-UNSAT")
    );
}

#[test]
fn empty_history_is_cold_start() {
    // Empty vec = no context available. "it" reference has no antecedent.
    let empty: Vec<PriorMessage> = vec![];
    assert_eq!(
        run_input_gates_with_context("tell me more about it", Some(&empty)),
        Err("BR-SUBJECT-UNKNOWN")
    );
}

#[test]
fn system_messages_filtered() {
    // Only system messages — no assistant content for context resolution.
    // "it" reference cannot resolve.
    let msgs = vec![
        PriorMessage { role: "system".into(), content: "You are helpful".into() },
        PriorMessage { role: "system".into(), content: "Be concise".into() },
    ];
    assert_eq!(
        run_input_gates_with_context("tell me more about it", Some(&msgs)),
        Err("BR-SUBJECT-UNKNOWN")
    );
}

#[test]
fn cold_start_clean_input_unaffected() {
    // Standalone factual question — passes with or without context
    assert!(run_input_gates_with_context("What is the capital of France?", None).is_ok());
}

#[test]
fn context_clean_input_unaffected() {
    // Standalone input doesn't change behavior when context is present
    let ctx = vec![
        PriorMessage { role: "user".into(), content: "hi".into() },
        PriorMessage { role: "assistant".into(), content: "hello".into() },
    ];
    assert!(run_input_gates_with_context("What is the capital of France?", Some(&ctx)).is_ok());
}
