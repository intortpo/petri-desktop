/** Composer `/` catalog: commands, skills, plugins, MCP, management views. */

export type SlashKind = "command" | "skill" | "plugin" | "mcp" | "page";

export type SlashItem = {
  cmd: string;
  hint: string;
  kind: SlashKind;
  arg?: boolean;
};

export const PAGE_TO_WIN: Record<string, string> = {
  "/chat": "chat",
  "/projects": "projects",
  "/github": "github",
  "/repos": "github",
  "/users": "users",
  "/schedule": "schedule",
  "/workspace": "workspace",
  "/settings": "settings",
  "/profile": "profile",
  "/branch": "branch",
};

export const SHIPPED_COMMANDS: SlashItem[] = [
  { cmd: "/new", hint: "New thread", kind: "command" },
  { cmd: "/auth", hint: "Auth status", kind: "command" },
  { cmd: "/auth doctor", hint: "Auth doctor", kind: "command" },
  { cmd: "/github token", hint: "Save GitHub PAT", kind: "command", arg: true },
  { cmd: "/umb", hint: "Update memory bank", kind: "command" },
  { cmd: "/remember", hint: "Remember a lesson", kind: "command", arg: true },
  { cmd: "/recall", hint: "Recall knowledge", kind: "command", arg: true },
  { cmd: "/skills", hint: "List skills", kind: "command" },
  { cmd: "/skill", hint: "Pin a skill this turn", kind: "command", arg: true },
  { cmd: "/plugin", hint: "Plugin help", kind: "plugin" },
  { cmd: "/plugin install", hint: "Install plugin from URL", kind: "plugin", arg: true },
  { cmd: "/mode", hint: "Switch mode", kind: "command", arg: true },
  { cmd: "/mode architect", hint: "Architect mode", kind: "command" },
  { cmd: "/mode code", hint: "Code mode", kind: "command" },
  { cmd: "/mode debug", hint: "Debug mode", kind: "command" },
  { cmd: "/mode ask", hint: "Ask mode", kind: "command" },
  { cmd: "/mode orchestrator", hint: "Orchestrator mode", kind: "command" },
  { cmd: "/learn", hint: "Learn a skill from this thread", kind: "command" },
  { cmd: "/index", hint: "Index workspace", kind: "command" },
  { cmd: "/tree", hint: "Toggle thread tree", kind: "command" },
  { cmd: "/resume", hint: "Resume last agy turn", kind: "command" },
  { cmd: "/goto", hint: "Jump to a thread", kind: "command", arg: true },
  { cmd: "/fork", hint: "Fork a child thread", kind: "command", arg: true },
  { cmd: "/task", hint: "List or queue tasks", kind: "command" },
  { cmd: "/project", hint: "List or switch project", kind: "command" },
  { cmd: "/export", hint: "Export thread", kind: "command" },
  { cmd: "/open-agy", hint: "Open in agy", kind: "command" },
  { cmd: "/apps", hint: "Open all accessible apps", kind: "command" },
  { cmd: "/layout", hint: "Switch layout mode (tabs|split|grid)", kind: "command", arg: true },
  { cmd: "/layout tabs", hint: "Single focused tab mode", kind: "command" },
  { cmd: "/layout split", hint: "Dual-pane master/detail mode", kind: "command" },
  { cmd: "/layout grid", hint: "Tiled dashboard grid mode", kind: "command" },
  { cmd: "/tab next", hint: "Switch to next tab", kind: "command" },
  { cmd: "/tab prev", hint: "Switch to previous tab", kind: "command" },
  { cmd: "/modules", hint: "List modules", kind: "plugin" },
  { cmd: "/theme", hint: "Theme system|light|dark", kind: "command" },
  { cmd: "/theme system", hint: "Follow system theme", kind: "command" },
  { cmd: "/theme light", hint: "Light theme", kind: "command" },
  { cmd: "/theme dark", hint: "Dark theme", kind: "command" },
  { cmd: "/plan", hint: "Implementation plan", kind: "command", arg: true },
  { cmd: "/grill-me", hint: "Stress-test design assumptions", kind: "command", arg: true },
  { cmd: "/adr", hint: "Author an ADR (docs/adr/)", kind: "command", arg: true },
  { cmd: "/context", hint: "Domain dictionary (CONTEXT.md)", kind: "command", arg: true },
  { cmd: "/rlm", hint: "Spawn RLM job", kind: "command", arg: true },
  { cmd: "/refine", hint: "Refine harness memory", kind: "command", arg: true },
  { cmd: "/everos", hint: "EverOS memory operations", kind: "command" },
  { cmd: "/everos status", hint: "EverOS health status", kind: "command" },
  { cmd: "/everos search", hint: "Search EverOS memory", kind: "command", arg: true },
  { cmd: "/everos flush", hint: "Flush EverOS memory turns", kind: "command" },
  { cmd: "/everos add", hint: "Add EverOS memory fact", kind: "command", arg: true },
  { cmd: "/everos profile", hint: "View EverOS user profile", kind: "command" },
  { cmd: "/stop", hint: "Stop current turn", kind: "command" },
];

export const SHIPPED_PAGES: SlashItem[] = [
  { cmd: "/chat", hint: "Chat", kind: "page" },
  { cmd: "/projects", hint: "Projects", kind: "page" },
  { cmd: "/github", hint: "GitHub repos", kind: "page" },
  { cmd: "/repos", hint: "GitHub repos", kind: "page" },
  { cmd: "/users", hint: "Users", kind: "page" },
  { cmd: "/schedule", hint: "Scheduled tasks", kind: "page" },
  { cmd: "/workspace", hint: "Workspace", kind: "page" },
  { cmd: "/settings", hint: "Settings", kind: "page" },
  { cmd: "/profile", hint: "Profile", kind: "page" },
  { cmd: "/branch", hint: "Branch", kind: "page" },
];

