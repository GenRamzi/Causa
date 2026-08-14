# Causa Completion Report

## Status

The previously empty `GenRamzi/Causa` repository has been turned into a buildable and testable **Causa v0.1 local preview**. This completion pass extends the original implementation with a recording OpenAI-compatible proxy, streaming responses, replay outputs, lineage-preserving forks, structured diffs, guard decision tapes, a tape regression command, SDK alignment, and v0.1 schema documentation.

## Implemented capabilities

| Area | Available capability |
|---|---|
| Core | Causal events, parent links, provenance labels, BLAKE3 content hashes, Merkle roots, and integrity validation. |
| Signatures | Optional Ed25519 signing and verification. A signature authenticates the envelope; it does not encrypt content. |
| Tape format | Versioned JSON envelope with metadata, event nodes, lineage fields, integrity data, and optional signatures. |
| Recording | Local child-process recording plus an OpenAI-compatible proxy that records request and response events. |
| Streaming | Server-Sent Events responses for `stream: true`, with the same response captured into the tape. |
| Replay | Offline event inspection, bounded JSON output overrides, replay output tapes, and recorded-response serving. |
| Timeline tools | Forks preserve source run metadata and hashes; diffs report changed fields; bisect reports the first divergent node and blast radius. |
| Guard | A documented policy grammar, inherited-label evaluation, enforce/audit modes, and optional guard decision tapes. |
| CLI | `demo`, `record`, `replay`, `view`, `verify`, `fork`, `diff`, `bisect`, `guard`, `test`, and `up`. |
| Viewer | Local browser viewer with structural validation, lineage metadata, filtering, event details, and explicit cryptographic verification guidance. |
| SDKs | Python and TypeScript helpers aligned with the Rust BLAKE3 and Merkle semantics. |
| Specification | English v0.1 JSON Schema and conformance notes. |
| Quality | Rust workspace, locked dependencies, toolchain pinning, CI, smoke scripts, and English-only repository content. |

## Validation performed

The project passes `cargo fmt --all -- --check`, `cargo test --all`, and `cargo clippy --all-targets --all-features -- -D warnings`. The core suite covers stable hashing, round-trip serialization, signatures, bounded assertions, fork lineage, replay overrides, and nested redaction.

The proxy was tested with health checks, standard JSON completions, Server-Sent Events, request/response recording, tape verification, and offline replay. The CLI was tested with demo generation, replay overrides, lineage-preserving forks, structured diffs, guard annotation, and the tape regression command.

The TypeScript SDK builds successfully, the Python SDK writes a valid tape using the BLAKE3 dependency, the JSON Schema parses successfully, and JavaScript syntax checks pass for the viewer and npm wrapper.

## Quickstart

```bash
cargo test --all
cargo run -p causa -- demo
cargo run -p causa -- verify fixtures/demo-fail.causa
cargo run -p causa -- replay fixtures/demo-fail.causa --from 2
cargo run -p causa -- bisect fixtures/demo-fail.causa \
  --good fixtures/demo-good.causa \
  --assert 'final.status == "ok"'
```

To record OpenAI-compatible local traffic:

```bash
cargo run -p causa -- up --record run.causa
```

To serve the recorded response offline:

```bash
cargo run -p causa -- up --replay run.causa
```

## Deliberate v0.1 boundaries

The local preview does not claim full arbitrary-provider streaming capture, a complete MCP protocol mediator, maintained adapters for every agent framework, chunked compressed storage, a cloud control plane, or enterprise tenancy. Those items are documented in `ROADMAP.md` as future milestones rather than being presented as complete features.
