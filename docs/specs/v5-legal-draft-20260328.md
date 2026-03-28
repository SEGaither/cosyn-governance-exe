# CoSyn v5-legal — Draft Specification

**Date:** 2026-03-28
**Status:** DRAFT — Pending go/no-go approval
**Author:** Shane Gaither
**Build Agent:** Claude Opus 4.6

---

## Overview

CoSyn v5-legal is a purpose-built version of the CoSyn Governance EXE for legal professional services. It replaces the cloud LLM backend with a local inference server, adds data classification enforcement, session isolation, controlled external access, and enhanced telemetry for ABA compliance auditing.

**Target domain:** Legal professional services
**Operator model:** Paralegal (primary operator), Attorney (oversight and review)
**Deployment:** Single workstation, local only

---

## What v5-legal IS

A CoSyn build that:
- Runs entirely on a local LLM (no client data leaves the machine)
- Enforces data classification at the code level (Client / Internal / External)
- Isolates sessions (no cross-session data leakage)
- Controls external access (internet research allowed, client data blocked from outbound)
- Produces telemetry sufficient for ABA compliance auditing

## What v5-legal IS NOT

- Not a multi-user platform
- Not a SaaS deployment
- Not a general-purpose refactor of CoSyn
- Not backward-compatible with the OpenAI path (v4.x remains the cloud-LLM version)

---

## Versioning

**v5.0.0**, not v4.2.0. The local LLM backend and data classification layer are architectural changes, not patches. The v4.x line continues as the OpenAI-backed version. The v5.x line is the local-LLM-with-governance line.

---

## Target Hardware

| Component | Specification |
|-----------|---------------|
| GPU | 2 × NVIDIA RTX 4090 (24 GB VRAM each) |
| CPU | AMD Threadripper PRO (32-core class) |
| RAM | 256 GB DDR5 ECC |
| Storage — OS + Models | 4 TB NVMe |
| Storage — Data + Logs | 8 TB NVMe |
| OS | Ubuntu LTS (Linux) |

**Hardware verdict:** Exceeds requirements. A 32B quantized model fits on a single GPU at 20-35 tokens/sec. See feasibility assessment for full analysis.

---

## Regulatory Context

### ABA Model Rules

| Rule | Requirement | How v5-legal Addresses It |
|------|-------------|---------------------------|
| 1.6 (Confidentiality) | Lawyer shall not reveal client information | Local-only processing. No external API for client work. Data classification prevents accidental inclusion in external queries. |
| 1.1 / Comment 8 (Competence) | Lawyer must understand technology used | System is auditable. Telemetry provides complete record. Governance pipeline is deterministic and explainable. |
| 5.1 / 5.3 (Supervision) | Lawyer must supervise non-lawyer assistants | Paralegal operates; attorney reviews. Telemetry provides audit trail. All output passes through governance before release. |
| 1.4 (Communication) | Keep client informed | System does not interact with clients directly. Outputs are drafts for attorney review. |

---

## Sub-Projects

### A — Local LLM Backend

**Complexity:** Low
**Scope:** ~50 lines changed in `llm_client/mod.rs`

Replace the OpenAI API call with a local inference server endpoint. Ollama exposes an OpenAI-compatible API on localhost, making this a URL + auth swap.

Changes:
- Replace `https://api.openai.com/v1/chat/completions` with configurable localhost endpoint
- Replace `OPENAI_API_KEY` with optional local auth or none
- Add configurable model name (default: selected during A.1)
- Add configurable endpoint URL (default: `http://localhost:11434/v1/chat/completions`)
- Update cost estimation for local inference (zero marginal cost)

**Gate:** `cosyn-cli "What is the capital of France?"` returns a response from the local model.

### A.1 — Model Qualification

**Complexity:** Medium
**Scope:** Run existing 25-prompt live test harness against 2-3 candidate models

Candidate models:
- Qwen2.5-32B-Instruct (Q4_K_M) — ~20 GB, single GPU
- Mistral-Small-24B-Instruct (Q6_K) — ~20 GB, single GPU
- DeepSeek-R1-Distill-Qwen-32B (Q4) — ~20 GB, single GPU

