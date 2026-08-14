# Causa — Project Architecture and Workflow

## Cover
Causa
The black box for AI agents
Project architecture from causal events to replay and verification
[Note: This is a text-only cover page. English text with Arabic explanation: "الصندوق الأسود للوكلاء الذكيين: هيكل المشروع من الحدث السببي إلى إعادة التشغيل والتحقق."]

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
[Note: Arabic explanation included in the slide: "السجلات التقليدية تخبرك بما حدث، لكنها لا توضح السبب. Causa يحول التنفيذ إلى شريط سببي قابل للنقل وإعادة التشغيل محلياً."]

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
[Note: Arabic explanation included: "معمارية المشروع مقسمة لطبقات: النواة تحفظ البيانات، أدوات سطر الأوامر تشغلها، والعارض يشرحها للمستخدم."]

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
[Note: Arabic explanation included: "الحدث السببي هو الوحدة الأساسية. السجل لم يعد مجرد نص، بل أصبح رسماً بيانياً يربط النتائج بمدخلاتها بدقة."]

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
[Note: Arabic explanation included: "كل ملف .causa قابل للتحقق تشفيرياً عبر BLAKE3 وشجرة Merkle وتوقيع Ed25519 لضمان سلامة التنفيذ."]

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
[Note: Arabic explanation included: "الوسوم تنتقل مع البيانات. وجود النص في السياق لا يجعله موثوقاً؛ قواعد الحماية تستخدم هذه الوسوم لمنع الأفعال الخطرة."]

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
[Note: Arabic explanation included: "التسجيل يحول التنفيذ إلى شريط. إعادة التشغيل تقرأ من الشريط محلياً بلا إنترنت، فالحتمية تأتي من التسجيل وليس من مزود الخدمة."]

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
[Note: Arabic explanation included: "أدوات سطر الأوامر تحول الشريط إلى آلة زمن: يمكنك تسجيل، إعادة تشغيل، تفريع، ومقارنة الأشرطة بسهولة."]

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
[Note: Arabic explanation included: "أداة bisect تعزل السبب الجذري عبر مقارنة شريط فاشل بآخر ناجح لتحديد نقطة الانحراف الأولى بدقة."]

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
[Note: Arabic explanation included: "قواعد الحماية تطبق عملياً بناءً على المصدر، مع دعم وضع المنع الفعلي أو وضع التدقيق فقط."]

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
[Note: Arabic explanation included: "مسار عمل المطور مصمم ليكون بسيطاً ومباشراً، من التجربة إلى التحقق وعزل الأخطاء."]

## Slide 11
### What works today — and what comes next
**Implemented in the v0.1 preview:**

The core format, hashes and Merkle verification, signatures, demo fixtures, the core CLI, replay/view/verify, fork/diff/bisect, an initial guard evaluator, a local viewer, SDK helpers, and CI.

**Next milestones:**

Full OpenAI streaming capture, MCP mediation, maintained framework adapters, chunked compressed storage, larger fuzz and benchmark suites, and optional CI/cloud integrations.

> A feature is not complete because a CLI command exists; it must connect the interface, fixture, test, and documentation.
[Note: Arabic explanation included: "النسخة الحالية v0.1 توفر الأساس المتين، بينما الإصدارات القادمة ستركز على التكامل الشامل مع أطر العمل والسحابة."]

## Slide 12
### Causa turns execution into evidence
Causa is not another monitoring dashboard.

It turns an agent run into a **verifiable causal artifact**:

**Record everything · Replay anything · Prove why**

Sources: `README.md` · `ARCHITECTURE.md` · `docs/spec/causa-format-v0.1.md` · `ROADMAP.md`
[Note: Arabic explanation included: "الخلاصة: Causa ليس مجرد لوحة مراقبة، بل أداة تحول التنفيذ إلى دليل قاطع: سجل كل شيء، أعد تشغيل أي شيء، وأثبت السبب."]
