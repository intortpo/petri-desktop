import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ctxTitle,
  describeContext,
  kindFromTarget,
  placeMenu,
  type CtxKind,
} from "../src/lib/ctx.ts";

test("each context kind yields a distinct action set", () => {
  const kinds: CtxKind[] = [
    "message",
    "fork",
    "thread",
    "project",
    "mode",
    "theme",
    "composer",
    "lcd",
    "auth",
    "mesh",
    "shell",
  ];
  const signatures = kinds.map((k) =>
    describeContext(k)
      .filter((i) => !i.sep)
      .map((i) => i.id)
      .join(","),
  );
  assert.equal(new Set(signatures).size, kinds.length, "kinds must not share the same menu");
});

test("message menu is copy/new; user messages add edit", () => {
  const assistant = describeContext("message", { role: "assistant" }).map((i) => i.id);
  const user = describeContext("message", { role: "user" }).map((i) => i.id);
  assert.ok(assistant.includes("copy"));
  assert.ok(assistant.includes("new"));
  assert.ok(assistant.includes("branch-off"));
  assert.ok(!assistant.includes("edit"));
  assert.ok(!assistant.includes("chase"));
  assert.ok(user.includes("edit"));
});

test("message with a selection prepends copy-selection", () => {
  const ids = describeContext("message", { hasSelection: true }).map((i) => i.id);
  assert.equal(ids[0], "copy-selection");
});

test("fork menu chases; result adds copy-result", () => {
  const bare = describeContext("fork").map((i) => i.id);
  const done = describeContext("fork", { hasResult: true }).map((i) => i.id);
  assert.ok(bare.includes("chase"));
  assert.ok(bare.includes("copy-id"));
  assert.ok(!bare.includes("copy-result"));
  assert.ok(done.includes("copy-result"));
  assert.ok(!bare.includes("edit"));
});

test("thread open is disabled when current", () => {
  const open = describeContext("thread", { isCurrent: true }).find((i) => i.id === "open-thread");
  assert.equal(open?.disabled, true);
});

test("project and composer do not share primary actions", () => {
  const project = describeContext("project").map((i) => i.id);
  const composer = describeContext("composer", { composerHasText: true }).map((i) => i.id);
  assert.ok(project.includes("copy-path"));
  assert.ok(project.includes("list-projects"));
  assert.ok(composer.includes("paste"));
  assert.ok(composer.includes("clear"));
  assert.ok(!composer.includes("copy-path"));
});

test("kindFromTarget reads closest data-ctx and falls back to shell", () => {
  const project = {
    dataset: { ctx: "project" },
    closest(sel: string) {
      return sel === "[data-ctx]" ? this : null;
    },
  };
  const inner = {
    closest(sel: string) {
      return sel === "[data-ctx]" ? project : null;
    },
  };
  const barren = {
    closest() {
      return null;
    },
  };
  assert.equal(kindFromTarget(project)?.kind, "project");
  assert.equal(kindFromTarget(inner)?.kind, "project");
  assert.equal(kindFromTarget(barren)?.kind, "shell");
  assert.equal(kindFromTarget(null), null);
});

test("placeMenu flips inside the viewport", () => {
  const flipped = placeMenu(900, 700, 200, 160, 1000, 800, 8);
  assert.equal(flipped.left, 792);
  assert.equal(flipped.top, 632);
  const inset = placeMenu(4, 4, 100, 80, 400, 300, 8);
  assert.equal(inset.left, 8);
  assert.equal(inset.top, 8);
});

test("titles match kinds", () => {
  assert.equal(ctxTitle("message"), "Message");
  assert.equal(ctxTitle("project"), "Project");
  assert.equal(ctxTitle("composer"), "Composer");
});
