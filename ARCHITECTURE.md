# Architecture

Causa is intentionally split into a format/core layer, a local CLI, and local presentation/integration layers.

## Core

`causa-core` owns the event model, provenance labels, content hashes, Merkle verification, optional Ed25519 signatures, tape serialization, safe assertions, and redaction helpers. It has no network client and can be embedded by other local tools.

## CLI

`causa` provides the human and CI entrypoints. `demo` creates fixtures without credentials. `record` captures a local child process boundary. `replay` reads only the tape. `view`, `verify`, `fork`, `diff`, `bisect`, and `guard` operate on local artifacts. `up` exposes a small local OpenAI-compatible surface for health checks and development wiring.

## Viewer

The browser viewer is static and account-free. It reads a user-selected file with the File API, renders the event timeline, and displays inputs, outputs, labels, parents, and hashes. It does not contain an upload endpoint.

## Extension points

The Python and TypeScript SDKs expose explicit event recording. A future provider adapter can translate OpenAI-compatible requests, MCP calls, or framework spans into the same event model without changing the envelope. Cloud storage, CI indexing, and enterprise policy distribution remain outside the local v0.1 core.
