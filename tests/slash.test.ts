import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  PAGE_TO_WIN,
  SHIPPED_MCP_SERVERS,
  SHIPPED_PLUGINS,
  SHIPPED_SKILLS,
  applySlashPick,
  buildCatalog,
  extraSlashItems,
  filterSlash,
  shippedCatalog,
} from "../src/lib/slash.ts";
import { WIN_IDS } from "../src/lib/windows.ts";

const root = dirname(fileURLToPath(import.meta.url));
const appSource = readFileSync(join(root, "../src/App.svelte"), "utf8");

const REQUIRED_COMMANDS = [
  "/new",
  "/auth",
  "/auth doctor",
  "/github token",
  "/umb",
  "/remember",
  "/recall",
  "/skills",
  "/skill",
  "/plugin install",
  "/mode",
  "/learn",
  "/index",
  "/tree",
  "/resume",
  "/goto",
  "/fork",
  "/task",
  "/project",
  "/export",
  "/open-agy",
  "/modules",
  "/theme",
  "/stop",
  "/mcp",
  "/rlm",
  "/everos",
];

const REQUIRED_PAGES = ["/chat", "/github", "/users", "/schedule"];

test("shipped catalog covers commands, pages, skills, plugins, and MCP", () => {
  const items = shippedCatalog();
  const cmds = new Set(items.map((i) => i.cmd));
  for (const cmd of REQUIRED_COMMANDS) {
    assert.ok(cmds.has(cmd), `missing command ${cmd}`);
  }
  for (const cmd of REQUIRED_PAGES) {
    assert.ok(cmds.has(cmd), `missing page ${cmd}`);
  }
  for (const skill of SHIPPED_SKILLS) {
    assert.ok(cmds.has(`/skill ${skill}`), `missing skill ${skill}`);
  }
  for (const server of SHIPPED_MCP_SERVERS) {
    assert.ok(cmds.has(`/mcp ${server}`), `missing mcp ${server}`);
  }
  for (const plugin of SHIPPED_PLUGINS) {
    assert.ok(cmds.has(plugin), `missing plugin ${plugin}`);
  }
  assert.equal(items.length, cmds.size);
  assert.ok(items.length >= REQUIRED_COMMANDS.length + REQUIRED_PAGES.length + SHIPPED_SKILLS.length);
});

test("typing / lists the full shipped catalog", () => {
  const all = filterSlash("/");
  assert.equal(all.length, shippedCatalog().length);
  assert.ok(all.some((i) => i.kind === "skill"));
  assert.ok(all.some((i) => i.kind === "mcp"));
  assert.ok(all.some((i) => i.kind === "plugin"));
  assert.ok(all.some((i) => i.kind === "page"));
  const mcp = filterSlash("/mcp");
  assert.ok(mcp.every((i) => i.cmd.toLowerCase().includes("/mcp") || i.hint.toLowerCase().includes("mcp")));
  assert.ok(mcp.some((i) => i.cmd === "/mcp ruflo"));
  const skill = filterSlash("/skill graph");
  assert.ok(skill.some((i) => i.cmd === "/skill graphify"));
});

test("runtime extras merge without dropping shipped items", () => {
  const extra = extraSlashItems({
    skills: ["brand-new-skill"],
    mcp: ["linear"],
    plugins: ["/custom-mod"],
  });
  const cmds = new Set(buildCatalog(extra).map((i) => i.cmd));
  assert.ok(cmds.has("/skill brand-new-skill"));
  assert.ok(cmds.has("/mcp linear"));
  assert.ok(cmds.has("/custom-mod"));
  assert.ok(cmds.has("/new"));
  assert.ok(cmds.has("/schedule"));
});

test("applySlashPick adds a trailing space only for arg commands", () => {
  assert.equal(applySlashPick({ cmd: "/new", hint: "New thread", kind: "command" }), "/new");
  assert.equal(
    applySlashPick({ cmd: "/fork", hint: "Fork", kind: "command", arg: true }),
    "/fork ",
  );
});

test("every handleCommand route is in the catalog", () => {
  const cmds = new Set(buildCatalog().map((i) => i.cmd));
  const exact = [...appSource.matchAll(/cmd === ["'`](\/[^"'`]+)["'`]/g)].map((m) => m[1]);
  const prefixed = [...appSource.matchAll(/cmd\.startsWith\(["'`](\/[^"'`]+)["'`]\)/g)].map((m) => m[1]);
  assert.ok(exact.length > 0, "expected handleCommand exact matches");
  for (const cmd of [...exact, ...prefixed]) {
    const hit = [...cmds].some((c) => c === cmd || c.startsWith(cmd + " ") || cmd.startsWith(c + " "));
    assert.ok(hit, `App.svelte command ${cmd} missing from slash catalog`);
  }
});

test("composer wires slash autocomplete", () => {
  assert.match(appSource, /from ['"]\.\/lib\/slash['"]/);
  assert.match(appSource, /filterSlash/);
  assert.match(appSource, /slash-menu/);
  assert.match(appSource, /e\.key === "Tab" \|\| e\.key === "Enter"/);
});

test("window pages have slash routes", () => {
  const wins = new Set(WIN_IDS);
  for (const [cmd, win] of Object.entries(PAGE_TO_WIN)) {
    assert.ok(wins.has(win as (typeof WIN_IDS)[number]), `${cmd} maps to unknown window ${win}`);
  }
  for (const id of WIN_IDS) {
    const hit = Object.values(PAGE_TO_WIN).includes(id);
    assert.ok(hit, `window ${id} has no slash page`);
  }
});
