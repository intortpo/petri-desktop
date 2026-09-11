/** Architect-mode design artifacts and fences — pure, no I/O. */

export const VALID_MODES = ["architect", "code", "debug", "ask", "orchestrator"] as const;
export type ModeId = (typeof VALID_MODES)[number];

export const MODEL_CHOICES = [
  "Gemini 3.1 Pro (High)",
  "Gemini 3.1 Pro",
  "Gemini 2.5 Pro",
] as const;

export const ADR_DESTINATION = "docs/adr/";
export const CONTEXT_DESTINATION = "CONTEXT.md";

export const DESIGN_ROUTES = ["/plan", "/grill-me", "/adr", "/context"] as const;
export const COMPOSER_ROUTES = ["/fork", "/task", "/theme", ...DESIGN_ROUTES] as const;

const WRITE_TOOLS = new Set(["write", "edit", "shell", "apply_patch", "rm", "rmdir"]);

export function isValidMode(mode: string): mode is ModeId {
  return (VALID_MODES as readonly string[]).includes(mode);
}

/** Ask / orchestrator / architect do not receive write tools. */
export function modeDeniesWrite(mode: string): boolean {
  return mode === "architect" || mode === "ask" || mode === "orchestrator";
}

export function architectIsReadOnly(mode: string): boolean {
  return mode === "architect";
}

export function architectAllowsTool(mode: string, tool: string): boolean {
  if (!modeDeniesWrite(mode)) return true;
  const name = String(tool || "").trim().toLowerCase();
  if (!name) return false;
  return !WRITE_TOOLS.has(name);
}

export function architectAllowsShell(mode: string, command: string): boolean {
  if (!modeDeniesWrite(mode)) return true;
  void command;
  return false;
}

export function classifyArchitectAction(
  mode: string,
  action: { type: "read" | "write" | "shell"; command?: string },
): "allow" | "deny" {
  if (action.type === "read") return "allow";
  if (action.type === "write") return architectAllowsTool(mode, "write") ? "allow" : "deny";
  return architectAllowsShell(mode, action.command || "") ? "allow" : "deny";
}

function str(v: unknown): string {
  if (v == null) return "";
  return String(v).trim();
}

function bullets(items: string[]): string {
  return items.map((item) => `- ${item}`).join("\n");
}

export type PlanInput = {
  topic: string;
  components?: string[];
  verification?: string[];
  risks?: string[];
  layout?: string[];
  modules?: string[];
  dependencies?: string[];
  boundaries?: string[];
  seams?: string[];
  audits?: string[];
};

export function buildPlan(input: PlanInput | string): string {
  const spec: PlanInput = typeof input === "string" ? { topic: input } : input || { topic: "" };
  const topic = str(spec.topic) || "untitled system";
  const components = nonempty(spec.components, [
    `Map ${topic} layout, core modules, and dependency graph`,
    `Name boundaries and seams that ${topic} must not leak across`,
    `Sequence implementation behind verification gates for ${topic}`,
  ]);
  const verification = nonempty(spec.verification, [
    `Each ${topic} component has a checkable acceptance criterion`,
    `Discovery artifacts (layout, modules, dependencies, boundaries) are listed`,
    `Risks for ${topic} have a mitigation or an explicit accept`,
  ]);
  const risks = nonempty(spec.risks, [
    `Premature file mutation or destructive shell while designing ${topic}`,
    `Tight coupling and leaky abstractions around ${topic} seams`,
    `Unverified security posture or data-flow bottlenecks in ${topic}`,
  ]);
  const layout = nonempty(spec.layout, [`Project layout of ${topic} (roots, packages, entrypoints)`]);
  const modules = nonempty(spec.modules, [`Core modules of ${topic}`]);
  const dependencies = nonempty(spec.dependencies, [`Inbound and outbound dependencies of ${topic}`]);
  const boundaries = nonempty(spec.boundaries, [`Ownership boundaries around ${topic}`]);
  const seams = nonempty(spec.seams, [
    `Tight coupling candidates in ${topic}`,
    `Leaky abstractions that a deep-module refactor could close`,
  ]);
  const audits = nonempty(spec.audits, [
    `Data-flow trace across ${topic}`,
    `Security posture of ${topic} trust boundaries`,
    `Bottleneck identification on the ${topic} hot path`,
  ]);

  return [
    `# Implementation plan: ${topic}`,
    "",
    "## Component breakdown",
    bullets(components),
    "",
    "## Verification criteria",
    bullets(verification),
    "",
    "## Risk assessment",
    bullets(risks),
    "",
    "## Codebase discovery",
    "### Layout",
    bullets(layout),
    "### Modules",
    bullets(modules),
    "### Dependencies",
    bullets(dependencies),
    "### Boundaries",
    bullets(boundaries),
    "",
    "## Pattern and seam identification",
    bullets(seams),
    "",
    "## Audits and diagnostic traces",
    bullets(audits),
  ].join("\n");
}

