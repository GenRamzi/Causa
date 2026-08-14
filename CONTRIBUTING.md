# Contributing

Causa is built in small, testable layers. Start with a reproducible issue or a focused feature proposal. For core changes, add or update a fixture and a unit or integration test. For CLI changes, update `--help`, README examples, and a smoke test. For format changes, update the specification and compatibility tests together.

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
```

Do not commit API keys, private tapes, generated `target/` files, or personal data. Keep commits focused and explain compatibility implications in the pull request description.
