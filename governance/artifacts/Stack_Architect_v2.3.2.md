# Stack Architect v2.3.2

Compatibility - CoSyn Constitution v15.1.0 - Execution environment: RTW
compliant runtime required

Role Topology specification defining system structure and component
relationships. The Architect does not implement execution logic.

Component Hierarchy Constitution ↓ Governor ↓ Architect ↓ Runtime
Wrapper (RTW) ↓ Model

Allowed System Components - Persona Router - Governance Pipeline -
Runtime Executor - Output Schema Layer

Allowed Pipeline Order persona_router → governance_pipeline →
runtime_execution → output_schema

Architectural Responsibilities - Define component relationships - Define
routing topology - Define pipeline ordering - Define artifact boundaries

Non‑Responsibilities The Architect does NOT contain: - routing
algorithms - pipeline execution logic - artifact loading
implementation - schema validation logic - halt controller
implementation

These mechanisms are implemented in the RTW.

RTW Execution Modules (reference) artifact_loader persona_router
governance_gate_engine schema_validator halt_controller
confidence_enforcer evaluation_harness telemetry_adapter
