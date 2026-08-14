"""Explicit event recording helpers for Causa v0.1 tapes."""
from __future__ import annotations

import json
import platform
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import blake3


def _canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def _hash_value(value: Any) -> str:
    return blake3.blake3(_canonical(value)).hexdigest()


def _merkle_root(event_hashes: list[str]) -> str:
    if not event_hashes:
        return blake3.blake3(b"causa:empty").hexdigest()
    level = [blake3.blake3(item.encode("ascii")).digest() for item in event_hashes]
    while len(level) > 1:
        next_level: list[bytes] = []
        for index in range(0, len(level), 2):
            pair = level[index] + level[index + 1 if index + 1 < len(level) else index]
            next_level.append(blake3.blake3(pair).digest())
        level = next_level
    return level[0].hex()


@dataclass
class Tape:
    command: str | None = None
    events: list[dict[str, Any]] = field(default_factory=list)

    def event(self, kind: str, name: str, input: Any, output: Any, labels: list[str] | None = None) -> str:
        parents = [self.events[-1]["hash"]] if self.events else []
        step = len(self.events) + 1
        item: dict[str, Any] = {
            "step": step,
            "kind": kind,
            "name": name,
            "input": input,
            "output": output,
            "labels": [{"namespace": value.split(":", 1)[0], "value": value.split(":", 1)[1]} for value in (labels or [])],
            "parents": parents,
        }
        item["hash"] = _hash_value(item)
        self.events.append(item)
        return item["hash"]

    def write(self, path: str | Path) -> None:
        payload = {
            "format": "0.1",
            "metadata": {
                "run_id": f"run-{_hash_value(time.time_ns())[:12]}",
                "created_at": f"unix:{int(time.time())}",
                "command": self.command,
                "platform": platform.system().lower(),
                "mode": "sdk",
                "content_policy": "recorded-content",
                "source_run_id": None,
                "fork_at": None,
            },
            "events": self.events,
            "merkle_root": _merkle_root([event["hash"] for event in self.events]),
            "signature": None,
        }
        Path(path).write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def start(command: str | None = None) -> Tape:
    return Tape(command=command)
