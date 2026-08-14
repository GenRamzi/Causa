# Format Conformance

The v0.1 conformance boundary is defined by three checks:

1. The envelope declares `format: "0.1"` and contains the required metadata, event, Merkle, and signature fields.
2. Every event hash matches its canonical content and the Merkle root matches the ordered event hashes.
3. When a signature is present, the Ed25519 public key verifies the unsigned envelope bytes.

The Rust implementation exposes these checks through `Tape::read`, `Tape::verify_integrity`, and the `causa verify` command. The JSON Schema describes shape and types; semantic hash verification remains an application-level requirement.
