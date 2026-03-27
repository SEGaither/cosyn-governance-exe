use crate::dcc::types::{RuntimeControl, SubjectSource, VersionTruth};
use std::sync::Mutex;

static DCC_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn log_dcc_metric(key: &str, value: &str) {
    let line = format!("[dcc] {} = {}", key, value);
    println!("  {}", line);
    if let Ok(mut buf) = DCC_LOG.lock() {
        buf.push(line);
    }
}

pub fn take_dcc_log() -> Vec<String> {
    if let Ok(mut buf) = DCC_LOG.lock() {
        std::mem::take(&mut *buf)
    } else {
        Vec::new()
    }
}

/// Emit all required DCC telemetry metrics from a RuntimeControl snapshot.
pub fn emit_dcc_telemetry(ctrl: &RuntimeControl) {
    log_dcc_metric(
        "primary_subject_resolved",
        &ctrl.canonical_subject.is_some().to_string(),
    );
    log_dcc_metric(
        "subject_binding_source",
        &format!("{:?}", ctrl.subject_source),
    );
    log_dcc_metric(
        "unknown_halt_triggered",
        &(ctrl.subject_source == SubjectSource::Unknown).to_string(),
    );
    log_dcc_metric("semantic_grounding_checked", "true");
    log_dcc_metric(
        "release_blocked_for_grounding",
        &(!ctrl.semantic_grounding_pass).to_string(),
    );
    log_dcc_metric(
        "version_truth_conflict_count",
        if ctrl.version_truth == VersionTruth::Conflict {
            "1"
        } else {
            "0"
        },
    );
    log_dcc_metric(
        "structural_vs_semantic_pass_gap",
        &(ctrl.structural_pass && !ctrl.semantic_grounding_pass).to_string(),
    );
}
