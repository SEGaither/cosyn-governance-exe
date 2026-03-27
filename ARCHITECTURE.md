# CoSyn Governance EXE — Architecture

**Version:** 4.1.0
**Language:** Rust (edition 2021)
**GUI:** eframe/egui
**LLM:** OpenAI gpt-4o-mini via OPENAI_API_KEY

---

## Pipeline

```
[User] -> [CoSyn EXE GUI]
              |
              v
         [Input Gate]
         Subject binding / Evidence / Ambiguity / Version truth
         (Model C: Hybrid Controlled Recognition)
              |
              v (pass)
         [LLM Client] -> OpenAI API (gpt-4o-mini, 1024 max tokens)
              |
              v
         [Output Governance]
         Structural grounding / Semantic grounding / Constitutional enforcement
         (7 checks via RuleVerdict system)
              |
              v (pass)              v (fail, attempt < 3)
         [Release Gate]       [Revision Loop]
              |               Re-prompt with specific failure details
              v                     |
         [Locked Output]       v (fail, attempt = 3)
              |               [Permanent Block]
              v
         [User sees result]
         [Telemetry -> cosyn_telemetry.log]
```

---

## Modules

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point: args, authority load, bootstrap, GUI launch |
| `cli.rs` | CLI binary: accepts prompt arg, runs full pipeline |
| `authority_loader.rs` | Embeds CGS/Governor/Architect via `include_str!`, validates identity strings |
| `orchestrator/mod.rs` | Full DCC pipeline with revision loop (3 attempts max) |
| `orchestrator/bootstrap.rs` | API key check at startup |
| `llm_client/mod.rs` | OpenAI API call, gpt-4o-mini, 1024 max tokens |
| `ui_runtime/mod.rs` | eframe/egui GUI with friendly error messages, telemetry flush |
| `governance_layer/mod.rs` | Constitutional enforcement (7 checks, RuleVerdict) |
| `validator/mod.rs` | Draft validation (empty, min length, sentinels) |
| `input_gate/integrity.rs` | Input integrity evaluation (entity grounding, intent resolution) |
| `input_gate/mod.rs` | Input gate entry point |
| `dcc/types.rs` | All DCC enums, RuntimeControl, BlockReasonCode with user_message() |
| `dcc/subject.rs` | Subject binding (Crs/UserText/Recognized/Unknown) |
| `dcc/evidence.rs` | Evidence scope evaluation |
| `dcc/ambiguity.rs` | Ambiguity state evaluation |
| `dcc/grounding.rs` | Structural + semantic grounding |
| `dcc/release.rs` | Release derivation chain |
| `dcc/block.rs` | Block evaluation (all gates in priority order) |
| `dcc/version.rs` | Version truth evaluation |
| `dcc/telemetry.rs` | DCC-specific telemetry emission |
| `telemetry/mod.rs` | Global log buffer, file persistence (cosyn_telemetry.log) |
| `audit/mod.rs` | Audit record storage |
| `core/types.rs` | DraftOutput, LockedOutput, ExecutionRequest, StageRecord |
| `core/errors.rs` | CosynError enum |
| `core/stage.rs` | Stage enum |
| `config_loader/mod.rs` | Configuration loading |
| `state_store/mod.rs` | State storage |
| `packager/mod.rs` | Packaging |
| `output_mode.rs` | Standard vs Artifact render modes |

---

## Design Principles

- **Fail closed.** On any ambiguous or invalid condition, the system blocks.
- **Constraint-first.** No rules-based or flowchart logic. Constitutional enforcement only.
- **Deterministic.** Same input produces same governance decision.
- **Narrow scope.** One LLM (gpt-4o-mini), one UI (egui). No multi-model, no multi-UI.
- **Embedded authority.** Governance artifacts compiled into the binary via `include_str!`. No external files required at runtime.
- **Observable.** Every stage transition logged. Telemetry persisted to file.

---

## Embedded Authority Files

Located at `governance/artifacts/` (compiled into binary):

| File | Identity | Validates Against |
|------|----------|-------------------|
| cosyn-constitution-v15.1.0.md | CoSyn Constitution v15.1.0 | `"CoSyn Constitution v15.1.0"` |
| Persona_Governor_v2.4.2.md | Persona Governor v2.4.2 | `"Persona Governor v2.4.2"` |
| Stack_Architect_v2.3.2.md | Stack Architect v2.3.2 | `"Stack Architect v2.3.2"` |

---

## Subject Classification (Model C)

| Subject Type | Source | Behavior |
|-------------|--------|----------|
| CRS-bound | `SubjectSource::Crs` | Full constraint evaluation |
| User-provided text | `SubjectSource::UserText` | Full constraint evaluation |
| Recognized but unbound | `SubjectSource::Recognized` | Cooperative mode (no fabrication, may proceed) |
| Unknown / unresolved | `SubjectSource::Unknown` | Hard block (BR-SUBJECT-UNKNOWN) |

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| serde | 1.0 | Serialization |
| serde_json | 1.0 | JSON handling |
| anyhow | 1.0 | Error handling |
| log | 0.4 | Logging facade |
| env_logger | 0.10 | Logger |
| uuid | 1 (v4) | Unique identifiers |
| reqwest | 0.12 (blocking, json) | HTTP client for OpenAI API |
| eframe | 0.31 | GUI framework |
