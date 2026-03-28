# CoSyn Live Testing Methodology — Design Spec

**Date:** 2026-03-28
**Version under test:** 4.1.0
**LLM target:** OpenAI gpt-4o-mini
**Author:** Shane Gaither
**Build agent:** Claude Opus 4.6

---

## Purpose

Establish a reusable live testing harness that runs cold-start prompts through the CoSyn CLI, compares results against declared expectations, and produces a plain-English markdown report. This testing round provides the data needed to:

- Measure false-block and false-pass rates for normal and edge-case use
- Disposition the input gate tuning gap (vague prompts passing through)
- Satisfy Phase 2 stop condition: "Runtime false block rate is low"
- Validate output quality, completeness, and format before GitHub Release

### Phase 2 stop condition threshold

The Phase 2 stop condition "Runtime false block rate is low" is met when:
- **Baseline false-block rate = 0%** — no baseline prompt is incorrectly blocked
- **Stress false-pass rate is documented** — every mismatch has a disposition (accept, or file for tuning)

---

## Scope

### In scope

- 25 cold-start, zero-context, first-message prompts across 3 categories
- Scripted batch execution via `cosyn-cli`
- Per-prompt telemetry isolation
- Markdown report with plain-English tables
- Cost tracking with $0.50 ceiling and auto-abort
- Reusable artifact for re-running after code changes

### Out of scope

- No code changes to CoSyn
- No threshold tuning (comes after reviewing results)
- No adversarial, injection, or jailbreak testing (future exercise)
- No GUI testing (CLI only)
- No modifications to GitHub repo (local only unless explicitly committed)

---

## Components

| Component | Path | Purpose |
|-----------|------|---------|
| Prompt manifest | `tests/live/cosyn-live-test-manifest.csv` | 25 prompts with category and expected result |
| Harness script | `tests/live/run-live-tests.sh` | Reads manifest, runs CLI, captures results |
| Generated report | `tests/live/cosyn-live-test-report-YYYY-MM-DD.md` | Summary tables and per-prompt results |
| Telemetry snapshots | `tests/live/telemetry-snapshots/prompt-NN.log` | Per-prompt telemetry log copies |

---

## Input Gate Decision Logic

The input gate (`src/input_gate/integrity.rs`) evaluates prompts through a 9-step pipeline. The checks that determine pass or block for this test suite are:

1. **Empty input** — blank or whitespace-only input is rejected
2. **Token count** — input is split on whitespace, punctuation stripped
3. **Unresolved reference check** — if total tokens < 3 AND reference tokens (it, this, that) are present without known entities, the prompt is blocked
4. **Entity grounding** — capitalized tokens (length > 2) must be either "cosyn" or defined inline (with is, =, refers, means)
5. **Intent resolvability** — the prompt must have one of: imperative opening, question word (what, how, why, when, where, ?), or relational keyword (is, does, should, can, has, are) with 3+ tokens

Key implications for expected results:
- Prompts with >= 3 tokens bypass the reference-token check even if they contain "it", "this", "that"
- Single-word or two-word prompts with no question word and no imperative are blocked under intent resolvability
- Gibberish is blocked under intent resolvability (no question words, no imperative, no relational keywords)
- Vague multi-word prompts with reference tokens but >= 3 tokens will likely **pass** the input gate — this is the known tuning gap

---

## Prompt Manifest Format

Plain text, pipe-delimited, one prompt per line. Lines starting with `#` are comments.

```
category | prompt | expected
```

- `category`: one of `baseline`, `stress`, `ux`
- `expected`: one of `pass`, `block`
- Prompts must not contain the pipe character `|`. If needed in future, switch to CSV with quoted fields.

---

## Prompt List

All prompts are episodic first messages. No prompt assumes prior conversation, shared state, or follow-up from a previous exchange.

### Baseline (10 prompts) — Routine requests a real user would send on first contact

| # | Prompt | Expected | Rationale |
|---|--------|----------|-----------|
| 1 | What is the capital of France? | pass | Simple factual question |
| 2 | Explain how photosynthesis works in three sentences. | pass | Bounded instructional request |
| 3 | List five benefits of regular exercise. | pass | Enumerated factual request |
| 4 | What is the difference between TCP and UDP? | pass | Technical comparison, clear subject |
| 5 | Summarize the plot of Romeo and Juliet in one paragraph. | pass | Constrained creative summary |
| 6 | How does a mortgage work? | pass | Common knowledge question |
| 7 | Write a professional email declining a meeting invitation. | pass | Realistic artifact request |
| 8 | What causes inflation? | pass | Economics explainer, clear intent |
| 9 | Describe the water cycle for a fifth grader. | pass | Audience-targeted explanation |
| 10 | What are the pros and cons of remote work? | pass | Balanced analysis request |

