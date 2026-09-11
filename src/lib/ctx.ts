/** Context-sensitive right-click: kind → action list. */

export type CtxKind =
  | "message"
  | "fork"
  | "thread"
  | "project"
  | "mode"
  | "theme"
  | "composer"
  | "lcd"
  | "auth"
  | "mesh"
  | "shell";

export type CtxSpec = {
  id: string;
  label: string;
  kbd?: string;
  checked?: boolean;
  disabled?: boolean;
  sep?: boolean;
};

export type CtxOpts = {
  role?: string;
  hasSelection?: boolean;
  hasText?: boolean;
  isCurrent?: boolean;
  hasResult?: boolean;
  expanded?: boolean;
  currentMode?: string;
  currentTheme?: string;
  composerHasText?: boolean;
};

const MODES = ["architect", "code", "debug", "ask", "orchestrator"] as const;

export function ctxTitle(kind: CtxKind): string {
  const titles: Record<CtxKind, string> = {
    message: "Message",
    fork: "Fork",
    thread: "Thread",
    project: "Project",
    mode: "Mode",
    theme: "Theme",
    composer: "Composer",
    lcd: "Transcript",
    auth: "Auth",
    mesh: "Mesh",
    shell: "Petri",
  };
  return titles[kind];
}

export function kindFromTarget(el: { closest: (sel: string) => unknown } | null): { kind: CtxKind; el: HTMLElement } | null {
  if (!el || typeof el.closest !== "function") return null;
  const hit = el.closest("[data-ctx]") as { dataset?: { ctx?: string } } | null;
  if (hit?.dataset?.ctx) return { kind: hit.dataset.ctx as CtxKind, el: hit as HTMLElement };
  return { kind: "shell", el: el as HTMLElement };
}

export function describeContext(kind: CtxKind, opts: CtxOpts = {}): CtxSpec[] {
  switch (kind) {
    case "message": {
      const items: CtxSpec[] = [];
      if (opts.hasSelection) items.push({ id: "copy-selection", label: "Copy selection" });
      items.push({ id: "copy", label: "Copy" });
      if (opts.role === "user") items.push({ id: "edit", label: "Edit in composer" });
      items.push({ id: "sep-1", label: "", sep: true });
      items.push({ id: "branch-off", label: "Branch off" });
      items.push({ id: "new", label: "New thread" });
      return items;
    }
    case "fork":
      return [
        { id: "chase", label: "Chase" },
        { id: "copy-id", label: "Copy id" },
        { id: "toggle-fork", label: opts.expanded ? "Collapse" : "Expand" },
        ...(opts.hasResult ? [{ id: "copy-result", label: "Copy result" }] : []),
      ];
    case "thread":
      return [
        { id: "open-thread", label: "Open", disabled: Boolean(opts.isCurrent) },
        { id: "copy-id", label: "Copy id" },
        { id: "export", label: "Export" },
        { id: "open-agy", label: "Open in agy" },
        { id: "resume", label: "Resume" },
        { id: "sep-1", label: "", sep: true },
        { id: "new", label: "New thread" },
      ];
    case "project":
      return [
        { id: "copy-path", label: "Copy path" },
        { id: "new", label: "New thread" },
        { id: "index", label: "Index workspace" },
        { id: "list-projects", label: "List projects" },
      ];
    case "mode":
      return MODES.map((m) => ({
        id: `mode-${m}`,
        label: m,
        checked: opts.currentMode === m,
      }));
    case "theme":
      return [
        { id: "theme-system", label: "System", checked: opts.currentTheme === "system" },
        { id: "theme-light", label: "Light", checked: opts.currentTheme === "light" },
        { id: "theme-dark", label: "Dark", checked: opts.currentTheme === "dark" },
      ];
    case "composer": {
      const items: CtxSpec[] = [];
      if (opts.hasSelection) {
        items.push({ id: "cut", label: "Cut" });
        items.push({ id: "copy-selection", label: "Copy" });
      }
      items.push({ id: "paste", label: "Paste" });
      if (opts.composerHasText) items.push({ id: "clear", label: "Clear" });
      items.push({ id: "sep-1", label: "", sep: true });
      items.push({ id: "insert-fork", label: "Insert /fork" });
      items.push({ id: "stop", label: "Stop" });
      items.push({ id: "tree", label: "Toggle tree" });
      return items;
    }
    case "lcd":
      return [
        { id: "new", label: "New thread" },
        { id: "tree", label: "Toggle tree" },
        { id: "export", label: "Export" },
        { id: "stop", label: "Stop" },
        { id: "resume", label: "Resume" },
      ];
    case "auth":
      return [
        { id: "auth", label: "Auth status" },
        { id: "auth-doctor", label: "Auth doctor" },
      ];
    case "mesh":
      return [{ id: "copy-mesh", label: "Copy mesh status" }];
    case "shell":
    default:
      return [
        { id: "new", label: "New thread" },
        { id: "tree", label: "Toggle tree" },
        { id: "list-projects", label: "Projects" },
        { id: "sep-1", label: "", sep: true },
        { id: "theme-cycle", label: "Cycle theme" },
      ];
  }
}

export function placeMenu(
  x: number,
  y: number,
  w: number,
  h: number,
  vw: number,
  vh: number,
  pad = 8,
): { left: number; top: number } {
  let left = x;
  let top = y;
  if (left + w > vw - pad) left = Math.max(pad, vw - w - pad);
  if (top + h > vh - pad) top = Math.max(pad, vh - h - pad);
  if (left < pad) left = pad;
  if (top < pad) top = pad;
  return { left, top };
}
