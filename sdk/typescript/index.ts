import { createHash } from "node:crypto";
import { writeFileSync } from "node:fs";

export type Label = `${string}:${string}`;
export type EventKind = "user.message" | "model.request" | "model.response" | "tool.call" | "tool.result" | "note" | "process.exit";

function hash(value: unknown): string {
  return createHash("blake2s256").update(JSON.stringify(value, Object.keys(value as object).sort())).digest("hex");
}

export class Tape {
  private events: any[] = [];
  constructor(private command?: string) {}

  event(kind: EventKind, name: string, input: unknown, output: unknown, labels: Label[] = []): string {
    const item: any = { step: this.events.length + 1, kind, name, input, output,
      labels: labels.map(label => { const [namespace, value] = label.split(":", 2); return { namespace, value }; }),
      parents: this.events.length ? [this.events[this.events.length - 1].hash] : [] };
    item.hash = hash(item);
    this.events.push(item);
    return item.hash;
  }

  write(file: string): void {
    const tape = { format: "0.1", metadata: { run_id: `run-${hash(Date.now()).slice(0, 12)}`,
      created_at: `unix:${Math.floor(Date.now() / 1000)}`, command: this.command,
      platform: process.platform, mode: "sdk", content_policy: "recorded-content" },
      events: this.events, merkle_root: hash(this.events.map(event => event.hash)), signature: null };
    writeFileSync(file, `${JSON.stringify(tape, null, 2)}\n`, "utf8");
  }
}

export const start = (command?: string) => new Tape(command);
