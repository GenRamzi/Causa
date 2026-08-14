# Causa Quick Start Guide

This guide shows two ways to use Causa from an application: explicit event recording with the Python or TypeScript SDK, and provider-compatible capture through the local OpenAI-compatible proxy.

> **Scope:** The v0.1 preview is local-first. Replay reads captured results from a tape and does not call an external model provider.

## 1. Install and run the deterministic demo

From the repository root:

```bash
cargo test --all
cargo run -p causa -- demo
cargo run -p causa -- verify fixtures/demo-fail.causa
cargo run -p causa -- replay fixtures/demo-fail.causa --from 2
cargo run -p causa -- bisect fixtures/demo-fail.causa \
  --good fixtures/demo-good.causa \
  --assert 'final.status == "ok"'
```

The demo creates a passing tape and a failing tape. `bisect` compares them and reports the first divergent causal node.

## 2. Python: record explicit events

The Python SDK is a small explicit-event helper. It is useful when an application knows the important boundaries it wants to record, such as user input, a tool call, a model response, and the final state.

Install the local SDK in an application environment:

```bash
python -m pip install -e sdk/python
```

Create `agent.py`:

```python
from causa import start


tape = start("python-agent")
tape.event(
    "user.message",
    "user.prompt",
    {"text": "Find the current price"},
    {"accepted": True},
    ["user:trusted"],
)
tape.event(
    "tool.result",
    "search.results",
    {"query": "current price"},
    {"columns": ["item", "currency", "amount"], "rows": [["widget", "USD", 10]]},
    ["web:untrusted", "tool:search"],
)
tape.event(
    "model.response",
    "agent.answer",
    {"context_step": 2},
    {"status": "ok", "answer": "The widget costs 10 USD."},
    ["web:untrusted"],
)
tape.write("python-agent.causa")
print("Wrote python-agent.causa")
```

Run and inspect the result:

```bash
python agent.py
cargo run -p causa -- verify python-agent.causa
cargo run -p causa -- view python-agent.causa
```

The SDK emits the same v0.1 event fields used by the Rust core: `step`, `kind`, `name`, `input`, `output`, `labels`, `parents`, and `hash`.

## 3. TypeScript: record explicit events

Install the local TypeScript SDK:

```bash
cd sdk/typescript
npm install
npm run build
cd ../..
```

Create `agent.ts` in a TypeScript application:

```typescript
import { start } from "@causa/sdk";

const tape = start("typescript-agent");
tape.event(
  "user.message",
  "user.prompt",
  { text: "Find the current price" },
  { accepted: true },
  ["user:trusted"],
);
tape.event(
  "tool.result",
  "search.results",
  { query: "current price" },
  { columns: ["item", "currency", "amount"], rows: [["widget", "USD", 10]] },
  ["web:untrusted", "tool:search"],
);
tape.event(
  "model.response",
  "agent.answer",
  { context_step: 2 },
  { status: "ok", answer: "The widget costs 10 USD." },
  ["web:untrusted"],
);
tape.write("typescript-agent.causa");
console.log("Wrote typescript-agent.causa");
```

Compile and inspect the tape:

```bash
npx tsc --target ES2020 --module NodeNext --moduleResolution NodeNext \
  --skipLibCheck agent.ts
node agent.js
cargo run -p causa -- verify typescript-agent.causa
```

## 4. Any OpenAI-compatible application: record through the proxy

The proxy lets an application keep its provider client and request shape. Start Causa with a recording destination:

```bash
cargo run -p causa -- up --record openai-run.causa
```

Point the client at the local endpoint:

```bash
export OPENAI_BASE_URL=http://127.0.0.1:7777
```

A raw HTTP request is enough for a smoke test:

```bash
curl http://127.0.0.1:7777/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"causa-replay","messages":[{"role":"user","content":"Hello"}]}'
```

The proxy also supports Server-Sent Events when the request includes `"stream": true`:

```bash
curl -N http://127.0.0.1:7777/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"causa-replay","stream":true,"messages":[{"role":"user","content":"Hello"}]}'
```

Verify the recorded tape:

```bash
cargo run -p causa -- verify openai-run.causa
cargo run -p causa -- view openai-run.causa
```

## 5. Replay a recorded response offline

Start the proxy with a recorded tape instead of a provider:

```bash
cargo run -p causa -- up --replay openai-run.causa
```

The same request path now returns the captured model response. This is the local replay boundary: no API key or outbound provider call is required.

For a targeted output change, use a JSON fixture and write a derived tape:

```bash
cargo run -p causa -- replay openai-run.causa \
  --set step:2@fixtures/alternate-result.json \
  --output replay-with-override.causa
```

## 6. Inspect, fork, diff, and guard

```bash
cargo run -p causa -- view openai-run.causa
cargo run -p causa -- fork openai-run.causa --at 2 --output alternate.causa
cargo run -p causa -- diff openai-run.causa alternate.causa
cargo run -p causa -- guard openai-run.causa --policy policies/default.txt --audit
cargo run -p causa -- test fixtures
```

Use `--audit` to report policy matches without stopping a workflow. Use `-o decisions.causa` to save guard decision events into a derived tape.

## 7. Open the tape in the local viewer

Open [`viewer/index.html`](../viewer/index.html) in a browser, then drop any `.causa` file onto the page. The viewer reads the file locally and shows the timeline, labels, parent hashes, lineage metadata, and event payloads.

The viewer performs structural validation in the browser. Use `causa verify` for authoritative BLAKE3, Merkle, and signature verification.

## 8. Common troubleshooting

| Symptom | Resolution |
|---|---|
| `Address already in use` | Stop the existing local proxy or choose another port with `--port 7788`. |
| `unsupported tape format version` | Use a reader compatible with the tape’s declared format or migrate the tape explicitly. |
| `replay tape has no model.response event` | Record a provider response first or use an explicit SDK tape with a model response event. |
| Guard reports a malformed policy line | Follow the documented `deny: target when condition` syntax in `policies/default.txt`. |
| The viewer shows an invalid shape | Run `causa verify file.causa` to identify the authoritative integrity error. |

## Next steps

Read [`ARCHITECTURE.md`](../ARCHITECTURE.md) for the component model, [`docs/spec/causa-format-v0.1.md`](spec/causa-format-v0.1.md) for the envelope contract, and [`ROADMAP.md`](../ROADMAP.md) for the next capture and integration milestones.
