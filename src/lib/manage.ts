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
  login: string;
  role: string;
  status: string;
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
  { login: "hideo", role: "owner", status: "active" },
  { login: "agy", role: "member", status: "invited" },
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
      login,
      role: str(rec.role) || str(rec.permission) || "member",
      status: str(rec.status) || str(rec.state) || "active",
    });
  }
  return ok(items);
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