For each model, measure:
- 25-prompt suite pass/fail rates
- Revision loop trigger frequency
- Sentinel detection frequency
- Response quality comparison to gpt-4o-mini baseline
- Tokens per second (interactive usability)
- GPU memory utilization

Decisions made here:
- Primary model selection
- Whether sentinel list needs expansion for local model behavior
- Whether system prompt needs tuning for local model

**Gate:** At least one model achieves 0% baseline false-block rate and acceptable revision rate.

### B — Session Isolation

**Complexity:** Medium
**Scope:** ~200-300 lines new code

Changes:
- Generate UUID session ID at startup (CLI: per invocation, GUI: per explicit session start)
- Add session ID to all telemetry events
- Clear all in-memory state on session boundary in GUI
- Add explicit "New Session" action in GUI (not automatic on window close)
- Add explicit document loading action distinct from prompt input
- Log session start/end boundaries in telemetry
- CLI is already isolated by design (stateless per invocation)

**Gate:** Phase 3 test — attempt to reference Session A content from Session B fails.

### C — Data Classification

**Complexity:** High
**Scope:** ~400-600 lines across multiple modules

Changes:
- Add `DataClassification` enum to `dcc/types.rs`: `Client`, `Internal`, `External`
- Add classification field to `RuntimeControl`
- Default all input to `Client` unless operator explicitly selects otherwise
- Add classification selector to GUI (dropdown: Client / Internal / External)
- Add classification parameter to CLI (`--classification client|internal|external`, default: client)
- Propagate classification through input gate and orchestrator
- Log classification on every telemetry event
- Extend input gate to accept and validate classification

Enforcement rules:
- `Client` data: never included in any outbound request. Period.
- `Internal` data: stays within local/firm network. Not transmitted externally.
- `External` data: retrieved from internet. Cannot be mixed with Client data in LLM prompts.

**Gate:** All input defaults to Client. Operator can reclassify. Telemetry logs classification on every event.

### D — Controlled External Access

**Complexity:** High
**Scope:** ~300-500 lines, new module

New module: `external_access/mod.rs`

Behavior:
- Operator explicitly enters "External Research" mode (GUI toggle or CLI flag)
- System checks that no Client-classified data is in the current prompt context
- If Client data detected in outbound buffer: block with clear error message
- Query is sent to external source (self-hosted SearXNG or similar search API)
- Response is tagged `External` in the pipeline
- External data is presented to operator separately
- External data cannot be concatenated with Client data in subsequent LLM prompts

The external access module sits OUTSIDE the main governance pipeline. It does not pass through the LLM. It is a controlled research tool, not a generation tool.

**Gate:** Phase 4 (external research works, no client data in outbound) and Phase 5 (attempt to mix client + external data is blocked or flagged).

### E — Enhanced Telemetry

**Complexity:** Medium
**Scope:** ~100-200 lines changed in existing modules

Extend `telemetry/mod.rs` and related:
- Session ID on every event
- Data classification on every event
- Reference source tag on every event (local / internal / external)
- Session start/end boundary markers
- Structured log format (consider JSON lines for machine parsing, alongside human-readable)

The enhanced telemetry must support:
- Complete audit trail retrieval per session
- Per-client data isolation in logs (session boundaries)
- Proof of data classification for each input
- Proof that no client data appeared in external queries

**Gate:** Phase 6 — logs include all inputs, outputs, session separation, data source classification, and action sequence.

### F — Validation Test

**Complexity:** Medium
**Scope:** Execute 7-phase test procedure, document results

| Phase | Test | Pass Criteria |
|-------|------|---------------|
| 1 | Session A — Client work | Summarize, identify issues, draft response from internal docs |
| 2 | Session B — Separate matter | Repeat with unrelated docs |
| 3 | Isolation | No access to Session A content from Session B |
| 4 | External research | Generic query succeeds, no client data transmitted |
| 5 | Data separation | System prevents/flags mixing client + external data |
| 6 | Telemetry review | Complete structured logs with session, classification, source |
| 7 | Constraints | External transmission blocked, large doc load, multiple sessions |