### Stress (10 prompts) — Sloppy, vague, or incomplete cold-start inputs

| # | Prompt | Expected | Gate rule | Rationale |
|---|--------|----------|-----------|-----------|
| 11 | fix it | block | Intent resolvability — 2 tokens, no question word, "it" is reference token but < 3 tokens triggers reference check too | Two words, no grounded subject, unresolvable reference |
| 12 | Do that thing with it | pass (tuning gap) | 6 tokens >= 3, bypasses reference check. Imperative opening detected ("Do"). Intent resolvable. | Known tuning gap — vague with unresolvable references but passes gate rules. Mismatch documents the gap. |
| 13 | yes | block | Intent resolvability — 1 token, no question word, no imperative, no relational keyword | Single-word affirmation, no parseable intent |
| 14 | more | block | Intent resolvability — 1 token, no question word, no imperative, no relational keyword | Single-word continuation, no parseable intent |
| 15 | Can you help me? | pass | 4 tokens, question word "Can" (relational), question mark present | Valid question with resolvable intent, even if vague |
| 16 | I need to do something with the thing from before | pass (tuning gap) | 11 tokens >= 3, bypasses reference check. Relational keyword "need" not in gate list, but token count and structure pass intent check. | Known tuning gap — unresolvable references but passes gate rules. Mismatch documents the gap. |
| 17 | ok do it | block | 3 tokens, "it" is reference token. Intent check: "do" may register as imperative but "ok" is not capitalized. Edge case — may pass or block. | Borderline — expected block but may pass. Either result is informative. |
| 18 | Write a project plan with placeholder sections to fill in later | pass | Imperative opening ("Write"), 12 tokens, clear structure | Legitimate request — but output may contain sentinel words. Tests output governance, not input gate. |
| 19 | asdfghjkl | block | Intent resolvability — 1 token, no question word, no imperative, no relational keyword | Keyboard mash, no parseable intent of any kind |
| 20 | Tell me about it | block | 4 tokens >= 3, bypasses reference check. But: "it" has no antecedent. Intent check: relational keyword absent, no question word. "Tell" is imperative. Edge case — may pass. | Borderline — expected block on intent grounds but imperative "Tell" may satisfy the check. Either result is informative. |

**Note on tuning gap prompts (12, 16):** These are expected to pass the current gate despite being contextually meaningless in a cold start. They are included specifically to document this behavior and provide data for the tuning gap disposition decision. A `pass` result is not a test failure — it is the expected current behavior and a known gap.

**Note on borderline prompts (17, 20):** These sit on the boundary of gate rules. The expected result is our best prediction, but either outcome is valid data. If they pass, they join the tuning gap documentation. If they block, the gate is tighter than predicted.

### UX (5 prompts) — First-contact requests that test output quality and completeness

| # | Prompt | Expected | Rationale |
|---|--------|----------|-----------|
| 21 | Write a short bedtime story about a dog who learns to fly. | pass | Creative output, tests completeness at length |
| 22 | Explain quantum computing to someone with no technical background. | pass | Tests clarity and length of instructional output |
| 23 | Give me a recipe for chicken stir fry with ingredients and steps. | pass | Structured output with multiple sections |
| 24 | What should I consider before starting a small business? | pass | Advisory output, tests substantive content |
| 25 | Draft a one-page resignation letter that is polite and professional. | pass | Realistic document artifact, tests format and tone |

---

## Test Harness Behavior

### Exit code handling

- Exit code 0 = pass (output released)
- Exit code 1 = block (output blocked by governance)
- Any other exit code = error (logged as ERROR in the actual column, always counts as mismatch, includes stderr capture for diagnosis)

### Per-prompt execution

For each manifest line:

1. Delete `cosyn_telemetry.log` (clean slate per prompt)
2. Record start time
3. Run: `cosyn-cli "<prompt>"`
4. Record end time, compute elapsed milliseconds
5. Capture: exit code, stdout, stderr
6. Copy `cosyn_telemetry.log` to `telemetry-snapshots/prompt-NN.log`
7. Parse telemetry for: attempt count, token count, block reason code (see telemetry format below)
8. Compute actual result: exit 0 = pass, exit 1 = block, other = error
9. Compute match: actual == expected
10. Estimate cost from token count, add to running total
11. If running total >= $0.50: abort and write partial report with all results collected so far

