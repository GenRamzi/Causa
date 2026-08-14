# Causa Format v0.1

## Status

This document defines the local preview envelope produced by `causa-core` 0.1. It is an implementation contract for this repository, not a claim that the format is already governed by an external standards body.

## Envelope

A `.causa` file is a UTF-8 JSON object with the following required fields:

| Field | Type | Meaning |
|---|---|---|
| `format` | string | Currently `"0.1"`. Readers must reject unsupported major versions. |
| `metadata` | object | Run identity, timestamp, command, platform, mode, and content policy. |
| `events` | array | Ordered causal event nodes. |
| `merkle_root` | string | Hex-encoded BLAKE3 root over event hashes. |
| `signature` | object/null | Optional Ed25519 public key and signature over the unsigned envelope. |

Each event contains `step`, `kind`, `name`, `input`, `output`, `labels`, `parents`, and `hash`. The hash covers the canonical event content excluding its own `hash` field. Parent values are event hashes, so a reader can reconstruct the recorded causal chain without trusting step numbers alone.

## Labels

Labels are objects with `namespace` and `value` and are displayed as `namespace:value`. The preview includes labels such as `user:trusted`, `web:untrusted`, `db:pii`, and `tool:search`. Labels are data; a policy engine decides whether a label is allowed to reach a sink.

## Integrity

Readers validate every event hash and then recompute the Merkle root. A signature, when present, authenticates the unsigned envelope using Ed25519. Signatures do not encrypt payloads. Applications that handle secrets must redact or hash content before recording.

## Compatibility

Patch releases may add optional fields. Readers should ignore unknown optional fields and preserve required fields when rewriting. A reader must fail closed on an unknown format version or invalid integrity data. Cross-version fixtures belong under `conformance/`.
