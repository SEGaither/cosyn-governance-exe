use crate::dcc::types::EvidenceScope;

/// Evaluate evidence scope. Evidence is SATISFIED when:
/// - input is non-empty
/// - input contains resolvable content (not just structural markers)
/// Returns UNSATISFIED otherwise. No fallback.
pub fn evaluate_evidence(input: &str) -> EvidenceScope {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return EvidenceScope::Unsatisfied;
    }

    // All-structural content (only markdown markers, no substance)
    let all_structural = trimmed.lines().all(|line| {
        let t = line.trim();
        t.is_empty()
            || t.starts_with('#')
            || t.starts_with("---")
            || t.starts_with("```")
            || t == ">"
            || t == "-"
            || t == "*"
    });

    if all_structural {
        return EvidenceScope::Unsatisfied;
    }

    EvidenceScope::Satisfied
}
