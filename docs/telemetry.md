# Telemetry policy

The Causa v0.1 local preview does not send telemetry, analytics, tape contents, prompts, tool results, or crash reports to a Causa service. The viewer reads selected files locally. The CLI's `up` command binds to loopback by default.

If a future optional integration adds diagnostics or cloud storage, it must be opt-in, documented, disabled in offline/replay mode, and must never transmit raw tape content without an explicit user action.
