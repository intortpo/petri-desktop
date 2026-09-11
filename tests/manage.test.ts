import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  ScheduleBook,
  cancelSchedule,
  createSchedule,
  listRepos,
  listSchedules,
  listUsers,
} from "../src/lib/manage.ts";

const appSource = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../src/App.svelte"),
  "utf8",
);

test("sample repo list exposes name, visibility, and html_url", () => {
  const raw = [
    { name: "alpha", visibility: "public", html_url: "https://github.com/ex/alpha" },
    { name: "beta", private: true, html_url: "https://github.com/ex/beta" },
  ];
  const listed = listRepos(raw);
  assert.equal(listed.ok, true);
  assert.equal(listed.items.length, 2);
  const names = listed.items.map((r) => r.name);
  assert.ok(names.includes("alpha"));
  assert.ok(names.includes("beta"));
  const alpha = listed.items.find((r) => r.name === "alpha")!;
  const beta = listed.items.find((r) => r.name === "beta")!;
  assert.equal(alpha.visibility, "public");
  assert.equal(beta.visibility, "private");
  assert.equal(alpha.html_url, "https://github.com/ex/alpha");
  assert.equal(beta.html_url, "https://github.com/ex/beta");
});

test("repo listing passes through branch, worktree, and commit when present", () => {
  const listed = listRepos([
    {
      name: "alpha",
      visibility: "public",
      html_url: "https://github.com/ex/alpha",
      branch: "main",
      worktree: "/wt/alpha",
      commit: "abc1234",
    },
    { name: "beta", private: true, html_url: "https://github.com/ex/beta" },
  ]);
  const alpha = listed.items.find((r) => r.name === "alpha")!;
  const beta = listed.items.find((r) => r.name === "beta")!;
  assert.equal(alpha.branch, "main");
  assert.equal(alpha.worktree, "/wt/alpha");
  assert.equal(alpha.commit, "abc1234");
  assert.equal(beta.branch, undefined);
  assert.equal(beta.worktree, undefined);
  assert.equal(beta.commit, undefined);
});

test("sample user list exposes login, role, and status", () => {
  const raw = [
    { login: "ada", role: "owner", status: "active" },
    { login: "grace", role: "member", status: "invited" },
  ];
  const listed = listUsers(raw);
  assert.equal(listed.ok, true);
  assert.equal(listed.items.length, 2);
  const ada = listed.items.find((u) => u.login === "ada")!;
  assert.equal(ada.role, "owner");
  assert.equal(ada.status, "active");
  const grace = listed.items.find((u) => u.login === "grace")!;
  assert.equal(grace.role, "member");
  assert.equal(grace.status, "invited");
});

test("create then list finds the schedule; cancel then list does not", () => {
  const book = new ScheduleBook();
  const made = createSchedule(book, { title: "nightly index", runAt: "2026-09-12T04:00:00Z" });
  assert.ok(made);
  assert.equal(made.title, "nightly index");
  assert.equal(made.runAt, "2026-09-12T04:00:00Z");
  const afterCreate = listSchedules(book);
  assert.equal(afterCreate.items.some((s) => s.id === made.id && s.title === "nightly index"), true);
  const gone = cancelSchedule(book, made.id);
  assert.equal(gone, true);
  const afterCancel = listSchedules(book);
  assert.equal(afterCancel.items.some((s) => s.id === made.id), false);
});

test("empty and invalid repo or user input does not throw and is not a populated list", () => {
  for (const bad of [null, undefined, "", 0, { error: true }, "nope"]) {
    const repos = listRepos(bad);
    assert.equal(Array.isArray(repos.items), true);
    assert.equal(repos.items.length, 0, "invalid repos must not look populated");
    const users = listUsers(bad);
    assert.equal(Array.isArray(users.items), true);
    assert.equal(users.items.length, 0, "invalid users must not look populated");
  }
  const emptyRepos = listRepos([]);
  assert.equal(emptyRepos.ok, true);
  assert.equal(emptyRepos.items.length, 0);
  const emptyUsers = listUsers([]);
  assert.equal(emptyUsers.ok, true);
  assert.equal(emptyUsers.items.length, 0);
  const skipped = listRepos([{ html_url: "https://x" }, null, 4]);
  assert.equal(skipped.items.length, 0);
});

test("HiVE chrome ships window strip and GitHub management transforms", () => {
  assert.match(appSource, /HIVE/);
  assert.match(appSource, /by petri/);
  assert.match(appSource, /GitHub repos/);
  assert.match(appSource, /Scheduled tasks/);
  assert.match(appSource, /from ['"]\.\/lib\/manage['"]/);
  assert.match(appSource, /listRepos/);
  assert.match(appSource, /listUsers/);
  assert.match(appSource, /createSchedule/);
  assert.match(appSource, /cancelSchedule/);
  assert.match(appSource, /WIN_IDS|toggleWindow/);
  assert.match(appSource, /repo\.branch/);
  assert.match(appSource, /repo\.worktree/);
  assert.match(appSource, /repo\.commit/);
});
