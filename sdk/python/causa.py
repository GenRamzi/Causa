"""Small explicit-span helper for creating Causa-compatible local tapes."""
from __future__ import annotations

import hashlib
import json
import platform
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


def _hash(value: Any) -> str:
    data = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    return hashlib.blake2s(data, digest_size=32).hexdigest()


@dataclass
class Tape:
    command: str | None = None
    events: list[dict[str, Any]] = field(default_factory=list)

    def event(self, kind: str, name: str, input: Any, output: Any, labels: list[str] | None = None) -> str:
        parents = [self.events[-1]["hash"]] if self.events else []
        step = len(self.events) + 1
        item = {"step": step, "kind": kind, "name": name, "input": input, "output": output,
                "labels": [{"namespace": x.split(":", 1)[0], "value": x.split(":", 1)[1]} for x in (labels or [])],
                "parents": parents}
        item["hash"] = _hash(item)
        self.events.append(item)
        return item["hash"]

    def write(self, path: str | Path) -> None:
        payload = {"format": "0.1", "metadata": {"run_id": f"run-{_hash(time.time_ns())[:12]}",
            "created_at": f"unix:{int(time.time())}", "command": self.command,
            "platform": platform.system().lower(), "mode": "sdk", "content_policy": "recorded-content"},
            "events": self.events, "merkle_root": _hash([e["hash"] for e in self.events]), "signature": None}
        Path(path).write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def start(command: str | None = None) -> Tape:
    return Tape(command=command)
