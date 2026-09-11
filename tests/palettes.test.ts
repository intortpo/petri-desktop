import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_PACK_ID, PACKS, hexRgb, packById } from "../src/lib/palettes.ts";
import { compilePersonaBrief, isSysmin } from "../src/lib/persona.ts";
import { cosine, computeEmbedding } from "../src/lib/reasoningbank.ts";
import { defaultOpen, toggleWin } from "../src/lib/windows.ts";

test("Field Signal is the default pack", () => {
  const p = packById(DEFAULT_PACK_ID);
  assert.equal(p.id, "field-signal");
  assert.equal(p.ice, "#c0cdce");
  assert.equal(p.void, "#3c4c85");
  assert.ok(PACKS.length >= 17);
});

test("Field Signal maps the five sampled stripes and keeps lcd pale", () => {
  const p = packById("field-signal");
  assert.equal(p.record, "#964444");
  assert.equal(p.slate, "#789044");
  assert.equal(p.ash, "#dab71f");
  assert.equal(p.void, "#3c4c85");
  assert.equal(p.ice, "#c0cdce");
  assert.equal(p.lcdInk, "#c0cdce");
  assert.equal(p.hot, "record");
});

test("hexRgb parses 6-digit hex", () => {
  const [r, g, b] = hexRgb("#ffffff");
  assert.equal(r, 1);
  assert.equal(g, 1);
  assert.equal(b, 1);
});

test("sysmin is hideo or intortpo@gmail.com", () => {
  assert.equal(isSysmin("hideo", ""), true);
  assert.equal(isSysmin("x", "intortpo@gmail.com"), true);
  assert.equal(isSysmin("ada", "ada@ex.com"), false);
});

test("persona brief is capped", () => {
  const rows = Array.from({ length: 40 }, (_, i) => ({
    role: "user",
    content: "word ".repeat(80) + String(i),
  }));
  const brief = compilePersonaBrief(rows, 200);
  assert.ok(brief.length <= 200);
});

test("window toggle never drops the last pane", () => {
  const next = toggleWin(["chat"], "chat");
  assert.deepEqual(next, ["chat"]);
  assert.ok(defaultOpen(true).includes("chat"));
});

test("embeddings cosine is 1 for identical text", () => {
  const a = computeEmbedding("hive persona");
  assert.ok(cosine(a, a) > 0.99);
});
