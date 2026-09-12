/** Repo list, user list, and schedule create/list/cancel — pure, testable. */

export type RepoRecord = {
  name: string;
  visibility: string;
  html_url: string;
  branch?: string;
  worktree?: string;
  commit?: string;
};

export type UserRecord = {
  id?: string;
  login: string;
  display_name?: string;
  email?: string;
  avatar_url?: string;
  role: string;
  status: string;
  created_at?: string;
  last_active_at?: string;
};

export type ProjectRecord = {
  id: string;
  workspace_id: string;
  name: string;
  slug: string;
  description?: string;
  root_path: string;
  default_mode: string;
  default_model: string;
  status: "active" | "archived";
  created_at?: string;
  updated_at?: string;
  thread_count?: number;
  member_count?: number;
  repo_count?: number;
};

export type WorkspaceRecord = {
  id: string;
  name: string;
  slug: string;
  description?: string;
  root_path: string;
  created_at?: string;
};

export type Schedule = {
  id: string;
  title: string;
  runAt: string;
  interval: string;
};

export type ListResult<T> = {
  ok: boolean;
  items: T[];
  error: string;
};

export const FIXTURE_REPOS: unknown[] = [
  {
    name: "hideo",
    visibility: "private",
    html_url: "https://github.com/hideo/hideo",
    branch: "main",
    worktree: "/home/hideo/Documents/GitHub/hideo",
    commit: "0bf09928",
  },
  {
    name: "deepagents-app",
    private: false,
    html_url: "https://github.com/hideo/deepagents-app",
    branch: "main",
    worktree: "/home/hideo/Documents/GitHub/hideo/deepagents-app",
    commit: "head",
  },
];

export const FIXTURE_USERS: unknown[] = [
  { id: "u-hideo", login: "hideo", display_name: "Hideo", role: "sysmin", status: "active" },
  { id: "u-intortpo", login: "intortpo", display_name: "Intortpo", role: "sysmin", status: "active" },
  { id: "u-agy", login: "agy", display_name: "Antigravity Agent", role: "member", status: "invited" },
];

export const FIXTURE_PROJECTS: unknown[] = [
  {
    id: "prj-petri",
    workspace_id: "ws-default",
    name: "Petri Desktop",
    slug: "petri-desktop",
    description: "Petri AI Pair-Programming Cockpit",
    root_path: "/home/hideo/Documents/GitHub/hideo/deepagents-app",
    default_mode: "code",
    default_model: "Gemini 3.1 Pro",
    status: "active",
    thread_count: 5,
    member_count: 3,
    repo_count: 1,
  },
  {
    id: "prj-hideo",
    workspace_id: "ws-default",
    name: "Hideo Labs",
    slug: "hideo-labs",
    description: "Core agents framework and mesh nodes",
    root_path: "/home/hideo/Documents/GitHub/hideo",
    default_mode: "architect",
    default_model: "Gemini 3.1 Pro",
    status: "active",
    thread_count: 12,
    member_count: 2,
    repo_count: 2,
  },
];

function fail<T>(error: string): ListResult<T> {
  return { ok: false, items: [], error };
}

function ok<T>(items: T[]): ListResult<T> {
  return { ok: true, items, error: "" };
}

function asRecord(row: unknown): Record<string, unknown> | null {
  if (row == null || typeof row !== "object" || Array.isArray(row)) return null;
  return row as Record<string, unknown>;
}

function str(v: unknown): string {
  if (v == null) return "";
  return String(v).trim();
}

function visibilityOf(row: Record<string, unknown>): string {
  const vis = str(row.visibility);
  if (vis) return vis;
  if (row.private === true) return "private";
  if (row.private === false) return "public";
  return "public";
}

/** Normalize unknown input into repo rows. Never throws. */
export function listRepos(input: unknown): ListResult<RepoRecord> {
  if (input == null) return ok([]);
  if (!Array.isArray(input)) return fail("invalid repo list");
  const items: RepoRecord[] = [];
  for (const row of input) {
    const rec = asRecord(row);
    if (!rec) continue;
    const name = str(rec.name) || str(rec.full_name);
    if (!name) continue;
    const branch = str(rec.branch) || str(rec.default_branch);
    const worktree = str(rec.worktree) || str(rec.working_tree);
    const commit = str(rec.commit) || str(rec.sha) || str(rec.head);
    const item: RepoRecord = {
      name,
      visibility: visibilityOf(rec),
      html_url: str(rec.html_url) || str(rec.url),
    };
    if (branch) item.branch = branch;
    if (worktree) item.worktree = worktree;
    if (commit) item.commit = commit;
    items.push(item);
  }
  return ok(items);
}

/** Normalize unknown input into user rows. Never throws. */
export function listUsers(input: unknown): ListResult<UserRecord> {
  if (input == null) return ok([]);
  if (!Array.isArray(input)) return fail("invalid user list");
  const items: UserRecord[] = [];
  for (const row of input) {
    const rec = asRecord(row);
    if (!rec) continue;
    const login = str(rec.login) || str(rec.username) || str(rec.name);
    if (!login) continue;
    items.push({
      id: str(rec.id) || `u-${login}`,
      login,
      display_name: str(rec.display_name) || login,
      email: str(rec.email) || undefined,
      avatar_url: str(rec.avatar_url) || undefined,
      role: str(rec.role) || str(rec.permission) || "member",
      status: str(rec.status) || str(rec.state) || "active",
      created_at: str(rec.created_at) || undefined,
      last_active_at: str(rec.last_active_at) || undefined,
    });
  }
  return ok(items);
}

