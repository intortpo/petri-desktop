import { test } from "node:test";
import assert from "node:assert/strict";
import {
  FIXTURE_PROJECTS,
  FIXTURE_USERS,
  filterProjects,
  filterUsers,
  listProjects,
  listUsers,
} from "../src/lib/manage.ts";
import { WIN_IDS, WIN_LABEL, allowedWins } from "../src/lib/windows.ts";
import { PAGE_TO_WIN, filterSlash } from "../src/lib/slash.ts";

test("listProjects extracts name, slug, root_path, and relational metric counts", () => {
  const raw = [
    {
      id: "prj-alpha",
      workspace_id: "ws-1",
      name: "Alpha Project",
      slug: "alpha-project",
      root_path: "/path/to/alpha",
      status: "active",
      thread_count: 8,
      member_count: 4,
      repo_count: 2,
    },
    {
      name: "Beta Project",
      project_path: "/path/to/beta",
      status: "archived",
    },
  ];
  const listed = listProjects(raw);
  assert.equal(listed.ok, true);
  assert.equal(listed.items.length, 2);

  const alpha = listed.items[0];
  assert.equal(alpha.name, "Alpha Project");
  assert.equal(alpha.slug, "alpha-project");
  assert.equal(alpha.root_path, "/path/to/alpha");
  assert.equal(alpha.status, "active");
  assert.equal(alpha.thread_count, 8);
  assert.equal(alpha.member_count, 4);
  assert.equal(alpha.repo_count, 2);

  const beta = listed.items[1];
  assert.equal(beta.name, "Beta Project");
  assert.equal(beta.slug, "beta-project");
  assert.equal(beta.root_path, "/path/to/beta");
  assert.equal(beta.status, "archived");
  assert.equal(beta.thread_count, 0);
});

test("filterProjects filters by text query and status", () => {
  const projects = listProjects(FIXTURE_PROJECTS).items;
  assert.ok(projects.length >= 2);

  const petriHits = filterProjects(projects, "petri");
  assert.equal(petriHits.length, 1);
  assert.equal(petriHits[0].slug, "petri-desktop");

  const activeHits = filterProjects(projects, "", "active");
  assert.equal(activeHits.length, projects.length);

  const archivedHits = filterProjects(projects, "", "archived");
  assert.equal(archivedHits.length, 0);
});

test("filterUsers filters by text, role, and status", () => {
  const users = listUsers(FIXTURE_USERS).items;
  assert.ok(users.length >= 3);

  const sysmins = filterUsers(users, "", "sysmin");
  assert.equal(sysmins.length, 2);
  assert.ok(sysmins.some((u) => u.login === "hideo"));
  assert.ok(sysmins.some((u) => u.login === "intortpo"));

  const members = filterUsers(users, "", "member");
  assert.equal(members.length, 1);
  assert.equal(members[0].login, "agy");

  const querySearch = filterUsers(users, "hideo");
  assert.equal(querySearch.length, 1);
  assert.equal(querySearch[0].login, "hideo");
});

test("hierarchy routing: /projects maps to projects window and is accessible in navigation", () => {
  assert.ok(WIN_IDS.includes("projects"));
  assert.equal(WIN_LABEL["projects"], "Projects");
  assert.equal(PAGE_TO_WIN["/projects"], "projects");

  const hits = filterSlash("/proj");
  assert.ok(hits.some((h) => h.cmd === "/projects"));

  const sysminWins = allowedWins(true);
  const memberWins = allowedWins(false);
  assert.ok(sysminWins.includes("projects"));
  assert.ok(memberWins.includes("projects"));
});
