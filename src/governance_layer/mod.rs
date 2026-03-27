use crate::core::errors::{CosynError, CosynResult};
use crate::core::types::DraftOutput;

/// Constitutional enforcement for CoSyn.
///
/// Every draft must pass all constitutional rules before it can be locked.
/// These are explicit, deterministic checks. The system decides, not the model.

const MIN_MEANINGFUL_LENGTH: usize = 20;
const MAX_DRAFT_LENGTH: usize = 10_000;

const BLOCKED_SENTINELS: &[&str] = &[
    "todo",
    "placeholder",
    "mock failure",
    "not yet implemented",
    "fixme",
    "lorem ipsum",
];

#[derive(Debug, Clone)]
pub struct RuleVerdict {
    pub rule: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// Run all constitutional enforcement checks against the input and draft.
/// Returns Ok(()) if every rule passes.
/// Returns Err on the first failing rule — draft must not proceed to Lock.
pub fn enforce(input: &str, draft: &DraftOutput) -> CosynResult<()> {
    let verdicts = evaluate_all(input, draft);
    for v in &verdicts {
        if !v.passed {
            return Err(CosynError::Governance(format!(
                "rule '{}': {}",
                v.rule, v.detail
            )));
        }
    }
    Ok(())
}

/// Evaluate all constitutional rules. Returns the full verdict list.
pub fn evaluate_all(input: &str, draft: &DraftOutput) -> Vec<RuleVerdict> {
    let input_trimmed = input.trim();
    let draft_trimmed = draft.text.trim();

    vec![
        rule_input_present(input_trimmed),
        rule_draft_present(draft_trimmed),
        rule_draft_min_length(draft_trimmed),
        rule_draft_max_length(draft_trimmed),
        rule_no_sentinel_text(draft_trimmed),
        rule_no_verbatim_echo(input_trimmed, draft_trimmed),
        rule_no_empty_structure(draft_trimmed),
    ]
}

fn rule_input_present(input: &str) -> RuleVerdict {
    RuleVerdict {
        rule: "input-present",
        passed: !input.is_empty(),
        detail: if input.is_empty() {
            "input is empty or whitespace-only".into()
        } else {
            "input present".into()
        },
    }
}

fn rule_draft_present(draft: &str) -> RuleVerdict {
    RuleVerdict {
        rule: "draft-present",
        passed: !draft.is_empty(),
        detail: if draft.is_empty() {
            "draft is empty or whitespace-only".into()
        } else {
            "draft present".into()
        },
    }
}

fn rule_draft_min_length(draft: &str) -> RuleVerdict {
    let ok = draft.len() >= MIN_MEANINGFUL_LENGTH;
    RuleVerdict {
        rule: "draft-min-length",
        passed: ok,
        detail: if ok {
            "draft meets minimum length".into()
        } else {
            format!(
                "draft too short ({} chars, minimum {})",
                draft.len(),
                MIN_MEANINGFUL_LENGTH
            )
        },
    }
}

fn rule_draft_max_length(draft: &str) -> RuleVerdict {
    let ok = draft.len() <= MAX_DRAFT_LENGTH;
    RuleVerdict {
        rule: "draft-max-length",
        passed: ok,
        detail: if ok {
            "draft within length bounds".into()
        } else {
            format!(
                "draft exceeds maximum length ({} chars, limit {})",
                draft.len(),
                MAX_DRAFT_LENGTH
            )
        },
    }
}

fn rule_no_sentinel_text(draft: &str) -> RuleVerdict {
    let lower = draft.to_lowercase();
    for sentinel in BLOCKED_SENTINELS {
        if lower.contains(sentinel) {
            return RuleVerdict {
                rule: "no-sentinel-text",
                passed: false,
                detail: format!("draft contains blocked sentinel: \"{}\"", sentinel),
            };
        }
    }
    RuleVerdict {
        rule: "no-sentinel-text",
        passed: true,
        detail: "no sentinel text detected".into(),
    }
}

fn rule_no_verbatim_echo(input: &str, draft: &str) -> RuleVerdict {
    if input.is_empty() || draft.is_empty() {
        return RuleVerdict {
            rule: "no-verbatim-echo",
            passed: true,
            detail: "skipped (empty input or draft)".into(),
        };
    }

    let input_lower = input.to_lowercase();
    let draft_lower = draft.to_lowercase();

    let is_echo = draft_lower == input_lower
        || (draft_lower.contains(&input_lower)
            && input_lower.len() as f64 / draft_lower.len() as f64 > 0.9);

    RuleVerdict {
        rule: "no-verbatim-echo",
        passed: !is_echo,
        detail: if is_echo {
            "draft is a verbatim echo of input".into()
        } else {
            "draft is not an echo".into()
        },
    }
}

fn rule_no_empty_structure(draft: &str) -> RuleVerdict {
    if draft.is_empty() {
        return RuleVerdict {
            rule: "no-empty-structure",
            passed: true,
            detail: "skipped (empty draft handled by draft-present)".into(),
        };
    }

    let all_structural = draft.lines().all(|line| {
        let t = line.trim();
        t.is_empty()
            || t.starts_with('#')
            || t.starts_with("---")
            || t.starts_with("```")
            || t == ">"
            || t == "-"
            || t == "*"
    });

    RuleVerdict {
        rule: "no-empty-structure",
        passed: !all_structural,
        detail: if all_structural {
            "draft contains only structural markers, no substantive content".into()
        } else {
            "draft has substantive content".into()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(text: &str) -> DraftOutput {
        DraftOutput {
            text: text.into(),
        }
    }

    // ── Pass cases ──

    #[test]
    fn pass_normal_draft() {
        let d = draft("[draft] Governed response to: Summarize the governance policy");
        assert!(enforce("Summarize the governance policy", &d).is_ok());
    }

    #[test]
    fn pass_all_verdicts_true() {
        let d = draft("[draft] Governed response to: Summarize the governance policy");
        let verdicts = evaluate_all("Summarize the governance policy", &d);
        assert!(verdicts.iter().all(|v| v.passed), "all rules should pass");
    }

    // ── Block cases ──

    #[test]
    fn block_empty_input() {
        let d = draft("[draft] some valid response text here");
        let err = enforce("", &d).unwrap_err();
        assert!(err.to_string().contains("input-present"));
    }

    #[test]
    fn block_whitespace_input() {
        let d = draft("[draft] some valid response text here");
        let err = enforce("   \n\t  ", &d).unwrap_err();
        assert!(err.to_string().contains("input-present"));
    }

    #[test]
    fn block_empty_draft() {
        let err = enforce("some input", &draft("")).unwrap_err();
        assert!(err.to_string().contains("draft-present"));
    }

    #[test]
    fn block_whitespace_draft() {
        let err = enforce("some input", &draft("   \n\t  ")).unwrap_err();
        assert!(err.to_string().contains("draft-present"));
    }

    #[test]
    fn block_short_draft() {
        let err = enforce("some input", &draft("too short")).unwrap_err();
        assert!(err.to_string().contains("draft-min-length"));
    }

    #[test]
    fn block_exceeds_max_length() {
        let long = "a".repeat(10_001);
        let err = enforce("some input", &draft(&long)).unwrap_err();
        assert!(err.to_string().contains("draft-max-length"));
    }

    #[test]
    fn block_sentinel_todo() {
        let d = draft("[draft] Governed response to: TODO fix this later");
        let err = enforce("TODO fix this later", &d).unwrap_err();
        assert!(err.to_string().contains("no-sentinel-text"));
    }

    #[test]
    fn block_sentinel_placeholder() {
        let d = draft("[draft] This is a placeholder response for testing");
        let err = enforce("test request", &d).unwrap_err();
        assert!(err.to_string().contains("no-sentinel-text"));
    }

    #[test]
    fn block_sentinel_fixme() {
        let d = draft("[draft] Response needs FIXME before release");
        let err = enforce("some input", &d).unwrap_err();
        assert!(err.to_string().contains("no-sentinel-text"));
    }

    #[test]
    fn block_verbatim_echo() {
        let input = "Summarize the governance policy";
        let err = enforce(input, &draft(input)).unwrap_err();
        assert!(err.to_string().contains("no-verbatim-echo"));
    }

    #[test]
    fn block_structural_only() {
        let d = draft("# Header\n---\n```\n```\n---");
        let err = enforce("some input", &d).unwrap_err();
        assert!(err.to_string().contains("no-empty-structure"));
    }

    // ── Verify blocked cases stop before Lock ──

    #[test]
    fn blocked_draft_never_reaches_orchestrator_lock() {
        // A blocked draft returns Err from enforce().
        // The orchestrator checks enforce() before Lock.
        // If enforce() returns Err, the orchestrator returns early — Lock never runs.
        let d = draft("");
        let result = enforce("input", &d);
        assert!(result.is_err(), "empty draft must be blocked before lock");
    }
}
