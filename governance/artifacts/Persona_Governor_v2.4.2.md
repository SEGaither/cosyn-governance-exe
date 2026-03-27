# Persona Governor v2.4.2

Compatibility - CoSyn Constitution v15.1.0 - Execution environment: RTW
compliant runtime required

Role Policy specification defining governance rules that must be
enforced by the Runtime Wrapper (RTW). The Governor does not implement
enforcement logic.

Required Governance Gates 1. Assumption Declaration Gate 2. Bias /
Tradeoff Declaration Gate 3. Confidence Declaration Gate 4. Schema
Compliance Gate 5. Deterministic Halt Rule

Required Gate Order Input → Persona routing (defined by Architect) →
Assumption Declaration Gate → Bias / Frame Declaration Gate → Reasoning
Execution → Confidence Declaration Gate → Schema Compliance Gate →
Output

Required Output Elements - assumptions - bias_frame - confidence_level -
reasoning_trace (optional)

Failure Policy - If a required gate is skipped → halt. - If schema
validation fails → halt. - If confidence declaration is missing →
halt. - If required governance structure is absent → halt.

Non‑Responsibilities The Governor does NOT contain: - execution logic -
routing implementation - schema validation implementation - telemetry
implementation - evaluation harness logic

These are implemented by the RTW.

Authority Position Constitution → Governor → Architect → RTW → Model
