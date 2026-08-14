# Causa

> **The black box for AI agents. Record everything. Replay anything. Prove why.**

Causa is a local, open-source runtime for recording agent executions as portable, content-addressed `.causa` tapes. The current v0.1 preview provides a deterministic event model, BLAKE3 content hashes, Merkle integrity verification, optional Ed25519 signatures, provenance labels, an offline replay view, fork/diff/bisect workflows, guard-policy evaluation, and a local OpenAI-compatible proxy health surface.

## Why Causa exists

Agent logs tell you what happened, but usually not which input caused the outcome. Causa stores an execution as an ordered causal graph. Each event contains its inputs, output, labels, parent hashes, and content hash. A tape can be copied to a bug report, inspected without an account, and verified without contacting a service.

> **Replay is capture-based.** Causa replays recorded model/tool/process results; it does not claim that an external provider is deterministic.

## Quickstart

The repository is a Cargo workspace. Install Rust 1.75+ or the current stable toolchain, then run:

```bash
cargo test --all
cargo run -p causa -- demo
cargo run -p causa -- verify fixtures/demo-fail.causa
cargo run -p causa -- replay fixtures/demo-fail.causa --from 2
cargo run -p causa -- bisect fixtures/demo-fail.causa \
  --good fixtures/demo-good.causa \
  --assert 'final.status == "ok"'
```

The demo creates a good and a failing tape. `bisect` identifies the first divergent causal node: the search result whose currency column changed order.

To run the local OpenAI-compatible proxy with recording:

```bash
cargo run -p causa -- up --record run.causa
curl http://127.0.0.1:7777/v1/chat/completions \\
  -H 'content-type: application/json' \\
  -d '{"model":"causa-replay","messages":[{"role":"user","content":"Hello"}]}'
causa verify run.causa
```

The proxy supports standard JSON completions and Server-Sent Events when `stream: true`. To serve a recorded response offline, run `causa up --replay run.causa`.

To record an ordinary local command:

```bash
cargo run -p causa -- record -o run.causa -- sh -c 'printf hello'
cargo run -p causa -- view run.causa
cargo run -p causa -- verify run.causa
```

## Command reference

| Command | Purpose |
|---|---|
| `demo` | Create a reproducible good/bad pair without an API key. |
| `record -- <command>` | Capture a local process start and exit into a tape. |
| `replay <tape>` | Inspect the recorded timeline offline, optionally from a step; use `--set step:N@fixture.json -o derived.causa` for a safe output override. |
| `view <tape>` | Print the causal timeline and provenance labels; use `--json` for machine output. |
| `verify <tape>` | Validate format version, event hashes, Merkle root, and signatures. |
| `fork <tape> --at N` | Create an alternate timeline up to a selected step while preserving source lineage metadata. |
| `diff <left> <right>` | Compare causal nodes and report the first divergence. |
| `bisect <bad> --good <good>` | Isolate the first divergent node when an assertion flips. |
| `guard <tape> [--policy policy.txt]` | Evaluate provenance-aware rules; use `--audit` for non-blocking review and `-o decisions.causa` to save decision events. |
| `test [directory]` | Verify every `.causa` tape in a directory as a local regression suite. |
| `up` | Start a local OpenAI-compatible proxy on `127.0.0.1:7777`; use `--record tape.causa` to capture requests and responses or `--replay tape.causa` to serve recorded responses. |

## Tape format

A `.causa` file is a JSON envelope with a stable v0.1 format marker, run metadata, event nodes, a Merkle root, and an optional Ed25519 signature. Event addresses are BLAKE3 hashes of canonical event content. Labels use a `namespace:value` form such as `user:trusted`, `web:untrusted`, `db:pii`, and `tool:filesystem`.

The format and design notes live in [`docs/spec/causa-format-v0.1.md`](docs/spec/causa-format-v0.1.md) and [`docs/spec/rfc-0001-causal-tape.md`](docs/spec/rfc-0001-causal-tape.md).

## Local viewer

Open [`viewer/index.html`](viewer/index.html) in a browser, drop a `.causa` file onto it, and inspect the timeline locally. The viewer uses the browser File API and does not upload the tape. It supports timeline filtering, event details, provenance labels, and integrity status.

## Architecture

```text
causa-core
  event model · labels · BLAKE3 hashes · Merkle root · signatures · tape I/O
       │
causa CLI
  demo · record · replay · view · verify · fork · diff · bisect · guard · up
       │
viewer/ + sdk/
  account-free local viewer · Python event builder · TypeScript event builder
```

Replay overrides use a documented, bounded syntax rather than executing arbitrary expressions:

```bash
cargo run -p causa -- replay run.causa \\
  --set step:2@fixtures/alternate-result.json \\
  --output forked-replay.causa
```

The implementation intentionally keeps Cloud, Enterprise storage, and framework-specific adapters out of the v0.1 local core. Their interfaces are documented as follow-on work in [`ROADMAP.md`](ROADMAP.md).

## Security and privacy

Causa is local-first and has no telemetry in this repository. Do not record secrets or personal data without an explicit retention policy. Use the hash-only/redaction hooks in `causa-core` before publishing tapes. A tape signature proves integrity and authorship of the signed envelope; it does not encrypt content.

See [`SECURITY.md`](SECURITY.md) for reporting and threat-model notes.

## License

The core and CLI are Apache-2.0. The format specification is CC BY 4.0. Example museum tapes are intended to be redacted and CC0.
