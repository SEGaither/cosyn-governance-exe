# CoSyn Governance EXE

Standalone runtime wrapper that enforces CoSyn constitutional governance over all user-to-LLM interaction. Ships as a single downloadable Windows EXE with no external artifact dependencies.

## What It Does

Every user prompt passes through a deterministic governance pipeline before reaching the LLM, and every LLM response passes through constitutional enforcement before reaching the user. The system fails closed on any invalid condition.

```
User -> [Input Gate] -> [LLM] -> [Output Governance] -> [Release or Block]
```

**Input Gate:** Subject binding, evidence evaluation, ambiguity detection, version truth check (Model C: Hybrid Controlled Recognition).

**Output Governance:** Structural grounding, semantic grounding, constitutional enforcement (7 checks), sentinel detection. On failure, the system retries with specific failure feedback (up to 2 revisions), then permanently blocks.

**Authority:** Three governance artifacts are embedded at compile time:
- CoSyn Constitution v15.1.0
- Persona Governor v2.4.2
- Stack Architect v2.3.2

## Quick Start

1. Download `cosyn-v4.1.0.exe` from [Releases](https://github.com/SEGaither/cosyn-governance-exe/releases)
2. Set your OpenAI API key: `set OPENAI_API_KEY=sk-...`
3. Run the EXE

The GUI opens. Type a prompt. The system validates, sends to gpt-4o-mini, evaluates the response, and releases or blocks it.

> **Note:** Windows may show a SmartScreen warning for unsigned executables. This is expected for open-source Rust builds.

### CLI

A command-line interface is also available for scripting and testing:

```
cosyn-cli "Explain how photosynthesis works"
cosyn-cli --version
cosyn-cli --help
```

## Building from Source

**Requirements:** Rust toolchain (edition 2021)

```bash
cargo build --release
```

Binaries output to `target/release/`:
- `cosyn.exe` (GUI)
- `cosyn-cli.exe` (CLI)

## Running Tests

```bash
cargo test
```

30 tests across 4 suites (governance layer, DCC enforcement, audit records, telemetry events). Two tests are ignored by default (require live API key). To run all tests including live API:

```bash
set OPENAI_API_KEY=sk-...
cargo test -- --ignored
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full pipeline specification.

## Governance Artifacts

The `governance/` directory contains the constitutional documents this EXE enforces:

- `governance/artifacts/` — Embedded authority files (compiled into the binary)
- `governance/constitution/` — CoSyn Constitution v15.1.0 (reference copy)
- `governance/legal/` — Legal framework (patents, trademark, fork policy, security)
- `governance/glossary.md` — Terminology reference

## Project History

This EXE is the standalone implementation of CoSyn governance. The original Python/FastAPI middleware reference implementation lives at [cosyn-runtime-wrapper](https://github.com/SEGaither/cosyn-runtime-wrapper).

Build history and technical details are in `docs/build-reports/`.

## License

Source Available License for non-commercial use. Commercial use requires a separate license agreement. See [LICENSE](LICENSE) and [LICENSE-COMMERCIAL](LICENSE-COMMERCIAL).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions must comply with the CoSyn Constitution and governance architecture.

## Security

Report vulnerabilities to cosyn.dce@gmail.com. See [governance/legal/SECURITY.md](governance/legal/SECURITY.md).

## Contact

- Shane Gaither
- cosyn.dce@gmail.com
- [Substack](https://substack.com/@shanegaither/posts)
