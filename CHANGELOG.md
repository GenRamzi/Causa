# Changelog

## 0.1.0-preview — 2026-08-14

### Added

- Rust workspace with `causa-core` and `causa` CLI.
- Portable `.causa` JSON envelope with event hashes, causal parents, provenance labels, Merkle verification, and optional Ed25519 signatures.
- Local `demo`, `record`, `replay`, `view`, `verify`, `fork`, `diff`, `bisect`, `guard`, and `up` commands.
- Deterministic good/bad demo fixtures and a local static viewer.
- Small explicit-event Python and TypeScript SDK helpers.
- CI workflow for formatting, tests, clippy, and Node wrapper smoke testing.

### Limitations

The preview does not yet capture arbitrary provider streaming traffic, implement MCP protocol mediation, provide framework-maintained adapters, compress chunk storage, or offer cloud/enterprise services. Those items remain on the roadmap and are intentionally not presented as complete in v0.1 documentation.
