# Security Policy

## Scope

Causa processes agent inputs, tool results, model responses, and local process output. Treat tapes as potentially sensitive evidence. A Merkle root detects modification; an Ed25519 signature authenticates a signed envelope; neither property hides the payload.

## Safe defaults

Record only the fields required for debugging. Redact secrets and personal data before publishing fixtures. Keep tapes outside public repositories unless they have been reviewed. Replay is designed to be offline and should not be given write access to production workspaces.

## Threat model

The v0.1 CLI rejects invalid format and integrity data, limits assertions to a small comparison grammar, and evaluates guard policies without executing tape content. Future work must continue to treat tapes and policy files as untrusted input, defend against path traversal and resource exhaustion, and avoid shell interpolation.

## Reporting

Please report a reproducible security issue privately to the repository maintainers before opening a public issue. Include the Causa version, operating system, minimal redacted tape or fixture, and the command that reproduces the behavior.
