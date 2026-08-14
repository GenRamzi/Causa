# Causa — Project Architecture and Workflow

## Cover
Causa
The black box for AI agents
Project architecture from causal events to replay and verification

## Slide 1
### Why Causa exists
Traditional agent logs answer **what happened**, but rarely explain **which input caused the outcome**.

| Failure mode | Practical impact |
|---|---|
| Model and tool non-determinism | Failures are difficult to reproduce |
| No causal chain | The outcome is visible, but the divergence point is not |
| Trusted and untrusted data are mixed | Malicious instructions can reach dangerous tools |
| Full reruns after every change | Debugging becomes slow and expensive |

**Causa’s idea:** turn an agent execution into a portable causal tape that can be inspected and replayed locally.

## Slide 2
### The architecture is intentionally layered
```text
┌──────────────────────────────────────────────┐
│  Local Viewer — open .causa without an account│
├──────────────────────────────────────────────┤
│  CLI — demo · record · replay · verify       │
│         fork · diff · bisect · guard · up    │
├──────────────────────────────────────────────┤
│  causa-core — format · graph · labels        │
│  hashes · Merkle · signatures · tape I/O     │
├──────────────────────────────────────────────┤
│  Integration — Python SDK · TypeScript SDK   │
│  OpenAI-compatible proxy · future adapters   │
└──────────────────────────────────────────────┘
```

Each layer has one job: **the core preserves truth, the CLI operates on it, and the viewer explains it.**

## Slide 3
### The causal event is the core primitive
Every agent step becomes an independent `Event`.

| Field | Role |
|---|---|
| `step` and `kind` | Sequence and type: `model.response`, `tool.call`, `fs.read`… |
| `input` and `output` | What entered and left the step |
| `parents` | Hashes of the events that caused this step |
| `labels` | Data source and trust context such as `web:untrusted` |
| `hash` | Content address used for integrity and deduplication |

**Result:** the log is no longer a text list; it is a **causal dependency graph** that can be traced from outcome back to inputs.

## Slide 4
### The `.causa` file makes execution verifiable
```text
Event 1 ──hash──▶ Event 2 ──hash──▶ Event 3 ──hash──▶ Event 4
   └────────────── Merkle tree over event hashes ──────────────┘
                              │
                        merkle_root
                              │
                    optional Ed25519 signature
```

1. BLAKE3 is computed from the canonical event content, excluding the `hash` field itself.
2. Event hashes form a Merkle tree and produce the `merkle_root`.
3. `causa verify` checks every event and then the root.
4. The envelope can optionally be signed with Ed25519. **A signature proves origin; it does not encrypt content.**

## Slide 5
### Provenance labels travel with the data
Source labels are attached to events and inherited by downstream events.

```text
web:untrusted ──▶ tool.search.result ──▶ model.response ──▶ tool.email.send
       │                                                        │
       └──────────── guard policy: DENY external sink ─────────┘
```

**The key distinction:** text appearing in context does not make it trusted. A policy engine uses labels and explicit rules to decide whether a sink is allowed.

Examples:

`user:trusted` · `web:untrusted` · `db:pii` · `tool:filesystem` · `human:approved`

## Slide 6
### Recording turns execution into a replayable tape
```text
Agent process
    │  record
    ▼
TapeBuilder → Event nodes → BLAKE3/Merkle → run.causa
                                          │
                                          ├─ view / verify
                                          ├─ replay offline
                                          └─ fork / diff / bisect
```

During `record`, Causa captures process boundaries and integration events where available. During `replay`, recorded results are read from the tape; the network and model provider are not called.

**Determinism comes from capture, not from assuming that `temperature=0` makes a provider deterministic.**

## Slide 7
### CLI tools turn the tape into a time machine
| Command | What it does |
|---|---|
| `record` | Captures a local command into a `.causa` tape |
| `replay --from N` | Starts from a selected step, offline |
| `fork --at N` | Creates an alternate timeline up to a step |
| `diff` | Compares two timelines and finds the first divergence |
| `verify` | Checks hashes, Merkle integrity, and signatures |

The power is not one command. It is that **every command operates on the same primitive: the causal tape.**

## Slide 8
### `bisect` isolates the root cause
Demo scenario:

```text
Good: step 2 → columns = [item, currency, amount]
Bad:  step 2 → columns = [item, amount, currency]
                         ▲
                    first divergence
```

`causa bisect` compares a failing tape with a passing tape under a safe assertion:

```bash
causa bisect fail.causa --good pass.causa \
  --assert 'final.status == "ok"'
```

Useful output includes the step, divergent node hash, blast radius, and minimal reproduction command. The tool should not claim a unique root cause when the evidence is ambiguous.

## Slide 9
### Guard policies make provenance operational
```yaml
policies:
  - deny: tool.email.send
    when: args tainted_by(source.web)
  - deny: tool.http.post
    when: context has label(pii)
```

`causa guard` supports two operating modes:

| Mode | Behavior |
|---|---|
| `enforce` | Blocks the event and returns a machine-readable failure |
| `--audit` | Reports the potential violation without stopping the run |

The viewer is local through the browser File API. `causa up` binds to `127.0.0.1` by default, and the v0.1 preview ships without telemetry.

## Slide 10
### The developer workflow is intentionally small
```bash
cargo run -p causa -- demo
cargo run -p causa -- verify fixtures/demo-fail.causa
cargo run -p causa -- replay fixtures/demo-fail.causa --from 2
cargo run -p causa -- bisect fixtures/demo-fail.causa \
  --good fixtures/demo-good.causa \
  --assert 'final.status == "ok"'
```

| Surface | Responsibility |
|---|---|
| `causa-core` | Data format, graph, labels, and integrity |
| Rust CLI | Operations and CI automation |
| `viewer/` | Local visual investigation |
| `sdk/python` and `sdk/typescript` | Explicit event recording from applications |

## Slide 11
### What works today — and what comes next
**Implemented in the v0.1 preview:**

The core format, hashes and Merkle verification, signatures, demo fixtures, the core CLI, replay/view/verify, fork/diff/bisect, an initial guard evaluator, a local viewer, SDK helpers, and CI.

**Next milestones:**

Full OpenAI streaming capture, MCP mediation, maintained framework adapters, chunked compressed storage, larger fuzz and benchmark suites, and optional CI/cloud integrations.

> A feature is not complete because a CLI command exists; it must connect the interface, fixture, test, and documentation.

## Slide 12
### Causa turns execution into evidence
Causa is not another monitoring dashboard.

It turns an agent run into a **verifiable causal artifact**:

**Record everything · Replay anything · Prove why**

Sources: `README.md` · `ARCHITECTURE.md` · `docs/spec/causa-format-v0.1.md` · `ROADMAP.md`
