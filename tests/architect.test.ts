import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  ADR_DESTINATION,
  CONTEXT_DESTINATION,
  MODEL_CHOICES,
  VALID_MODES,
  architectAllowsShell,
  architectAllowsTool,
  architectIsReadOnly,
  buildAdr,
  buildDomainDictionary,
  buildGrill,
  buildPlan,
  classifyArchitectAction,
  isValidMode,
  recognizeRoute,
  runDesignCommand,
} from "../src/lib/architect.ts";

const root = dirname(fileURLToPath(import.meta.url));
const appSource = readFileSync(join(root, "../src/App.svelte"), "utf8");
const winSource = readFileSync(join(root, "../src/lib/windows.ts"), "utf8");

test("architect is a valid selectable mode and header chrome is labeled", () => {
  assert.equal(isValidMode("architect"), true);
  assert.ok(VALID_MODES.includes("architect"));
  assert.ok(MODEL_CHOICES.some((m) => m.includes("Gemini 3.1 Pro")));
  assert.match(appSource, /<em>proj<\/em>/);
  assert.match(appSource, /<em>mode<\/em>/);
  assert.match(appSource, /<em>model<\/em>/);
  assert.match(appSource, /['"]OK['"]/);
  assert.match(appSource, />MESH</);
  assert.match(appSource, /['"]NITE['"]/);
  assert.match(appSource, /VALID_MODES/);
  assert.match(appSource, /MODEL_CHOICES/);
  assert.match(appSource, /selectMode/);
  assert.match(appSource, /selectModel/);
});

test("view chrome titles include Chat, GitHub repos, Users, Scheduled tasks", () => {
  assert.match(winSource, /chat:\s*"Chat"/);
  assert.match(winSource, /github:\s*"GitHub repos"/);
  assert.match(winSource, /users:\s*"Users"/);
  assert.match(winSource, /schedule:\s*"Scheduled tasks"/);
  assert.match(appSource, /GitHub repos/);
  assert.match(appSource, />Users</);
  assert.match(appSource, /Scheduled tasks/);
  assert.match(appSource, />Home</);
  assert.match(appSource, /class="send">SEND</);
  assert.match(appSource, /\/fork/);
  assert.match(appSource, /\/task/);
  assert.match(appSource, /\/theme/);
  assert.match(appSource, /from ['"]\.\/lib\/architect['"]/);
  assert.match(appSource, /runDesignCommand\("\/plan"/);
  assert.match(appSource, /runDesignCommand\("\/grill-me"/);
});

test("buildPlan includes component breakdown, verification criteria, and risk assessment", () => {
  const topic = "deepagents-app shell";
  const out = buildPlan({
    topic,
    components: ["chrome header", "window strip"],
    verification: ["header chips render"],
    risks: ["premature mutation"],
  });
  assert.match(out, /Component breakdown/i);
  assert.match(out, /Verification criteria/i);
  assert.match(out, /Risk assessment/i);
  assert.match(out, /chrome header/);
  assert.match(out, /header chips render/);
  assert.match(out, /premature mutation/);
  assert.match(out, new RegExp(topic));
  assert.match(out, /Codebase discovery/i);
  assert.match(out, /seam/i);
  const viaCmd = runDesignCommand("/plan", topic);
  assert.ok(viaCmd);
  assert.match(viaCmd, /Component breakdown/i);
  assert.match(viaCmd, new RegExp(topic));
});

test("ADR builder records trade-offs, boundaries, and choices at docs/adr/", () => {
  const title = "Keep architect read-only";
  const made = buildAdr({
    title,
    context: "Designers were mutating files during planning.",
    decision: "Author ADRs as structured artifacts.",
    tradeoffs: ["speed vs recoverable record"],
    boundaries: ["no general code writes in architect"],
    choices: ["transcript ADR", "silent skip"],
  });
  assert.equal(made.destination, ADR_DESTINATION);
  assert.equal(made.destination, "docs/adr/");
  assert.match(made.markdown, /Trade-offs/i);
  assert.match(made.markdown, /Boundaries/i);
  assert.match(made.markdown, /Decision/i);
  assert.match(made.markdown, /Choices/i);
  assert.match(made.markdown, /speed vs recoverable record/);
  assert.match(made.markdown, /no general code writes in architect/);
  assert.match(made.markdown, /transcript ADR/);
  assert.match(made.markdown, new RegExp(title));
  const viaCmd = runDesignCommand("/adr", title);
  assert.ok(viaCmd);
  assert.match(viaCmd, /docs\/adr\//);
  assert.match(viaCmd, /Trade-offs/i);
});

test("domain dictionary maps terms to definitions at CONTEXT.md", () => {
  const made = buildDomainDictionary({
    proj: "bound workspace",
    mesh: "execution cluster",
  });
  assert.equal(made.destination, CONTEXT_DESTINATION);
  assert.equal(made.destination, "CONTEXT.md");
  assert.match(made.markdown, /\| Term \| Definition \|/);
  assert.match(made.markdown, /\| proj \| bound workspace \|/);
  assert.match(made.markdown, /\| mesh \| execution cluster \|/);
  const viaCmd = runDesignCommand("/context", "mode: operating persona; nite: dark theme");
  assert.ok(viaCmd);
  assert.match(viaCmd, /CONTEXT\.md/);
  assert.match(viaCmd, /\| mode \| operating persona \|/);
  assert.match(viaCmd, /\| nite \| dark theme \|/);
});

test("grill-me challenges the supplied assumptions", () => {
  const topic = "scheduled tasks book";
  const claim = "the cron daemon is already running";
  const out = buildGrill({ topic, assumptions: [claim] });
  assert.match(out, /Challenge/i);
  assert.match(out, /assumption/i);
  assert.match(out, new RegExp(claim));
  assert.match(out, new RegExp(topic));
  assert.match(out, /falsif/i);
  const viaCmd = runDesignCommand("/grill-me", `${topic}; ${claim}`);
  assert.ok(viaCmd);
  assert.match(viaCmd, /Challenge/i);
  assert.match(viaCmd, new RegExp(claim));
});

test("architect mode is classified read-only and denies write or destructive shell", () => {
  assert.equal(architectIsReadOnly("architect"), true);
  assert.equal(architectIsReadOnly("code"), false);
  assert.equal(architectAllowsTool("architect", "read"), true);
  assert.equal(architectAllowsTool("architect", "write"), false);
  assert.equal(architectAllowsTool("architect", "edit"), false);
  assert.equal(architectAllowsTool("architect", "apply_patch"), false);
  assert.equal(architectAllowsShell("architect", "rm -rf src"), false);
  assert.equal(architectAllowsShell("code", "rm -rf src"), true);
  assert.equal(classifyArchitectAction("architect", { type: "write" }), "deny");
  assert.equal(classifyArchitectAction("architect", { type: "shell", command: "rm" }), "deny");
  assert.equal(classifyArchitectAction("architect", { type: "read" }), "allow");
  assert.equal(classifyArchitectAction("code", { type: "write" }), "allow");
});

test("/fork /task /theme /plan /grill-me are recognized composer routes", () => {
  assert.equal(recognizeRoute("/fork child"), "/fork");
  assert.equal(recognizeRoute("/task"), "/task");
  assert.equal(recognizeRoute("/theme dark"), "/theme");
  assert.equal(recognizeRoute("/plan the shell"), "/plan");
  assert.equal(recognizeRoute("/grill-me seams"), "/grill-me");
  assert.equal(recognizeRoute("hello"), null);
  assert.equal(recognizeRoute("/unknown"), null);
});