### Telemetry log format

Each line in `cosyn_telemetry.log` is a timestamped event:

```
[2026-03-27T14:32:01Z] input_received: "What is the capital of France?"
[2026-03-27T14:32:01Z] input_validation_result: allow
[2026-03-27T14:32:02Z] llm_call_start
[2026-03-27T14:32:03Z] llm_call_end: 156 tokens
[2026-03-27T14:32:03Z] output_validation_result: allow
[2026-03-27T14:32:03Z] final_release_decision: allow
```

The harness parses:
- **Attempt count**: number of `llm_call_start` events (1 = no revision, 2-3 = revision loop triggered)
- **Token count**: numeric value from `llm_call_end` line
- **Block reason code**: value from `final_release_decision` if not "allow", or from stderr

### Block reason codes

The 8 machine-emitted block reason codes (from `src/dcc/types.rs`):

| Code | Meaning |
|------|---------|
| BR-SUBJECT-UNKNOWN | Subject not recognized or bound |
| BR-EVIDENCE-UNSAT | Evidence requirements not satisfied |
| BR-AMBIGUITY | Input classified as ambiguous |
| BR-STRUCTURAL-FAIL | Output failed structural grounding |
| BR-GROUNDING-FAIL | Output failed semantic grounding |
| BR-VERSION-CONFLICT | Version conflict detected |
| BR-VERSION-UNDEFINED | Version undefined |
| BR-RELEASE-DENIED | Final release gate denied |

### Cost estimation

gpt-4o-mini pricing (OpenAI, as of 2026-03): $0.15 per 1M input tokens, $0.60 per 1M output tokens. Estimated total for 25 prompts averaging 200 output tokens: approximately $0.003. The $0.50 ceiling provides over 100x headroom.

---

## Report Format

File: `tests/live/cosyn-live-test-report-YYYY-MM-DD.md`

### Computed metrics section

| Metric | Value | Threshold |
|--------|-------|-----------|
| Baseline false-block rate (baseline blocks / 10) | ...% | Must be 0% for Phase 2 |
| Stress false-pass rate (unexpected stress passes / stress expected blocks) | ...% | Documented, no threshold |
| Revision loop trigger rate (prompts with attempts > 1 / total passed) | ...% | Informational |
| Estimated API cost | $X.XX | Must be under $0.50 |

### Summary section

| Category | Total | Pass | Block | Error | Match | Mismatch |
|----------|-------|------|-------|-------|-------|----------|
| Baseline | 10 | ... | ... | ... | ... | ... |
| Stress | 10 | ... | ... | ... | ... | ... |
| UX | 5 | ... | ... | ... | ... | ... |
| **Total** | **25** | ... | ... | ... | ... | ... |

### Mismatches section (only if any exist)

| # | Category | Prompt | Expected | Actual | Notes |
|---|----------|--------|----------|--------|-------|
| 12 | stress | Do that thing with it | pass (tuning gap) | pass | Confirmed: input gate allows vague prompt through as predicted |

### Per-category result tables

| # | Prompt | Expected | Actual | Match | Attempts | Notes |
|---|--------|----------|--------|-------|----------|-------|
| 1 | What is the capital of France? | pass | pass | Yes | 1 | Clean response |

### Telemetry notes section

Any anomalies observed during the run:
- Revision loop triggers (which prompts, how many attempts, what failed)
- Unusual token counts (very high or very low)
- Unexpected block reason codes
- Error exit codes (non-0, non-1)

---

## What This Produces

- **False-block rate**: baseline prompts that got blocked when they should not have (target: 0%)
- **False-pass rate**: stress prompts that got through when they should not have (documented with disposition)
- **Revision loop data**: which prompts triggered retries and how many
- **Data to disposition the input gate tuning gap** (open item #4) — prompts 12, 16 and borderline 17, 20 provide specific evidence
- **Evidence for Phase 2 stop condition** ("Runtime false block rate is low" — met at 0% baseline false-block)
- **Reusable artifact** for re-running after any code change

---

## Constraints

- All prompts are episodic first messages with zero prior context
- $0.50 cost ceiling with auto-abort
- Realistic use only — no synthetic adversarial or injection attacks
- Per-prompt telemetry isolation for clean analysis
- Report in plain English tables
- Prompts must not contain the pipe character

---

**End of Spec**
