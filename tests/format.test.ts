import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { formatMessage } from "../src/lib/format.ts";

const appSource = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../src/App.svelte"),
  "utf8",
);

test("markdown sample yields heading, list, fenced code, inline code, emphasis, and link", () => {
  const raw = [
    "# Title line",
    "",
    "A paragraph with *italic* and **bold** and `inline()`.",
    "",
    "- first bullet",
    "- second bullet",
    "",
    "1. alpha",
    "2. beta",
    "",
    "See [docs](https://example.com/path).",
    "",
    "```js",
    "const x = 1;",
    "```",
  ].join("\n");

  const html = formatMessage(raw);

  assert.match(html, /<h1\b/i, "heading structure");
  assert.match(html, /Title line/, "heading text");
  assert.match(html, /<ul class="md-ul">/, "unordered list class the LCD styles");
  assert.match(html, /<ol class="md-ol">/, "ordered list class the LCD styles");
  assert.match(html, /<li\b/i, "list item");
  assert.match(html, /first bullet/);
  assert.match(html, /<pre\b/i, "fenced code block");
  assert.match(html, /<code\b[^>]*>[\s\S]*const x = 1;/i, "fenced code body");
  assert.match(html, /<code class="md-code">inline\(\)<\/code>/, "inline code");
  assert.match(html, /<em>italic<\/em>/, "emphasis");
  assert.match(html, /<strong>bold<\/strong>/, "strong");
  assert.match(html, /<a\b[^>]*href="https:\/\/example.com\/path"/i, "link href");
  assert.match(html, />docs<\/a>/, "link label");
});

test("plain text with newlines keeps line breaks and has no markdown tags", () => {
  const raw = "line one\nline two\nline three";
  const html = formatMessage(raw);
  assert.match(html, /line one/);
  assert.match(html, /line two/);
  assert.match(html, /<br>/, "newlines preserved as breaks");
  assert.doesNotMatch(html, /<h[1-6]\b/i);
  assert.doesNotMatch(html, /<ul\b/i);
});

test("empty and whitespace-only bodies are empty strings", () => {
  assert.equal(formatMessage(""), "");
  assert.equal(formatMessage("   \n\t  "), "");
});

test("LCD styles restore disc/decimal list markers against Tailwind preflight", () => {
  assert.match(appSource, /\.md-ul[\s\S]{0,80}list-style:\s*disc/, "unordered lists show discs");
  assert.match(appSource, /\.md-ol[\s\S]{0,80}list-style:\s*decimal/, "ordered lists show decimals");
  assert.match(appSource, /::marker[\s\S]{0,120}color:\s*var\(--lcd-ink\)/, "list markers use LCD ink");
});

test("HTML and script payloads stay inert", () => {
  const raw = 'click <script>alert(1)</script> and <img onerror="alert(2)" src=x>';
  const html = formatMessage(raw);
  assert.doesNotMatch(html, /<script\b/i, "script tag must not survive");
  assert.doesNotMatch(html, /<img\b/i, "img tag must not survive");
  assert.match(html, /&lt;script/);
  assert.match(html, /&lt;img/);
  assert.match(html, /onerror=&quot;/, "attribute text is escaped, not a live handler");
});
