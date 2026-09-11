import { test } from "node:test";
import assert from "node:assert/strict";
import {
  checkEverosHealth,
  addEverosTurn,
  rememberEveros,
  flushEveros,
  searchEveros,
  getEverosProfile,
  formatEverosRecall,
  type EverosMemoryHit,
} from "../src/lib/everos.ts";

test("everos client reports health in fallback mode", async () => {
  const health = await checkEverosHealth();
  assert.equal(health.status, "ok");
  assert.ok(health.capabilities.markdown_native);
  assert.ok(health.storage_root.includes("everos"));
});

test("everos client remembers, adds turn, flushes, and searches", async () => {
  await rememberEveros("Use strict TypeScript 6 types", "typescript");
  await addEverosTurn("test_session_42", "user", "How do we write TypeScript 6?");
  await addEverosTurn("test_session_42", "assistant", "Use strict types and no any.");

  const flushRes = await flushEveros("test_session_42");
  assert.equal(flushRes.status, "ok");
  assert.equal(flushRes.flushed, true);

  const searchHits = await searchEveros("TypeScript 6");
  assert.ok(searchHits.length > 0);
  assert.ok(searchHits.some((h) => h.content.includes("TypeScript 6")));

  const profile = await getEverosProfile();
  assert.ok(profile.length > 0);
});

test("formatEverosRecall formats empty and populated lists", () => {
  assert.equal(formatEverosRecall([]), "No matching EverOS memories found.");

  const hits: EverosMemoryHit[] = [
    {
      id: "1",
      type: "fact",
      source_path: "/path/to/facts.md",
      session_id: null,
      content: "Markdown memory is canonical",
      created_at: Date.now(),
      similarity: 0.95,
    },
  ];
  const formatted = formatEverosRecall(hits);
  assert.ok(formatted.includes("[FACT]"));
  assert.ok(formatted.includes("Markdown memory is canonical"));
  assert.ok(formatted.includes("score: 0.95"));
});
