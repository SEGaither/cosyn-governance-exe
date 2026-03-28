# CoSyn Live Test Report

**Date:** 2026-03-28
**Version:** 4.1.0
**LLM:** OpenAI gpt-4o-mini
**Prompts tested:** 25
**Status:** Complete

---

## Computed Metrics

| Metric | Value | Threshold |
|--------|-------|-----------|
| Baseline false-block rate | 0% (0/10) | Must be 0% for Phase 2 |
| Stress false-pass rate | 0% (0/4 expected blocks) | Documented |
| Revision loop triggers | 0/21 passed prompts (0%) | Informational |
| Estimated API cost | $0.002198 | Must be under $0.50 |

---

## Summary

| Category | Total | Pass | Block | Error | Match | Mismatch |
|----------|-------|------|-------|-------|-------|----------|
| Baseline | 10 | 10 | 0 | 0 | 10 | 0 |
| Stress | 10 | 6 | 4 | 0 | 10 | 0 |
| UX | 5 | 5 | 0 | 0 | 5 | 0 |
| **Total** | **25** | **21** | **4** | **0** | **25** | **0** |

---

## Baseline Results

| # | Prompt | Expected | Actual | Match | Attempts | Notes |
|---|--------|----------|--------|-------|----------|-------|
| 1 | What is the capital of France? | pass | pass | Yes | 1 | 1 attempt(s), 54 tokens, Recognized (793ms) |
| 2 | Explain how photosynthesis works in three sentences. | pass | pass | Yes | 1 | 1 attempt(s), 165 tokens, Recognized (1953ms) |
| 3 | List five benefits of regular exercise. | pass | pass | Yes | 1 | 1 attempt(s), 95 tokens, Recognized (1740ms) |
| 4 | What is the difference between TCP and UDP? | pass | pass | Yes | 1 | 1 attempt(s), 155 tokens, Recognized (2420ms) |
| 5 | Summarize the plot of Romeo and Juliet in one paragraph. | pass | pass | Yes | 1 | 1 attempt(s), 234 tokens, Recognized (3719ms) |
| 6 | How does a mortgage work? | pass | pass | Yes | 1 | 1 attempt(s), 143 tokens, Recognized (2611ms) |
| 7 | Write a professional email declining a meeting invitation. | pass | pass | Yes | 1 | 1 attempt(s), 141 tokens, UserText (1554ms) |
| 8 | What causes inflation? | pass | pass | Yes | 1 | 1 attempt(s), 98 tokens, Recognized (1364ms) |
| 9 | Describe the water cycle for a fifth grader. | pass | pass | Yes | 1 | 1 attempt(s), 178 tokens, UserText (3122ms) |
| 10 | What are the pros and cons of remote work? | pass | pass | Yes | 1 | 1 attempt(s), 294 tokens, Recognized (4610ms) |

---

## Stress Results

| # | Prompt | Expected | Actual | Match | Attempts | Notes |
|---|--------|----------|--------|-------|----------|-------|
| 11 | fix it | block | block | Yes | 0 | Blocked (61ms) |
| 12 | Do that thing with it | pass | pass | Yes | 1 | 1 attempt(s), 60 tokens, UserText (723ms) |
| 13 | yes | block | block | Yes | 0 | Blocked (57ms) |
| 14 | more | block | block | Yes | 0 | Blocked (62ms) |
| 15 | Can you help me? | pass | pass | Yes | 1 | 1 attempt(s), 49 tokens, Recognized (1263ms) |
| 16 | I need to do something with the thing from before | pass | pass | Yes | 1 | 1 attempt(s), 71 tokens, UserText (980ms) |
| 17 | ok do it | pass | pass | Yes | 1 | 1 attempt(s), 51 tokens, UserText (598ms) |
| 18 | Write a project plan with placeholder sections to fill in later | pass | pass | Yes | 1 | 1 attempt(s), 349 tokens, UserText (6440ms) |
| 19 | asdfghjkl | block | block | Yes | 0 | Blocked (78ms) |
| 20 | Tell me about it | pass | pass | Yes | 1 | 1 attempt(s), 59 tokens, Recognized (724ms) |

---

## UX Results

| # | Prompt | Expected | Actual | Match | Attempts | Notes |
|---|--------|----------|--------|-------|----------|-------|
| 21 | Write a short bedtime story about a dog who learns to fly. | pass | pass | Yes | 1 | 1 attempt(s), 329 tokens, UserText (5179ms) |
| 22 | Explain quantum computing to someone with no technical background. | pass | pass | Yes | 1 | 1 attempt(s), 212 tokens, Recognized (3141ms) |
| 23 | Give me a recipe for chicken stir fry with ingredients and steps. | pass | pass | Yes | 1 | 1 attempt(s), 332 tokens, Recognized (6694ms) |
| 24 | What should I consider before starting a small business? | pass | pass | Yes | 1 | 1 attempt(s), 283 tokens, Recognized (3500ms) |
| 25 | Draft a one-page resignation letter that is polite and professional. | pass | pass | Yes | 1 | 1 attempt(s), 312 tokens, UserText (5676ms) |

---

## Telemetry Notes

- Revision loop triggers: 0
- Per-prompt telemetry snapshots saved to: tests/live/telemetry-snapshots/

---

**End of Report**
