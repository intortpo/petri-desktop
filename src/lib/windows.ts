export const WIN_IDS = [
  "chat",
  "github",
  "workspace",
  "settings",
  "users",
  "profile",
  "branch",
] as const;

export type WinId = (typeof WIN_IDS)[number];

export const WIN_LABEL: Record<WinId, string> = {
  chat: "Chat",
  github: "GitHub",
  workspace: "Workspace",
  settings: "Settings",
  users: "Users",
  profile: "Profile",
  branch: "Branch",
};

export function defaultOpen(desktop: boolean): WinId[] {
  return desktop ? ["workspace", "chat", "github"] : ["chat"];
}

export function toggleWin(open: WinId[], id: WinId): WinId[] {
  if (open.includes(id)) {
    const next = open.filter((x) => x !== id);
    return next.length ? next : ["chat"];
  }
  return [...open, id];
}

export function ensureWin(open: WinId[], id: WinId): WinId[] {
  return open.includes(id) ? open : [...open, id];
}

export function isDesktop(width: number): boolean {
  return width >= 900;
}
