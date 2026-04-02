//! Cold-start gate integration tests.

use cosyn::dcc::subject::bind_subject;
use cosyn::dcc::ambiguity::evaluate_ambiguity;
use cosyn::dcc::evidence::evaluate_evidence;
use cosyn::dcc::types::{SubjectSource, AmbiguityState, EvidenceScope};

/// Run all three input gates with no context. Returns first block reason or Ok.
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
    // "do the thing with the stuff" — subject passes (imperative structure).
    // All tokens are in NON_SUBSTANTIVE, so evidence gate blocks before ambiguity.
    assert_eq!(run_input_gates("do the thing with the stuff"), Err("BR-EVIDENCE-UNSAT"));
}

#[test]
fn all_function_words() {
    assert_eq!(run_input_gates("is the a"), Err("BR-EVIDENCE-UNSAT"));
}

#[test]
fn all_vague_nouns() {
    // "something whatever" — subject gate passes (2-token structurally resolvable).
    // Evidence gate blocks first: all tokens are in NON_SUBSTANTIVE, so BR-EVIDENCE-UNSAT.
    assert_eq!(run_input_gates("something whatever"), Err("BR-EVIDENCE-UNSAT"));
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
