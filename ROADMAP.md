# Roadmap

## v0.1 Preview — implemented locally

The repository now includes the portable tape model, content-addressed event hashes, Merkle verification, optional signatures, a Rust CLI, deterministic demo fixtures, local process recording, offline replay inspection, fork/diff/bisect workflows, a provenance-aware guard evaluator, a local viewer, and small Python/TypeScript SDK helpers.

## Next milestones

| Milestone | Scope |
|---|---|
| v0.2 capture | Full OpenAI-compatible recording proxy with streaming capture, request/response redaction, and replay cache. |
| v0.2 tools | MCP capture, schema-aware tool events, and stronger policy/declassifier semantics. |
| v0.3 integrations | Maintained adapters for LangGraph, CrewAI, Agents SDK, and Vercel AI with version matrices. |
| v0.3 scale | Compressed chunk storage, lazy viewer parsing, fuzzing, and large-tape benchmarks. |
| Later | OpenTelemetry export, CI incremental test runner, and optional team/cloud storage. |

Cloud and Enterprise services are deliberately not part of this local repository's v0.1 acceptance target.