/** Normalize unknown input into project rows. Never throws. */
export function listProjects(input: unknown): ListResult<ProjectRecord> {
  if (input == null) return ok([]);
  if (!Array.isArray(input)) return fail("invalid project list");
  const items: ProjectRecord[] = [];
  for (const row of input) {
    const rec = asRecord(row);
    if (!rec) continue;
    const name = str(rec.name);
    if (!name) continue;
    const root_path = str(rec.root_path) || str(rec.project_path) || ".";
    const slug = str(rec.slug) || name.toLowerCase().replace(/\s+/g, "-");
    items.push({
      id: str(rec.id) || `prj-${slug}`,
      workspace_id: str(rec.workspace_id) || "ws-default",
      name,
      slug,
      description: str(rec.description) || undefined,
      root_path,
      default_mode: str(rec.default_mode) || "code",
      default_model: str(rec.default_model) || "Gemini 3.1 Pro",
      status: (str(rec.status) === "archived" ? "archived" : "active") as "active" | "archived",
      created_at: str(rec.created_at) || undefined,
      updated_at: str(rec.updated_at) || undefined,
      thread_count: typeof rec.thread_count === "number" ? rec.thread_count : 0,
      member_count: typeof rec.member_count === "number" ? rec.member_count : 0,
      repo_count: typeof rec.repo_count === "number" ? rec.repo_count : 0,
    });
  }
  return ok(items);
}

export function filterUsers(
  items: UserRecord[],
  search: string = "",
  roleFilter: string = "all",
  statusFilter: string = "all"
): UserRecord[] {
  const q = search.trim().toLowerCase();
  return items.filter((u) => {
    if (roleFilter !== "all" && u.role !== roleFilter) return false;
    if (statusFilter !== "all" && u.status !== statusFilter) return false;
    if (!q) return true;
    return (
      u.login.toLowerCase().includes(q) ||
      (u.display_name && u.display_name.toLowerCase().includes(q)) ||
      (u.email && u.email.toLowerCase().includes(q))
    );
  });
}

export function filterProjects(
  items: ProjectRecord[],
  search: string = "",
  statusFilter: string = "all"
): ProjectRecord[] {
  const q = search.trim().toLowerCase();
  return items.filter((p) => {
    if (statusFilter !== "all" && p.status !== statusFilter) return false;
    if (!q) return true;
    return (
      p.name.toLowerCase().includes(q) ||
      p.slug.toLowerCase().includes(q) ||
      p.root_path.toLowerCase().includes(q) ||
      (p.description && p.description.toLowerCase().includes(q))
    );
  });
}

export class ScheduleBook {
  private rows: Schedule[];

  constructor(rows: Schedule[] = []) {
    this.rows = rows.map(cloneSchedule).filter((s) => s.title);
  }

  static fromUnknown(input: unknown): ScheduleBook {
    const listed = listSchedules(input);
    return new ScheduleBook(listed.items);
  }

  create(input: { title?: string; runAt?: string; interval?: string } | null | undefined): Schedule | null {
    if (input == null || typeof input !== "object") return null;
    const title = str(input.title);
    const runAt = str(input.runAt);
    const interval = str(input.interval);
    if (!title) return null;
    if (!runAt && !interval) return null;
    const item: Schedule = {
      id: newId(),
      title,
      runAt,
      interval,
    };
    this.rows = [...this.rows, item];
    return item;
  }

  list(): Schedule[] {
    return this.rows.map(cloneSchedule);
  }

  cancel(id: string): boolean {
    const want = str(id);
    if (!want) return false;
    const next = this.rows.filter((s) => s.id !== want);
    if (next.length === this.rows.length) return false;
    this.rows = next;
    return true;
  }
}

/** List schedules from a book or a raw array. Never throws. */
export function listSchedules(input: unknown): ListResult<Schedule> {
  if (input == null) return ok([]);
  if (input instanceof ScheduleBook) return ok(input.list());
  if (!Array.isArray(input)) return fail("invalid schedule list");
  const items: Schedule[] = [];
  for (const row of input) {
    const rec = asRecord(row);
    if (!rec) continue;
    const title = str(rec.title);
    if (!title) continue;
    items.push({
      id: str(rec.id) || newId(),
      title,
      runAt: str(rec.runAt) || str(rec.run_at),
      interval: str(rec.interval),
    });
  }
  return ok(items);
}

export function createSchedule(
  book: ScheduleBook,
  input: { title?: string; runAt?: string; interval?: string },
): Schedule | null {
  return book.create(input);
}

export function cancelSchedule(book: ScheduleBook, id: string): boolean {
  return book.cancel(id);
}

function cloneSchedule(s: Schedule): Schedule {
  return { id: s.id, title: s.title, runAt: s.runAt || "", interval: s.interval || "" };
}

function newId(): string {
  return `sch_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}
