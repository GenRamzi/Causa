import { start } from "@causa/sdk";

const tape = start("typescript-agent");
tape.event(
  "user.message",
  "user.prompt",
  { text: "Find the current price" },
  { accepted: true },
  ["user:trusted"],
);
tape.event(
  "tool.result",
  "search.results",
  { query: "current price" },
  { columns: ["item", "currency", "amount"], rows: [["widget", "USD", 10]] },
  ["web:untrusted", "tool:search"],
);
tape.event(
  "model.response",
  "agent.answer",
  { context_step: 2 },
  { status: "ok", answer: "The widget costs 10 USD." },
  ["web:untrusted"],
);
tape.write("typescript-agent.causa");
console.log("Wrote typescript-agent.causa");
