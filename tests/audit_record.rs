//! Audit record integration tests.

use cosyn::audit::{AuditRecord, AuditDecision, take_audit_log, clear_audit_log};

#[test]
fn audit_record_is_append_only() {
    clear_audit_log();

    let r1 = AuditRecord {
        input: "test input 1".into(),
        input_decision: AuditDecision::Allow,
        llm_output: Some("draft output".into()),
        output_decision: Some(AuditDecision::Allow),
        final_decision: AuditDecision::Allow,
        timestamp_start: "2026-03-26T00:00:00Z".into(),
        timestamp_end: "2026-03-26T00:00:01Z".into(),
    };
    let r2 = AuditRecord {
        input: "test input 2".into(),
        input_decision: AuditDecision::Deny,
        llm_output: None,
        output_decision: None,
        final_decision: AuditDecision::Deny,
        timestamp_start: "2026-03-26T00:00:02Z".into(),
        timestamp_end: "2026-03-26T00:00:03Z".into(),
    };

    cosyn::audit::append_record(r1);
    cosyn::audit::append_record(r2);

    let log = take_audit_log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].input, "test input 1");
    assert_eq!(log[1].input, "test input 2");
}

#[test]
fn audit_record_fields_nullable_when_blocked() {
    clear_audit_log();

    let r = AuditRecord {
        input: "blocked input".into(),
        input_decision: AuditDecision::Deny,
        llm_output: None,
        output_decision: None,
        final_decision: AuditDecision::Deny,
        timestamp_start: "2026-03-26T00:00:00Z".into(),
        timestamp_end: "2026-03-26T00:00:01Z".into(),
    };
    cosyn::audit::append_record(r);

    let log = take_audit_log();
    assert_eq!(log.len(), 1);
    assert!(log[0].llm_output.is_none());
    assert!(log[0].output_decision.is_none());
}

#[test]
fn take_audit_log_drains() {
    clear_audit_log();

    cosyn::audit::append_record(AuditRecord {
        input: "x".into(),
        input_decision: AuditDecision::Allow,
        llm_output: None,
        output_decision: None,
        final_decision: AuditDecision::Deny,
        timestamp_start: "t0".into(),
        timestamp_end: "t1".into(),
    });

    let first = take_audit_log();
    assert_eq!(first.len(), 1);

    let second = take_audit_log();
    assert!(second.is_empty(), "take must drain the log");
}
