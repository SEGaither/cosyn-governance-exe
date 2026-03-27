use std::sync::Mutex;

static AUDIT_LOG: Mutex<Vec<AuditRecord>> = Mutex::new(Vec::new());

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub input: String,
    pub input_decision: AuditDecision,
    pub llm_output: Option<String>,
    pub output_decision: Option<AuditDecision>,
    pub final_decision: AuditDecision,
    pub timestamp_start: String,
    pub timestamp_end: String,
}

pub fn append_record(record: AuditRecord) {
    if let Ok(mut log) = AUDIT_LOG.lock() {
        log.push(record);
    }
}

pub fn take_audit_log() -> Vec<AuditRecord> {
    if let Ok(mut log) = AUDIT_LOG.lock() {
        std::mem::take(&mut *log)
    } else {
        Vec::new()
    }
}

pub fn clear_audit_log() {
    if let Ok(mut log) = AUDIT_LOG.lock() {
        log.clear();
    }
}
