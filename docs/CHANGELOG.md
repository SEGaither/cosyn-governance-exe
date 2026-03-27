# Changelog

## v4.1.0 (2026-03-27)

Initial release in the `cosyn-governance-exe` repository.

### Runtime Stabilization (Phase 1 complete)

- Fixed authority validator strings to match embedded governance file contents
- Cleaned sentinel blocklist — removed overly broad tokens (undefined, hack, xxx, tbd)
- Increased LLM max tokens from 200 to 1024
- Relaxed input gate thresholds for short/reference-heavy prompts
- Added user-friendly error messages (BlockReasonCode to plain English)
- Added API key check at startup with GUI warning
- Replaced CLI test harness with functional CLI (accepts real input via args)
- Implemented revision loop (3 attempts max, fail-closed on exhaustion)
- Removed redundant bootstrap authority check
- Verified persistent telemetry logging to cosyn_telemetry.log

### Repository

- Separated from [cosyn-runtime-wrapper](https://github.com/SEGaither/cosyn-runtime-wrapper) (Python/FastAPI) into dedicated Rust EXE repo
- Renamed governance artifact files to match content identity strings
- Updated author attribution to Shane Gaither
- 30 tests passing, 0 failed, 2 ignored (require live API key)
- Live end-to-end verified against OpenAI gpt-4o-mini

## Prior History

Build history from v1.0 through v4.0 is documented in `docs/build-reports/cosyn-v4.1.0-technical-report.md`. Source history is in the original [cosyn-runtime-wrapper](https://github.com/SEGaither/cosyn-runtime-wrapper) repository.