export type AdrInput = {
  title: string;
  context?: string;
  decision?: string;
  tradeoffs?: string[];
  boundaries?: string[];
  choices?: string[];
};

export function buildAdr(input: AdrInput | string): { destination: string; markdown: string } {
  const spec: AdrInput = typeof input === "string" ? { title: input } : input || { title: "" };
  const title = str(spec.title) || "Untitled decision";
  const context = str(spec.context) || `Context for choosing how to bound ${title}.`;
  const decision = str(spec.decision) || `Choose the option that keeps ${title} behind a stable boundary.`;
  const tradeoffs = nonempty(spec.tradeoffs, [
    `Simpler local change vs a deeper module boundary for ${title}`,
    `Speed of delivery vs long-term isolation of ${title}`,
  ]);
  const boundaries = nonempty(spec.boundaries, [
    `${title} must not leak write/shell side effects into architect mode`,
    `Callers depend on the published interface, not the internals of ${title}`,
  ]);
  const choices = nonempty(spec.choices, [
    `Keep ${title} as a structured transcript artifact (docs/adr/)`,
    `Fold ${title} into ad-hoc comments (rejected: no recoverable record)`,
  ]);
  const markdown = [
    `# ADR: ${title}`,
    "",
    `Destination: ${ADR_DESTINATION}`,
    "",
    "## Context",
    context,
    "",
    "## Decision",
    decision,
    "",
    "## Choices",
    bullets(choices),
    "",
    "## Trade-offs",
    bullets(tradeoffs),
    "",
    "## Boundaries",
    bullets(boundaries),
  ].join("\n");
  return { destination: ADR_DESTINATION, markdown };
}

export type TermPair = { term: string; definition: string };

export function buildDomainDictionary(
  terms: Record<string, string> | TermPair[] | string,
): { destination: string; markdown: string } {
  const pairs = normalizeTerms(terms);
  const rows = pairs.map((p) => `| ${p.term} | ${p.definition} |`);
  const markdown = [
    "# CONTEXT.md",
    "",
    `Destination: ${CONTEXT_DESTINATION}`,
    "",
    "## Ubiquitous language",
    "",
    "| Term | Definition |",
    "| --- | --- |",
    ...rows,
  ].join("\n");
  return { destination: CONTEXT_DESTINATION, markdown };
}

export type GrillInput = {
  topic: string;
  assumptions?: string[];
};

export function buildGrill(input: GrillInput | string): string {
  const spec: GrillInput = typeof input === "string" ? { topic: input } : input || { topic: "" };
  const topic = str(spec.topic) || "the current design";
  const assumptions = nonempty(spec.assumptions, defaultAssumptions(topic));
  const challenges = assumptions.map((assumption, i) => {
    return [
      `### Assumption ${i + 1}`,
      `Claim: ${assumption}`,
      `Challenge: What evidence falsifies this for ${topic}? What happens if the opposite is true?`,
    ].join("\n");
  });
  return [
    `# Grill: ${topic}`,
    "",
    "Relentless stress-test of design assumptions before implementation.",
    "",
    ...challenges,
    "",
    "## Coupling and seams",
    `Challenge: which ${topic} abstractions leak, and which modules are too tightly coupled to split?`,
    "",
    "## Data-flow and security",
    `Challenge: name a ${topic} data-flow the plan has not traced, and a trust boundary it has not audited.`,
    "",
    "## Premature mutation",
    `Challenge: why must ${topic} stay read-only until verification criteria are accepted?`,
  ].join("\n");
}