**Deliverable:** Pass/Fail report with performance metrics, resource utilization, observed issues, recommended adjustments.

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation | Sub-Project |
|------|-----------|--------|------------|-------------|
| Local model output quality triggers excessive revision loops | Medium | Medium | Test with live harness in A.1 before proceeding. Tune system prompt. Try multiple models. | A.1 |
| Local model produces sentinels/placeholders more than gpt-4o-mini | Medium | Low | Governance catches these. Measure in A.1. Expand sentinel list if needed. | A.1 |
| Data classification is operator-dependent (human error) | High | High | Default all input to Client. Fail-closed on ambiguity. | C |
| External access pathway introduces attack surface | Low | High | Enforce at code level. No client-tagged data in outbound buffer — code-level gate, not policy. | D |
| 70B model spans both GPUs, reduces throughput | Low | Low | 32B on single GPU is the sweet spot. Only escalate if quality requires it. | A.1 |
| Ollama stability under sustained use | Low | Medium | Session restart between matters. Monitor memory. | B |
| Bar ethics opinion on AI use changes | Medium | High | Architecture is the defense — governance pipeline, telemetry, data classification, fail-closed design. | All |

---

## Design Decisions (Pending)

| # | Decision | Lean | Rationale |
|---|----------|------|-----------|
| 1 | Separate repo or branch? | Separate repo | Different target, deployment, licensing. v4.x and v5.x should not share commit history. |
| 2 | External research: search API or web fetch? | Self-hosted SearXNG | Controlled, auditable, no browser dependency. |
| 3 | Data classification: dropdown or inferred? | Explicit dropdown, default Client | Inference is unreliable and creates liability. |
| 4 | Internal network access (DMS/SharePoint)? | Defer to v5.1 | File system first. Network integration adds complexity. |
| 5 | Legal-specific test manifest? | Yes | Legal prompts test different governance behaviors. |

---

## CoSyn Tweaks Required

| Tweak | Module | Sub-Project |
|-------|--------|-------------|
| Replace OpenAI API call with local inference endpoint | `llm_client/mod.rs` | A |
| Add configurable model name and endpoint URL | `llm_client/mod.rs` | A |
| Add `DataClassification` enum | `dcc/types.rs` | C |
| Add classification field to `RuntimeControl` | `dcc/types.rs` | C |
| Add classification parameter to input gate | `input_gate/integrity.rs` | C |
| Add session ID to telemetry events | `telemetry/mod.rs` | B |
| Add data source classification to telemetry | `telemetry/mod.rs` | E |
| State clearing on session boundary in GUI | `ui_runtime/mod.rs` | B |
| Document loading action (distinct from prompt input) | `ui_runtime/mod.rs` | B |
| New controlled external access module | New: `external_access/mod.rs` | D |
| Classification enforcement at external access boundary | New: `external_access/mod.rs` | D |
| Sentinel list review for local model behavior | `governance_layer/mod.rs` | A.1 |

---

## Acceptance Gate

v5-legal is complete when all 7 phases of the validation test procedure pass:

- Client data never leaves the local or internal network environment
- External research is allowed but strictly separated
- No cross-session data leakage
- Telemetry is complete and structured
- System remains stable
- Response time is usable for interactive work

---

## References

- Feasibility assessment: `C:\1-cosyn-local-repo\use-case\legal-use-case\cosyn-v4.1.1-local-llm-feasibility-assessment.pdf`
- v4.1.1 build log: `docs/build-reports/cosyn-v4.1.1-build-log.md`
- Live testing methodology spec: `docs/specs/2026-03-28-live-testing-methodology-design.md`
- CoSyn Constitution v15.1.0: `governance/artifacts/cosyn-constitution-v15.1.0.md`

---

**End of Draft Specification**