export const SHIPPED_MCP_SERVERS = [
  "cloudflare-docs",
  "github",
  "google_calendar",
  "ruflo",
  "tasks",
] as const;

export const SHIPPED_PLUGINS = ["/github-viewer"] as const;

export const SHIPPED_SKILLS = [
  "agentdb-advanced",
  "agentdb-learning",
  "agentdb-memory-patterns",
  "agentdb-optimization",
  "agentdb-vector-search",
  "agents-sdk",
  "browser",
  "build-with-ai",
  "cavecrew",
  "caveman",
  "caveman-commit",
  "caveman-compress",
  "caveman-help",
  "caveman-review",
  "caveman-stats",
  "cloudflare",
  "cloudflare-email-service",
  "cloudflare-one",
  "cloudflare-one-migrations",
  "code-review",
  "create-skill",
  "create-workflow",
  "design",
  "docx",
  "durable-objects",
  "execute-plan",
  "find-skills",
  "flow-nexus-neural",
  "flow-nexus-platform",
  "flow-nexus-swarm",
  "game-animation-frames",
  "game-asset-core",
  "game-character-consistency",
  "game-tilesets",
  "game-ui-icons",
  "github-code-review",
  "github-multi-repo",
  "github-project-management",
  "github-release-management",
  "github-workflow-automation",
  "graphify",
  "hooks-automation",
  "humanise",
  "imagine",
  "implement",
  "interaction-design",
  "interface-design",
  "learn",
  "liquid-glass-element",
  "long-running-background-tasks",
  "memos-memory",
  "nextjs-on-cloudflare",
  "pair-programming",
  "pdf",
  "pptx",
  "pr-babysit",
  "reasoningbank-agentdb",
  "reasoningbank-intelligence",
  "resume-claude",
  "resume-codex",
  "resume-cursor",
  "review",
  "ruflo",
  "sandbox-migrate-to-next",
  "sandbox-next",
  "sandbox-stable",
  "skill-builder",
  "skill-design-principles",
  "sparc-methodology",
  "statusline",
  "stream-chain",
  "swarm-advanced",
  "swarm-orchestration",
  "turnstile-spin",
  "v3-cli-modernization",
  "v3-core-implementation",
  "v3-ddd-architecture",
  "v3-integration-deep",
  "v3-mcp-optimization",
  "v3-memory-unification",
  "v3-performance-optimization",
  "v3-security-overhaul",
  "v3-swarm-coordination",
  "verification-quality",
  "watch",
  "web-perf",
  "workers-best-practices",
  "wrangler",
] as const;

export function skillItems(names: readonly string[]): SlashItem[] {
  return uniqueNames(names).map((name) => ({
    cmd: `/skill ${name}`,
    hint: `Pin ${name}`,
    kind: "skill" as const,
  }));
}

export function mcpItems(names: readonly string[]): SlashItem[] {
  const list: SlashItem[] = [{ cmd: "/mcp", hint: "List MCP servers", kind: "mcp" }];
  for (const name of uniqueNames(names)) {
    list.push({ cmd: `/mcp ${name}`, hint: `MCP ${name}`, kind: "mcp" });
  }
  return list;
}

export function pluginItems(commands: readonly string[]): SlashItem[] {
  return uniqueNames(commands)
    .map((cmd) => (cmd.startsWith("/") ? cmd : `/${cmd}`))
    .map((cmd) => ({ cmd, hint: "Plugin command", kind: "plugin" as const }));
}

export function extraSlashItems(opts: {
  skills?: readonly string[];
  mcp?: readonly string[];
  plugins?: readonly string[];
} = {}): SlashItem[] {
  return [
    ...skillItems(opts.skills ?? []),
    ...mcpItems(opts.mcp ?? []).filter((i) => i.cmd !== "/mcp"),
    ...pluginItems(opts.plugins ?? []),
  ];
}

export function shippedCatalog(): SlashItem[] {
  return mergeItems([
    ...SHIPPED_COMMANDS,
    ...SHIPPED_PAGES,
    ...mcpItems(SHIPPED_MCP_SERVERS),
    ...pluginItems(SHIPPED_PLUGINS),
    ...skillItems(SHIPPED_SKILLS),
  ]);
}

export function buildCatalog(extra: SlashItem[] = []): SlashItem[] {
  return mergeItems([...shippedCatalog(), ...extra]);
}

export function filterSlash(input: string, extra: SlashItem[] = []): SlashItem[] {
  const raw = input ?? "";
  if (!raw.startsWith("/")) return [];
  const catalog = buildCatalog(extra);
  const q = raw.trim().toLowerCase();
  if (q === "/") return catalog;
  return catalog.filter((item) => {
    const cmd = item.cmd.toLowerCase();
    const hint = item.hint.toLowerCase();
    const needle = q.slice(1);
    return cmd.startsWith(q) || cmd.includes(q) || hint.includes(needle);
  });
}

export function applySlashPick(item: SlashItem): string {
  return item.arg ? `${item.cmd} ` : item.cmd;
}

function uniqueNames(names: readonly string[]): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const n of names) {
    const name = String(n ?? "").trim();
    if (!name || seen.has(name)) continue;
    seen.add(name);
    out.push(name);
  }
  return out;
}

function mergeItems(items: SlashItem[]): SlashItem[] {
  const seen = new Set<string>();
  const out: SlashItem[] = [];
  for (const item of items) {
    if (!item?.cmd || seen.has(item.cmd)) continue;
    seen.add(item.cmd);
    out.push(item);
  }
  return out;
}
