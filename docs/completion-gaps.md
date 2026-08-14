# v0.1 Completion Gap Review

The repository already has a buildable local preview with a Rust core, CLI, local viewer, SDK helpers, fixtures, and CI. The next completion pass focuses on making the advertised local workflow operational rather than demonstrative.

| Area | Current state | Completion target |
|---|---|---|
| Tape model | Stable JSON envelope, event hashes, Merkle root, optional signatures | Preserve lineage during forks, expose verified parent graphs, and add stronger malformed-input tests |
| Recording | Captures a child process boundary and final stdout/stderr | Add a recording OpenAI-compatible proxy path with request/response events and streaming support |
| Replay | Prints recorded events and accepts a display-only override | Apply safe JSON overrides and produce a derived tape with lineage metadata |
| Fork/diff | Commands exist but fork reconstructs nodes rather than preserving the source lineage | Preserve source hashes and report structured field-level differences |
| Bisect | Fixture-guided first divergence | Add deterministic probe accounting and ambiguity reporting |
| Guard | Text-pattern policy evaluator | Parse a small documented policy grammar and emit guard decision events |
| Viewer | Local timeline and detail view | Add graph-oriented parent display and verified/invalid states |
| SDKs | Minimal explicit event helpers | Align hashes and envelope semantics with the Rust core |
| Distribution | CI and npm wrapper | Add smoke tests and document what is local-only versus future work |
