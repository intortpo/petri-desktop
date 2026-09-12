export const WIN_IDS = [
  "chat",
  "github",
  "workspace",
  "settings",
  "users",
  "schedule",
  "profile",
  "branch",
] as const;

export type WinId = (typeof WIN_IDS)[number];

export const WIN_LABEL: Record<WinId, string> = {
  chat: "Chat",
  github: "GitHub repos",
  workspace: "Workspace",
  settings: "Settings",
  users: "Users",
  schedule: "Scheduled tasks",
  profile: "Profile",
  branch: "Branch",
};

export type LayoutMode = "tabs" | "split" | "grid";

export const WIN_DESCRIPTIONS: Record<WinId, string> = {
  chat: "Interactive AI pair programming and assistant turns",
  github: "Repository viewer, branches, worktrees, and status",
  workspace: "Local directory tree and project files",
  settings: "Visual theme packs and Sysmin persona configuration",
  users: "Team identities, role permissions, and access status",
  schedule: "Background cron tasks and scheduled operations",
  profile: "Current authenticated session and account role",
  branch: "Conversation branching tree and fork inspect",
};

export const WIN_ICONS: Record<WinId, string> = {
  chat: "M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H6l-2 2V4h16v12z",
  github: "M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57v-2.235c-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z",
  workspace: "M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z",
  settings: "M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z",
  users: "M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z",
  schedule: "M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z",
  profile: "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z",
  branch: "M6 2a3 3 0 0 0-3 3v14a3 3 0 0 0 5.83 1H16a3 3 0 0 0 3-3v-4.17A3 3 0 0 0 21 10a3 3 0 0 0-3-3 3 3 0 0 0-2 2.83V14H8.83A3 3 0 0 0 6 12V5a3 3 0 0 0-3-3h3zm0 18a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm12-10a1 1 0 1 1 0-2 1 1 0 0 1 0 2z",
};

export function defaultOpen(desktop: boolean): WinId[] {
  return desktop ? ["workspace", "chat", "github"] : ["chat"];
}

export function allowedWins(sysmin: boolean): WinId[] {
  return sysmin ? [...WIN_IDS] : WIN_IDS.filter((id) => id !== "users" && id !== "schedule");
}

export function activateTab(
  open: WinId[],
  active: WinId,
  target: WinId
): { open: WinId[]; active: WinId } {
  const nextOpen = open.includes(target) ? open : [...open, target];
  return { open: nextOpen, active: target };
}

export function closeTab(
  open: WinId[],
  active: WinId,
  target: WinId
): { open: WinId[]; active: WinId } {
  const nextOpen = open.filter((x) => x !== target);
  if (nextOpen.length === 0) {
    return { open: ["chat"], active: "chat" };
  }
  if (active === target) {
    const idx = open.indexOf(target);
    const newActive = nextOpen[Math.min(idx, nextOpen.length - 1)];
    return { open: nextOpen, active: newActive };
  }
  return { open: nextOpen, active };
}

export function cycleLayout(current: LayoutMode): LayoutMode {
  if (current === "tabs") return "split";
  if (current === "split") return "grid";
  return "tabs";
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
