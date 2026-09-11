export const DEFAULT_SYSTEM_PROMPT =
  "Always ask questions and be adversarial. brainstorm parallel. store every interaction with the app towards that users persona (each changes to adapt the user to the needs).";

export const SYSMIN_LOGINS = ["hideo"];
export const SYSMIN_EMAILS = ["intortpo@gmail.com"];

export function isSysmin(login?: string | null, email?: string | null): boolean {
  const l = (login || "").trim().toLowerCase();
  const e = (email || "").trim().toLowerCase();
  return SYSMIN_LOGINS.includes(l) || SYSMIN_EMAILS.includes(e);
}

export type PersonaRow = {
  role: string;
  content: string;
  workspace?: string;
  window?: string;
};

export function compilePersonaBrief(rows: PersonaRow[], cap = 1800): string {
  if (!rows.length) return "";
  const lines: string[] = ["[PERSONA]"];
  for (const row of rows) {
    const role = row.role === "user" ? "U" : row.role === "assistant" ? "A" : row.role;
    const text = (row.content || "").replace(/\s+/g, " ").trim();
    if (!text) continue;
    const loc = [row.workspace, row.window].filter(Boolean).join("/");
    lines.push(`${role}${loc ? `@${loc}` : ""}: ${text}`);
  }
  let out = lines.join("\n");
  if (out.length <= cap) return out;
  return out.slice(out.length - cap);
}
