use crate::core::stage::Stage;

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub id: String,
    pub input: String,
    pub canonical_subject: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DraftOutput {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct LockedOutput {
    pub text: String,
    pub locked: bool,
    pub block_reason_code: Option<crate::dcc::types::BlockReasonCode>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct StageRecord {
    pub stage: Stage,
    pub passed: bool,
    pub detail: String,
}
