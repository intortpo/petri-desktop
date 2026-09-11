import { DEFAULT_PACK_ID, packById, type Pack } from "./palettes";

export type ThemePref = "system" | "light" | "dark";

const KEY = "petri.theme";
const PACK_KEY = "petri.themePack";

export function readPref(): ThemePref {
  const v = localStorage.getItem(KEY);
  if (v === "light" || v === "dark" || v === "system") return v;
  return "system";
}

export function readPackId(): string {
  return localStorage.getItem(PACK_KEY) || DEFAULT_PACK_ID;
}

export function resolveTheme(pref: ThemePref): "light" | "dark" {
  if (pref === "light" || pref === "dark") return pref;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyPack(pack: Pack) {
  const root = document.documentElement;
  root.style.setProperty("--void", pack.void);
  root.style.setProperty("--ash", pack.ash);
  root.style.setProperty("--slate", pack.slate);
  root.style.setProperty("--ice", pack.ice);
  root.style.setProperty("--record", pack.record);
  root.style.setProperty("--lcd-ink", pack.lcdInk || pack.ash);
  root.dataset.pack = pack.id;
  root.dataset.packHot = pack.hot;
  localStorage.setItem(PACK_KEY, pack.id);
}

export function applyTheme(pref: ThemePref, packId?: string) {
  const resolved = resolveTheme(pref);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.themePref = pref;
  localStorage.setItem(KEY, pref);
  applyPack(packById(packId ?? readPackId()));
  return resolved;
}

export function parseThemeArg(raw: string): ThemePref | null {
  const a = raw.trim().toLowerCase();
  if (a === "light" || a === "day") return "light";
  if (a === "dark" || a === "nite" || a === "night") return "dark";
  if (a === "system" || a === "sys" || a === "") return "system";
  return null;
}
