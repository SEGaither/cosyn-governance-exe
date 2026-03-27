//! Tests that the 6 mandatory telemetry events are emitted on every path.

use cosyn::telemetry::take_log;

fn run_and_collect_log(input: &str) -> Vec<String> {
    // Clear prior state
    take_log();
    let _ = cosyn::orchestrator::run(input);
    take_log()
}

#[test]
#[ignore] // requires OPENAI_API_KEY — run with: cargo test -- --ignored
fn happy_path_emits_all_six_events() {
    let log = run_and_collect_log("Summarize the governance policy");
    let joined = log.join("\n");

    assert!(joined.contains("input_received"), "missing input_received\n{}", joined);
    assert!(joined.contains("input_validation_result"), "missing input_validation_result\n{}", joined);
    assert!(joined.contains("llm_call_start"), "missing llm_call_start\n{}", joined);
    assert!(joined.contains("llm_call_end"), "missing llm_call_end\n{}", joined);
    assert!(joined.contains("output_validation_result"), "missing output_validation_result\n{}", joined);
    assert!(joined.contains("final_release_decision"), "missing final_release_decision\n{}", joined);
}

#[test]
fn input_blocked_path_emits_input_events_and_final() {
    let log = run_and_collect_log("");
    let joined = log.join("\n");

    assert!(joined.contains("input_received"), "missing input_received\n{}", joined);
    assert!(joined.contains("input_validation_result"), "missing input_validation_result\n{}", joined);
    assert!(joined.contains("final_release_decision"), "missing final_release_decision\n{}", joined);
    // LLM events must NOT be present when input is blocked
    assert!(!joined.contains("llm_call_start"), "llm_call_start must not appear on input block\n{}", joined);
}
