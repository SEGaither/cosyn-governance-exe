use crate::core::stage::Stage;
use crate::core::types::StageRecord;

pub struct StateStore {
    pub current_stage: Stage,
    pub log: Vec<StageRecord>,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StateStore {
    pub fn new() -> Self {
        Self {
            current_stage: Stage::Input,
            log: Vec::new(),
        }
    }

    pub fn advance(&mut self, stage: Stage, passed: bool, detail: &str) {
        self.log.push(StageRecord {
            stage,
            passed,
            detail: detail.into(),
        });
        self.current_stage = stage;
    }
}