export type SlashRoute = { cmd: string; arg: string };

export function parseSlashRoute(text: string): SlashRoute | null {
  const t = str(text);
  if (!t.startsWith("/")) return null;
  const m = t.match(/^(\/[a-z0-9-]+)(?:\s+([\s\S]*))?$/i);
  if (!m) return null;
  return { cmd: m[1].toLowerCase(), arg: str(m[2]) };
}

export function recognizeRoute(text: string): string | null {
  const parsed = parseSlashRoute(text);
  if (!parsed) return null;
  return (COMPOSER_ROUTES as readonly string[]).includes(parsed.cmd) ? parsed.cmd : null;
}

export function runDesignCommand(cmd: string, arg = ""): string | null {
  const route = cmd.startsWith("/") ? cmd.toLowerCase() : `/${cmd.toLowerCase()}`;
  if (route === "/plan") return buildPlan(arg || "current workspace");
  if (route === "/grill-me") {
    const assumptions = splitAssumptions(arg);
    const topic = assumptions.length > 1 ? assumptions[0] : arg || "current design";
    return buildGrill({
      topic,
      assumptions: assumptions.length > 1 ? assumptions.slice(1) : assumptions.length === 1 ? assumptions : undefined,
    });
  }
  if (route === "/adr") {
    const made = buildAdr(arg || "Untitled decision");
    return `${made.destination}\n\n${made.markdown}`;
  }
  if (route === "/context") {
    const made = buildDomainDictionary(arg);
    return `${made.destination}\n\n${made.markdown}`;
  }
  return null;
}

function nonempty(given: string[] | undefined, fallback: string[]): string[] {
  const cleaned = (given || []).map(str).filter(Boolean);
  return cleaned.length ? cleaned : fallback;
}

function defaultAssumptions(topic: string): string[] {
  return [
    `${topic} boundaries are already well defined`,
    `existing modules can absorb ${topic} without a new seam`,
    `security and data-flow for ${topic} need no extra audit`,
    `implementation of ${topic} can start before a written plan`,
  ];
}

function splitAssumptions(arg: string): string[] {
  const raw = str(arg);
  if (!raw) return [];
  return raw
    .split(/[\n;]+/)
    .map(str)
    .filter(Boolean);
}

function normalizeTerms(terms: Record<string, string> | TermPair[] | string): TermPair[] {
  if (typeof terms === "string") {
    const parsed = parseTermPairs(terms);
    return parsed.length ? parsed : [{ term: terms || "proj", definition: terms ? `Working definition of ${terms}` : "bound workspace / repository" }];
  }
  if (Array.isArray(terms)) {
    return terms
      .map((row) => ({ term: str(row?.term), definition: str(row?.definition) }))
      .filter((row) => row.term);
  }
  if (terms && typeof terms === "object") {
    return Object.entries(terms)
      .map(([term, definition]) => ({ term: str(term), definition: str(definition) }))
      .filter((row) => row.term);
  }
  return [{ term: "proj", definition: "bound workspace / repository" }];
}

export function parseTermPairs(raw: string): TermPair[] {
  const text = str(raw);
  if (!text) return [];
  const parts = text.split(/[\n;]+/).map(str).filter(Boolean);
  const out: TermPair[] = [];
  for (const part of parts) {
    const m = part.match(/^([^:=]+)[:=]\s*(.+)$/);
    if (m) out.push({ term: str(m[1]), definition: str(m[2]) });
  }
  return out;
}
