import { hash as blake3 } from "blake3";
import { writeFileSync } from "node:fs";

export type Label = `${string}:${string}`;
export type EventKind = "user.message" | "model.request" | "model.response" | "tool.call" | "tool.result" | "note" | "process.exit";

type Json = Record<string, unknown> | unknown[] | string | number | boolean | null;

function canonical(value: Json): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(value, Object.keys(value as object).sort()));
}

function hex(bytes: Uint8Array): string {
  return [...bytes].map(byte => byte.toString(16).padStart(2, "0")).join("");
}

function digestBytes(input: Uint8Array): Uint8Array {
  const result = blake3(input) as unknown as Uint8Array | string;
  return typeof result === "string" ? new TextEncoder().encode(result) : new Uint8Array(result);
}

function digest(value: Json): string {
  return hex(digestBytes(canonical(value)));
}

function merkleRoot(eventHashes: string[]): string {
  if (eventHashes.length === 0) return hex(digestBytes(new TextEncoder().encode("causa:empty")));
  let level: Uint8Array[] = eventHashes.map(item => digestBytes(new TextEncoder().encode(item)));
  while (level.length > 1) {
    const next: Uint8Array[] = [];
    for (let index = 0; index < level.length; index += 2) {
      const right = level[index + 1] || level[index];
      const pair = new Uint8Array(level[index].length + right.length);
      pair.set(level[index]);
      pair.set(right, level[index].length);
      next.push(digestBytes(pair));
    }
    level = next;
  }
  return hex(level[0]);
}

export class Tape {
  private events: any[] = [];
  constructor(private command?: string) {}

  event(kind: EventKind, name: string, input: Json, output: Json, labels: Label[] = []): string {
    const item: any = {
      step: this.events.length + 1,
      kind,
      name,
      input,
      output,
      labels: labels.map(label => { const [namespace, value] = label.split(":", 2); return { namespace, value }; }),
      parents: this.events.length ? [this.events[this.events.length - 1].hash] : [],
    };
    item.hash = digest(item);
    this.events.push(item);
    return item.hash;
  }

  write(file: string): void {
    const tape = {
      format: "0.1",
      metadata: {
        run_id: `run-${digest(Date.now()).slice(0, 12)}`,
        created_at: `unix:${Math.floor(Date.now() / 1000)}`,
        command: this.command,
        platform: process.platform,
        mode: "sdk",
        content_policy: "recorded-content",
        source_run_id: null,
        fork_at: null,
      },
      events: this.events,
      merkle_root: merkleRoot(this.events.map(event => event.hash)),
      signature: null,
    };
    writeFileSync(file, `${JSON.stringify(tape, null, 2)}\n`, "utf8");
  }
}

export const start = (command?: string) => new Tape(command);
