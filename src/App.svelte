<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import SilkBg from './lib/SilkBg.svelte';
  import petriIcon from './assets/petri-icon.png';
  import { applyTheme, parseThemeArg, readPref, type ThemePref } from './lib/theme';
  import { DEFAULT_PACK_ID, PACKS } from './lib/palettes';
  import { DEFAULT_SYSTEM_PROMPT, isSysmin } from './lib/persona';
  import { recallPersona, rememberTurn } from './lib/reasoningbank';
  import { WIN_IDS, WIN_LABEL, defaultOpen, ensureWin, isDesktop, toggleWin, type WinId } from './lib/windows';
  import { formatMessage } from './lib/format';
  import {
    ctxTitle,
    describeContext,
    kindFromTarget,
    placeMenu,
    type CtxKind,
    type CtxSpec,
  } from './lib/ctx';
  import {
    FIXTURE_REPOS,
    FIXTURE_USERS,
    ScheduleBook,
    cancelSchedule,
    createSchedule,
    listRepos,
    listSchedules,
    listUsers,
    type ListResult,
    type RepoRecord,
    type Schedule,
    type UserRecord,
  } from './lib/manage';

  type MeshThread = {
    id: string;
    project_path?: string | null;
    mode: string;
    model: string;
    parent_id?: string | null;
    agy_conversation_id?: string | null;
    status: string;
    summary?: string | null;
  };

  type MeshTask = {
    id: string;
    thread_id: string;
    child_thread_id?: string | null;
    title: string;
    prompt: string;
    status: string;
    result?: string | null;
  };

  type ForkCard = {
    child_id: string;
    task_id: string;
    title: string;
    status: string;
    result?: string | null;
  };

  // Status Bar State
  let authStatus = "Checking...";
  let tailscaleStatus = "Checking...";
  let currentProject = ".";
  let currentMode = "architect";
  let currentModel = "Gemini 3.1 Pro (High)";
  let currentQuota = "Unknown";

  // Thread State
  let currentThreadId = "";
  let currentThread: MeshThread | null = null;
  let messages: any[] = [];
  let trailThreads: MeshThread[] = [];
  let showTree = false;
  let treeThreads: MeshThread[] = [];
  let unread: Record<string, number> = {};
  let expandedForks: Record<string, boolean> = {};
  let moduleCommands: Record<string, string> = {};

  // Input
  let composerInput = "";
  let themePref: ThemePref = "system";
  let themeResolved: "light" | "dark" = "light";
  let packId = DEFAULT_PACK_ID;
  let showProjects = false;
  let projectList: string[] = [];
  let projRoot: HTMLDivElement | null = null;
  let view: "chat" | "repos" | "users" | "schedule" = "chat";
  let openWins: WinId[] = defaultOpen(typeof window !== "undefined" ? isDesktop(window.innerWidth) : true);
  let session: { login: string; email: string } | null = null;
  let gateNotice = "";
  let patDraft = "";
  let deviceLogin: { user_code: string; verification_uri: string } | null = null;
  let meshOpen = false;
  let cores: { id: string; online: boolean }[] = [];
  let audit: { kind: string; detail: string; created_at?: string }[] = [];
  let systemPrompt = DEFAULT_SYSTEM_PROMPT;
  let branchThreadId = "";
  let branchMessages: any[] = [];
  let workspaceTree: { name: string; path: string; kind: string }[] = [];
  let repoResult: ListResult<RepoRecord> = listRepos([]);
  let userResult: ListResult<UserRecord> = listUsers([]);
  let scheduleBook = new ScheduleBook();
  let schedules: Schedule[] = [];
  let schedTitle = "";
  let schedWhen = "";
  let schedInterval = "";
  let schedNotice = "";
  let composerEl: HTMLInputElement | null = null;
  let ctxRoot: HTMLDivElement | null = null;
  let ctxMenu: { x: number; y: number; kind: CtxKind; spec: CtxSpec[]; title: string } | null = null;
  let ctxPayload: Record<string, any> = {};

  function projectName(path: string) {
    const parts = (path || "").replace(/\\/g, "/").split("/").filter(Boolean);
    return parts[parts.length - 1] || path || "project";
  }

  async function loadProjects() {
    try {
      let projects = await invoke<string[]>("list_projects");
      if (!projects.includes(currentProject)) projects = [currentProject, ...projects];
      projectList = projects;
    } catch {
      projectList = [currentProject];
    }
  }

  async function toggleProjects() {
    if (showProjects) {
      showProjects = false;
      return;
    }
    await loadProjects();
    showProjects = true;
  }

  async function selectProject(path: string) {
    currentProject = path;
    showProjects = false;
    if (showTree) await refreshTree();
  }

  function cycleTheme() {
    const order: ThemePref[] = ["system", "light", "dark"];
    const next = order[(order.indexOf(themePref) + 1) % order.length];
    themePref = next;
    themeResolved = applyTheme(next, packId);
  }

  function restoreSchedules() {
    try {
      const raw = JSON.parse(localStorage.getItem("petri.schedules") || "[]");
      scheduleBook = ScheduleBook.fromUnknown(raw);
    } catch {
      scheduleBook = new ScheduleBook();
    }
    schedules = listSchedules(scheduleBook).items;
  }

  function persistSchedules() {
    try {
      localStorage.setItem("petri.schedules", JSON.stringify(scheduleBook.list()));
    } catch {}
    schedules = listSchedules(scheduleBook).items;
  }

  async function githubJson(path: string): Promise<unknown> {
    const pat = await invoke<string>("get_github_pat");
    if (!pat) throw new Error("no pat");
    const res = await fetch(`https://api.github.com${path}`, {
      headers: {
        Authorization: `Bearer ${pat}`,
        Accept: "application/vnd.github+json",
      },
    });
    if (!res.ok) throw new Error(String(res.status));
    return res.json();
  }

  async function refreshRepos() {
    try {
      repoResult = listRepos(await githubJson("/user/repos?per_page=50"));
      if (repoResult.ok && repoResult.items.length) return;
    } catch {}
    repoResult = listRepos(FIXTURE_REPOS);
  }

  async function refreshUsers() {
    try {
      const me = await githubJson("/user");
      const rec = me && typeof me === "object" ? (me as { login?: string }) : {};
      const rows = rec.login ? [{ login: rec.login, role: "owner", status: "active" }] : [];
      userResult = listUsers(rows);
      if (userResult.ok && userResult.items.length) return;
    } catch {}
    userResult = listUsers(FIXTURE_USERS);
  }

  async function openView(next: typeof view) {
    view = next;
    if (next === "repos") await refreshRepos();
    if (next === "users") await refreshUsers();
    if (next === "schedule") restoreSchedules();
  }

  function submitSchedule() {
    const made = createSchedule(scheduleBook, {
      title: schedTitle,
      runAt: schedWhen,
      interval: schedInterval,
    });
    if (!made) {
      schedNotice = "Need a title and a run-at or interval.";
      return;
    }
    schedTitle = "";
    schedWhen = "";
    schedInterval = "";
    schedNotice = "";
    persistSchedules();
  }

  function dropSchedule(id: string) {
    cancelSchedule(scheduleBook, id);
    persistSchedules();
  }

  onMount(() => {
    themePref = readPref();
    packId = DEFAULT_PACK_ID;
    themeResolved = applyTheme(themePref, packId);
    restoreSchedules();
    checkStatus();
    setupListeners();
    openWins = defaultOpen(isDesktop(window.innerWidth));

    const onPtr = (e: PointerEvent) => {
      const t = e.target as Node | null;
      if (showProjects && projRoot && t && !projRoot.contains(t)) showProjects = false;
      if (ctxMenu && ctxRoot && t && !ctxRoot.contains(t)) ctxMenu = null;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        showProjects = false;
        ctxMenu = null;
      }
    };
    document.addEventListener("pointerdown", onPtr);
    document.addEventListener("keydown", onKey);

    (async () => {
      await restoreSession();
      if (!session) return;
      await loadModules();
      try {
        const last = await invoke<MeshThread | null>("get_last_thread");
        if (last?.id) {
          await openThread(last.id);
          return;
        }
      } catch (e) {
        console.log("No last thread", e);
      }
      await handleCommand("/new");
    })();

    return () => {
      document.removeEventListener("pointerdown", onPtr);
      document.removeEventListener("keydown", onKey);
    };
  });

  async function loadModules() {
    try {
      const manifests = await invoke<{ name: string; file: string; commands: string[] }[]>(
        "list_module_manifests",
        { projectPath: currentProject }
      );
      const map: Record<string, string> = {};
      for (const m of manifests) {
        for (const c of m.commands || []) {
          map[c] = m.name;
        }
      }
      moduleCommands = map;
    } catch {
      moduleCommands = {};
    }
  }

  async function checkStatus() {
    try {
      authStatus = await invoke<string>("get_auth_status");
    } catch (e) {
      authStatus = "Error: " + e;
    }

    try {
      tailscaleStatus = await invoke<string>("get_tailscale_status");
    } catch (e) {
      tailscaleStatus = "Offline";
    }
  }

  async function restoreSession() {
    try {
      const s = await invoke<{ login: string; email: string } | null>("github_session");
      if (s?.login) {
        session = s;
        gateNotice = "";
        return;
      }
    } catch {}
    try {
      const pat = await invoke<string>("get_github_pat");
      if (pat?.trim()) {
        const msg = await invoke<string>("verify_github_pat", { pat: pat.trim() });
        const login = (msg.match(/as\s+(\S+)/i) || [])[1] || "user";
        session = { login, email: "" };
        await invoke("save_github_session", { login, email: "" }).catch(() => {});
      }
    } catch {}
  }

  async function loginWithPat() {
    gateNotice = "";
    try {
      const msg = await invoke<string>("verify_github_pat", { pat: patDraft.trim() });
      await invoke("save_github_pat", { pat: patDraft.trim() });
      const login = (msg.match(/as\s+(\S+)/i) || [])[1] || "user";
      session = { login, email: "" };
      await invoke("save_github_session", { login, email: "" }).catch(() => {});
      patDraft = "";
      await loadModules();
      await handleCommand("/new");
    } catch (e) {
      gateNotice = String(e);
    }
  }

  async function startGithubDevice() {
    gateNotice = "";
    try {
      deviceLogin = await invoke("github_device_start");
    } catch (e) {
      gateNotice = String(e);
    }
  }

  function setPack(id: string) {
    packId = id;
    themeResolved = applyTheme(themePref, id);
  }

  function toggleWindow(id: WinId) {
    openWins = toggleWin(openWins, id);
    if (id === "repos" as never) openView("repos");
    if (id === "github") openView("repos");
    if (id === "users") openView("users");
    if (id === "workspace") loadTree();
  }

  async function loadTree() {
    try {
      workspaceTree = await invoke("list_workspace_tree", { path: currentProject });
    } catch {
      workspaceTree = [];
    }
  }

  async function openSysmin() {
    meshOpen = !meshOpen;
    if (!meshOpen) return;
    try {
      cores = await invoke("list_cores");
    } catch {
      cores = [
        { id: "gcloud", online: authStatus.includes("Authenticated") },
        { id: "laptop1", online: true },
        { id: "laptop2", online: false },
        { id: "phone1", online: false },
        { id: "phone2", online: false },
        { id: "phone3", online: false },
      ];
    }
    try {
      audit = await invoke("list_audit");
    } catch {
      audit = [];
    }
  }

  async function branchOff(idx: number) {
    const msg = messages[idx];
    const prompt = typeof msg?.content === "string" ? msg.content : "";
    if (!prompt || !currentThreadId) return;
    try {
      const forked = await invoke<{ thread: MeshThread; task: MeshTask; card: any }>("fork_thread", {
        parentId: currentThreadId,
        prompt: `Continue from: ${prompt.slice(0, 400)}`,
        mode: currentMode === "orchestrator" ? "code" : currentMode,
        start: true,
      });
      messages = [...messages, forked.card];
      branchThreadId = forked.thread.id;
      openWins = ensureWin(openWins, "branch");
      branchMessages = await invoke<any[]>("get_messages", { threadId: branchThreadId }).catch(() => [] as any[]);
    } catch (e) {
      system(`Error: ${e}`);
    }
  }

  function shortId(id: string) {
    return (id || "").slice(0, 8);
  }

  function threadLabel(t: MeshThread | null | undefined) {
    if (!t) return "thread";
    if (t.summary && t.summary.trim()) {
      const s = t.summary.trim().replace(/\s+/g, " ");
      return s.length > 28 ? s.slice(0, 28) + "…" : s;
    }
    return shortId(t.id);
  }

  function parseFork(msg: any): ForkCard | null {
    if (!msg || msg.role !== "fork") return null;
    try {
      const parsed = JSON.parse(msg.content);
      if (parsed && parsed.child_id) return parsed as ForkCard;
    } catch {
      return null;
    }
    return null;
  }

  function bumpUnread(threadId: string) {
    if (!threadId || threadId === currentThreadId) return;
    unread[threadId] = (unread[threadId] || 0) + 1;
    unread = { ...unread };
  }

  async function refreshTrail(threadId: string) {
    try {
      trailThreads = await invoke<MeshThread[]>("thread_trail", { threadId });
    } catch {
      trailThreads = currentThread ? [currentThread] : [];
    }
  }

  async function refreshTree() {
    try {
      treeThreads = await invoke<MeshThread[]>("list_threads", { projectPath: currentProject });
    } catch {
      treeThreads = [];
    }
  }

  async function openThread(threadId: string) {
    const thread = await invoke<MeshThread>("get_thread", { threadId });
    currentThread = thread;
    currentThreadId = thread.id;
    currentMode = thread.mode || currentMode;
    currentModel = thread.model || currentModel;
    if (thread.project_path) currentProject = thread.project_path;
    messages = await invoke<any[]>("get_messages", { threadId });
    unread[threadId] = 0;
    unread = { ...unread };
    await refreshTrail(threadId);
    if (showTree) await refreshTree();
  }

  function setupListeners() {
    listen("agy-event", (event: any) => {
      const data = event.payload || {};
      const eventThread = data.thread_id || currentThreadId;

      if (data.event === "init" && data.conversation_id) {
        invoke("save_agy_id", {
          threadId: eventThread,
          agyId: data.conversation_id
        }).catch(() => {});
      }

      if (eventThread && eventThread !== currentThreadId) {
        bumpUnread(eventThread);
        return;
      }

      if (data.event === "step_update" && data.step_update?.text_delta) {
        let lastMsg = messages[messages.length - 1];
        if (lastMsg && lastMsg.role === "assistant") {
          lastMsg.content += data.step_update.text_delta;
          messages = [...messages];
        } else {
          messages = [...messages, { role: "assistant", content: data.step_update.text_delta }];
        }
      } else if (data.event === "result") {
        // Assistant text is persisted by the Rust agy session so background forks boomerang.
      }
    }).catch(() => {});

    listen("agy-done", async (event: any) => {
      const payload = event.payload;
      const doneId = typeof payload === "string" ? payload : payload?.thread_id;
      if (!doneId) return;
      if (doneId === currentThreadId) return;
      if (currentThreadId) {
        try {
          const parent = await invoke<MeshThread>("get_thread", { threadId: doneId });
          if (parent.parent_id === currentThreadId) {
            messages = await invoke<any[]>("get_messages", { threadId: currentThreadId });
          }
        } catch {}
      }
      bumpUnread(doneId);
      if (showTree) await refreshTree();
    }).catch(() => {});

    listen("agy-log", (event: any) => {
      console.log("AGY LOG:", event.payload);
    }).catch(() => {});
  }

  function system(content: string) {
    messages = [...messages, { role: "system", content }];
  }

  async function handleCommand(cmd: string) {
    if (cmd === "/new") {
      try {
        let thread: MeshThread = await invoke("create_thread", {
          projectPath: currentProject,
          mode: currentMode,
          model: currentModel
        });
        currentThread = thread;
        currentThreadId = thread.id;
        messages = [];
        await refreshTrail(thread.id);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/auth") {
      system(`Auth Source: ${authStatus}\nRun 'agy' in terminal to log in.`);
    } else if (cmd === "/auth doctor") {
      try {
        const report = await invoke<string>("auth_doctor");
        system(report);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/github token")) {
      const pat = cmd.split(" ")[2];
      if (pat) {
        await invoke<string>("save_github_pat", { pat });
        let verify = await invoke<string>("verify_github_pat", { pat });
        system(`GitHub token saved! Status: ${verify}`);
      } else {
        system("Usage: /github token ghp_...");
      }
    } else if (cmd === "/umb") {
      system("Updating Memory Bank...");
      try {
        await invoke("update_memory_bank", { projectPath: currentProject });
        system("Memory Bank update initiated in background.");
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/remember ")) {
      const lesson = cmd.replace("/remember ", "");
      try {
        await invoke("remember_lesson", { projectPath: currentProject, lesson });
        system("Lesson remembered.");
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/recall ")) {
      const query = cmd.replace("/recall ", "");
      try {
        let results = await invoke<string>("recall_knowledge", { query, projectPath: currentProject });
        system(`Recall results:\n${results}`);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/skills") {
      try {
        let skills = await invoke<string[]>("list_skills", { projectPath: currentProject });
        let skillList = skills.length > 0 ? skills.join("\n- ") : "No skills found.";
        system(`Discovered Skills:\n- ${skillList}`);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/skill ")) {
      const skillName = cmd.replace("/skill ", "");
      system(`Pinned skill: ${skillName} for this turn.`);
    } else if (cmd.startsWith("/plugin install ")) {
      const url = cmd.replace("/plugin install ", "");
      system(`Installing plugin from ${url}...`);
      try {
        let result = await invoke<string>("install_plugin", { url });
        system(`Plugin installation result:\n${result}`);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/mode ")) {
      const mode = cmd.replace("/mode ", "");
      const validModes = ["architect", "code", "debug", "ask", "orchestrator"];
      if (validModes.includes(mode)) {
        currentMode = mode;
        if (currentThreadId) {
          try {
            await invoke<string>("set_thread_mode", { threadId: currentThreadId, mode });
            if (currentThread) currentThread = { ...currentThread, mode };
          } catch (e) {
            system(`Mode UI updated but persist failed: ${e}`);
          }
        }
        system(`Switched to mode: ${mode}\nTool fences active.`);
      } else {
        system(`Unknown mode. Valid modes: ${validModes.join(", ")}`);
      }
    } else if (cmd === "/learn") {
      system("Analyzing transcript to extract reusable skills...");
      try {
        let result = await invoke<string>("learn_from_thread", { threadId: currentThreadId });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/index") {
      system("Initiating local workspace vector indexing...");
      try {
        let result = await invoke<string>("index_workspace", { projectPath: currentProject });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/tree") {
      showTree = !showTree;
      if (showTree) await refreshTree();
    } else if (cmd === "/resume") {
      if (!currentThreadId) {
        system("No active thread to resume.");
        return;
      }
      system("Resuming last agy conversation for this thread...");
      try {
        await invoke("resume_thread", { threadId: currentThreadId, prompt: "Continue.", mode: currentMode });
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/goto ")) {
      const query = cmd.replace("/goto ", "").trim();
      try {
        const found = await invoke<MeshThread | null>("find_thread", { query });
        if (!found) {
          system(`No thread matching '${query}'.`);
        } else {
          await openThread(found.id);
          system(`Jumped to ${threadLabel(found)} (${shortId(found.id)})`);
        }
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/fork" || cmd.startsWith("/fork ")) {
      const prompt = cmd === "/fork" ? "" : cmd.replace("/fork ", "").trim();
      if (!prompt) {
        system("Usage: /fork <sub-task prompt>\nCreates a child thread, runs agy, and leaves a result card here.");
        return;
      }
      if (!currentThreadId) {
        system("Open or /new a thread first.");
        return;
      }
      try {
        const forked = await invoke<{ thread: MeshThread; task: MeshTask; card: any }>("fork_thread", {
          parentId: currentThreadId,
          prompt,
          mode: currentMode === "orchestrator" ? "code" : currentMode,
          start: true
        });
        messages = [...messages, forked.card];
        system(`Forked #${shortId(forked.thread.id)} · ${forked.task.title}\nChild is running in the background. Click the card to chase.`);
        if (showTree) await refreshTree();
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/task" || cmd.startsWith("/task ")) {
      const extra = cmd === "/task" ? "" : cmd.replace("/task ", "").trim();
      if (!currentThreadId) {
        system("No active thread.");
        return;
      }
      try {
        if (extra) {
          const task = await invoke<MeshTask>("create_task", {
            threadId: currentThreadId,
            title: extra.length > 48 ? extra.slice(0, 48) + "…" : extra,
            prompt: extra
          });
          system(`Queued task ${shortId(task.id)} · ${task.title}\nRun it with /fork ${extra}`);
        }
        const tasks = await invoke<MeshTask[]>("list_tasks", { threadId: currentThreadId });
        if (!tasks.length) {
          system("No tasks on this thread yet. /fork <prompt> creates a running child.");
          return;
        }
        const lines = tasks.map((t) => {
          const mark = t.status === "done" ? "done" : t.status === "failed" ? "fail" : t.status === "running" ? "run " : "wait";
          const child = t.child_thread_id ? `#${shortId(t.child_thread_id)}` : "—";
          const tail = t.result ? `\n    ${t.result.replace(/\n/g, " ").slice(0, 120)}` : "";
          return `  ${mark}  ${child}  ${t.title}${tail}`;
        });
        system(`Task graph · ${tasks.length}\n${lines.join("\n")}`);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/project" || cmd.startsWith("/project ")) {
      const arg = cmd === "/project" ? "list" : cmd.replace("/project ", "").trim();
      try {
        if (!arg || arg === "list") {
          let projects = await invoke<string[]>("list_projects");
          if (!projects.includes(currentProject)) projects = [currentProject, ...projects];
          system(`Projects\n${projects.map((p) => (p === currentProject ? `  * ${p}` : `    ${p}`)).join("\n")}\n\n/project <path> to switch.`);
        } else {
          currentProject = arg;
          system(`Project set to ${currentProject}\nNext /new or send uses this workspace.`);
        }
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/export") {
      try {
        const result = await invoke<string>("export_thread", { threadId: currentThreadId });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/open-agy") {
      try {
        const result = await invoke<string>("open_agy", { threadId: currentThreadId });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/modules") {
      try {
        await loadModules();
        const mods = await invoke<string[]>("list_modules", { projectPath: currentProject });
        const cmds = Object.keys(moduleCommands);
        system(
          (mods.length ? `Modules:\n- ${mods.join("\n- ")}` : "No modules in modules/.") +
            (cmds.length ? `\nRegistered commands: ${cmds.join(" ")}` : "")
        );
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/theme" || cmd.startsWith("/theme ")) {
      const arg = cmd === "/theme" ? "" : cmd.slice(7);
      const parsed = parseThemeArg(arg);
      if (!parsed && arg.trim()) {
        system("Usage: /theme [system|light|dark]");
      } else {
        themePref = parsed || "system";
        themeResolved = applyTheme(themePref, packId);
        system(`Theme: ${themePref} (${themeResolved})`);
      }
    } else if (cmd === "/stop") {
      try {
        const result = await invoke<string>("stop_turn", { threadId: currentThreadId });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/mcp") {
      try {
        const result = await invoke<string>("list_mcp", { projectPath: currentProject });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (moduleCommands[cmd] || moduleCommands[cmd.split(" ")[0]]) {
      const key = moduleCommands[cmd] ? cmd : cmd.split(" ")[0];
      system(`Module ${moduleCommands[key]} handled ${cmd} (no app rebuild).`);
    } else {
      system(`Unknown command: ${cmd}`);
    }
  }

  function aliasToCommand(text: string): string | null {
    const t = text.trim();
    const forkNl = t.match(/^(?:please\s+)?fork\s+(.+)$/i);
    if (forkNl) return `/fork ${forkNl[1]}`;
    const modeNl = t.match(/^switch to (architect|code|debug|ask|orchestrator)$/i);
    if (modeNl) return `/mode ${modeNl[1].toLowerCase()}`;
    if (/^open this in antigravity$/i.test(t)) return "/open-agy";
    if (/^show tasks$/i.test(t)) return "/task";
    if (/^show tree$/i.test(t)) return "/tree";
    const themeNl = t.match(/^theme (light|dark|system)$/i);
    if (themeNl) return `/theme ${themeNl[1].toLowerCase()}`;
    return null;
  }

  async function handleSend() {
    if (!composerInput.trim()) return;

    let text = composerInput;
    composerInput = "";

    const aliased = aliasToCommand(text);
    if (aliased) text = aliased;

    if (text.startsWith("/")) {
      await handleCommand(text);
      return;
    }

    messages = [...messages, { role: "user", content: text }];

    try {
      const persona = session ? await recallPersona(session.login, text) : "";
      await invoke("start_turn", {
        threadId: currentThreadId,
        prompt: text,
        mode: currentMode,
        persona: persona || null,
        systemPrompt,
      });
      if (session) {
        await rememberTurn({
          login: session.login,
          query: text,
          workspace: currentProject,
          window: "chat",
        });
      }
    } catch (e) {
      system(`Error: ${e}`);
    }
  }

  async function chaseFork(card: ForkCard) {
    if (!card?.child_id) return;
    try {
      await openThread(card.child_id);
    } catch (e) {
      system(`Error chasing fork: ${e}`);
    }
  }

  function toggleFork(id: string) {
    expandedForks[id] = !expandedForks[id];
    expandedForks = { ...expandedForks };
  }

  function treeChildren(parentId: string | null) {
    return treeThreads.filter((t) => (t.parent_id || null) === parentId);
  }

  function unreadOnTrail(t: MeshThread) {
    const own = unread[t.id] || 0;
    const kids = treeThreads.length
      ? treeThreads.filter((c) => c.parent_id === t.id).reduce((n, c) => n + (unread[c.id] || 0), 0)
      : 0;
    return own + kids;
  }

  async function windowAction(action: "min" | "max" | "close") {
    const w = getCurrentWindow();
    if (action === "min") await w.minimize();
    else if (action === "max") await w.toggleMaximize();
    else await w.close();
  }

  async function copyText(text: string) {
    const t = text ?? "";
    try {
      await navigator.clipboard.writeText(t);
    } catch {
      const ta = document.createElement("textarea");
      ta.value = t;
      ta.setAttribute("readonly", "");
      ta.style.position = "fixed";
      ta.style.left = "-9999px";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      ta.remove();
    }
  }

  function composerSel() {
    const el = composerEl;
    if (!el) return { start: 0, end: 0, text: "" };
    let start = el.selectionStart ?? 0;
    let end = el.selectionEnd ?? 0;
    if (start > end) [start, end] = [end, start];
    return { start, end, text: composerInput.slice(start, end) };
  }

  function insertComposer(text: string) {
    const el = composerEl;
    const start = el?.selectionStart ?? composerInput.length;
    const end = el?.selectionEnd ?? composerInput.length;
    const a = Math.min(start, end);
    const b = Math.max(start, end);
    composerInput = composerInput.slice(0, a) + text + composerInput.slice(b);
    tick().then(() => {
      if (!composerEl) return;
      const pos = a + text.length;
      composerEl.focus();
      composerEl.setSelectionRange(pos, pos);
    });
  }

  async function onContext(e: MouseEvent) {
    const t = e.target as HTMLElement | null;
    if (t?.closest(".ctx-menu")) return;
    e.preventDefault();
    showProjects = false;

    const hit = kindFromTarget(t);
    const kind = hit?.kind ?? "shell";
    const el = hit?.el;
    const liveSel = (window.getSelection?.()?.toString() || "").trim();
    const inComposer = Boolean(t?.closest("[data-ctx='composer']"));
    const csel = inComposer ? composerSel() : { text: "", start: 0, end: 0 };
    const hasSelection = inComposer ? csel.text.length > 0 : liveSel.length > 0;

    const payload: Record<string, any> = { selection: inComposer ? csel.text : liveSel };
    const opts: Parameters<typeof describeContext>[1] = { hasSelection };

    if (kind === "message" || kind === "fork") {
      const i = Number(el?.dataset.idx);
      const msg = messages[i];
      opts.role = msg?.role;
      payload.msg = msg;
      payload.idx = i;
      if (kind === "fork") {
        const card = parseFork(msg);
        payload.card = card;
        opts.hasResult = Boolean(card?.result);
        opts.expanded = Boolean(card && expandedForks[card.child_id]);
      }
    } else if (kind === "thread") {
      payload.id = el?.dataset.id || "";
      opts.isCurrent = payload.id === currentThreadId;
    } else if (kind === "mode") {
      opts.currentMode = currentMode;
    } else if (kind === "theme") {
      opts.currentTheme = themePref;
    } else if (kind === "composer") {
      opts.composerHasText = Boolean(composerInput);
    }

    ctxPayload = payload;
    ctxMenu = {
      x: e.clientX,
      y: e.clientY,
      kind,
      spec: describeContext(kind, opts),
      title: ctxTitle(kind),
    };
    await tick();
    if (ctxMenu && ctxRoot) {
      const r = ctxRoot.getBoundingClientRect();
      const p = placeMenu(ctxMenu.x, ctxMenu.y, r.width, r.height, window.innerWidth, window.innerHeight);
      ctxMenu = { ...ctxMenu, x: p.left, y: p.top };
      await tick();
      (ctxRoot.querySelector("button:not([disabled])") as HTMLButtonElement | null)?.focus();
    }
  }

  async function runCtx(id: string) {
    const payload = ctxPayload;
    ctxMenu = null;
    if (id === "copy-selection") await copyText(payload.selection || composerSel().text);
    else if (id === "copy") await copyText(payload.msg?.content || "");
    else if (id === "edit") {
      composerInput = payload.msg?.content || "";
      composerEl?.focus();
    } else if (id === "new") await handleCommand("/new");
    else if (id === "branch-off") await branchOff(Number(payload.idx));
    else if (id === "chase" && payload.card) await chaseFork(payload.card);
    else if (id === "copy-id") await copyText(payload.card?.child_id || payload.id || "");
    else if (id === "toggle-fork" && payload.card) toggleFork(payload.card.child_id);
    else if (id === "copy-result") await copyText(payload.card?.result || "");
    else if (id === "open-thread" && payload.id) await openThread(payload.id);
    else if (id === "export") await handleCommand("/export");
    else if (id === "open-agy") await handleCommand("/open-agy");
    else if (id === "resume") await handleCommand("/resume");
    else if (id === "copy-path") await copyText(currentProject);
    else if (id === "index") await handleCommand("/index");
    else if (id === "list-projects") await toggleProjects();
    else if (id.startsWith("mode-")) await handleCommand("/mode " + id.slice(5));
    else if (id === "theme-system" || id === "theme-light" || id === "theme-dark") {
      const pref = id.slice(6) as ThemePref;
      themePref = pref;
      themeResolved = applyTheme(pref);
    } else if (id === "theme-cycle") cycleTheme();
    else if (id === "cut") {
      const s = composerSel();
      await copyText(s.text);
      composerInput = composerInput.slice(0, s.start) + composerInput.slice(s.end);
    } else if (id === "paste") {
      try {
        insertComposer(await navigator.clipboard.readText());
      } catch {}
    } else if (id === "clear") composerInput = "";
    else if (id === "insert-fork") insertComposer("/fork ");
    else if (id === "stop") await handleCommand("/stop");
    else if (id === "tree") await handleCommand("/tree");
    else if (id === "auth") await handleCommand("/auth");
    else if (id === "auth-doctor") await handleCommand("/auth doctor");
    else if (id === "copy-mesh") await copyText(tailscaleStatus);
  }
</script>

<SilkBg />

{#if !session}
<div class="gate">
  <div class="glass gate-card">
    <img class="brand-icon" src={petriIcon} alt="" width="72" height="72" />
    <strong class="brand-name">HIVE</strong>
    <em class="brand-by">by petri</em>
    <p>Sign in with GitHub to enter.</p>
    <button type="button" class="send" on:click={startGithubDevice}>Continue with GitHub</button>
    {#if deviceLogin}
      <p class="empty">Open {deviceLogin.verification_uri} and enter {deviceLogin.user_code}</p>
    {/if}
    <input bind:value={patDraft} placeholder="or paste a GitHub PAT" aria-label="GitHub PAT" />
    <button type="button" class="key" on:click={loginWithPat}>Use PAT</button>
    {#if gateNotice}<p class="empty">{gateNotice}</p>{/if}
  </div>
</div>
{:else}
<div class="shell" role="application" on:contextmenu={onContext}>
  <header class="chrome">
    <div class="lead">
      <span class="brand" title="HIVE by petri">
        <img class="brand-icon" src={petriIcon} alt="" width="56" height="56" />
        <span class="brand-copy">
          <strong class="brand-name">HIVE</strong>
          <em class="brand-by">by petri</em>
        </span>
      </span>
      <div class="proj" data-ctx="project" bind:this={projRoot}>
        <button
          type="button"
          class="chip proj-key"
          aria-expanded={showProjects}
          aria-haspopup="listbox"
          on:click={toggleProjects}
        >
          <em>proj</em> {projectName(currentProject)}
        </button>
        {#if showProjects}
          <div class="proj-menu" role="listbox" aria-label="Projects">
            {#each projectList as p}
              <button
                type="button"
                role="option"
                aria-selected={p === currentProject}
                class:here={p === currentProject}
                on:click={() => selectProject(p)}
              >
                <span class="proj-name">{projectName(p)}</span>
                <span class="proj-path">{p}</span>
              </button>
            {/each}
            {#if projectList.length === 0}
              <p class="empty">No projects yet.</p>
            {/if}
          </div>
        {/if}
      </div>
      <span class="chip" data-ctx="mode"><em>mode</em> {currentMode}</span>
      <span class="chip truncate"><em>model</em> {currentModel}</span>
    </div>
    <div data-tauri-drag-region class="drag"></div>
    <div class="meta">
      <span data-ctx="auth" class="led {authStatus.includes('Unauthenticated') ? 'off' : 'on'}">{authStatus.includes('Unauthenticated') ? 'OFF' : 'OK'}</span>
      <span data-ctx="mesh" class="led {tailscaleStatus.includes('Offline') ? 'off' : 'on'}">MESH</span>
      <button type="button" class="theme-key" data-ctx="theme" on:click={cycleTheme} title="Theme">{themePref === 'system' ? 'SYS' : themePref === 'light' ? 'DAY' : 'NITE'}</button>
    </div>
    <div class="win">
      <button type="button" on:click={() => windowAction("min")} aria-label="Minimize">–</button>
      <button type="button" on:click={() => windowAction("max")} aria-label="Maximize">□</button>
      <button type="button" class="close" on:click={() => windowAction("close")} aria-label="Close">×</button>
    </div>
  </header>
  <nav class="views" aria-label="Windows">
    {#each WIN_IDS as id}
      <button type="button" class:here={openWins.includes(id)} on:click={() => toggleWindow(id)}>{WIN_LABEL[id]}</button>
    {/each}
  </nav>

  <main class="desk">
    {#if openWins.includes("workspace")}
      <section class="pane glass manage" data-view="workspace">
        <h1>Workspace</h1>
        {#if workspaceTree.length === 0}
          <p class="empty">{currentProject}</p>
        {:else}
          <ul class="cards">
            {#each workspaceTree as node}
              <li><strong>{node.name}</strong><span class="tag">{node.kind}</span></li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
    {#if openWins.includes("github")}
      <section class="pane glass manage" data-view="repos">
        <h1>GitHub repos</h1>
        {#if !repoResult.ok}
          <p class="empty">Could not load repos.</p>
        {:else if repoResult.items.length === 0}
          <p class="empty">No repos.</p>
        {:else}
          <ul class="cards">
            {#each repoResult.items as repo}
              <li>
                <strong>{repo.name}</strong>
                <span class="tag">{repo.visibility}</span>
                {#if repo.html_url}
                  <a class="md-a" href={repo.html_url} rel="noopener noreferrer">{repo.html_url}</a>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
    {#if openWins.includes("users")}
      <section class="pane glass manage" data-view="users">
        <h1>Users</h1>
        {#if !userResult.ok}
          <p class="empty">Could not load users.</p>
        {:else if userResult.items.length === 0}
          <p class="empty">No users.</p>
        {:else}
          <ul class="cards">
            {#each userResult.items as user}
              <li>
                <strong>{user.login}</strong>
                <span class="tag">{user.role}</span>
                <span class="tag">{user.status}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
    {#if openWins.includes("settings")}
      <section class="pane glass manage" data-view="settings">
        <h1>Settings</h1>
        <p class="tag">Theme pack</p>
        <ul class="cards">
          {#each PACKS as pack}
            <li>
              <button type="button" class:here={packId === pack.id} on:click={() => setPack(pack.id)}>{pack.name}</button>
            </li>
          {/each}
        </ul>
        {#if isSysmin(session?.login, session?.email)}
          <p class="tag">System prompt</p>
          <textarea class="prompt-box" bind:value={systemPrompt} rows="5"></textarea>
        {/if}
      </section>
    {/if}
    {#if openWins.includes("profile")}
      <section class="pane glass manage" data-view="profile">
        <h1>Profile</h1>
        <p><strong>{session?.login}</strong></p>
        <p class="tag">{session?.email || "github"}</p>
        <p class="tag">{isSysmin(session?.login, session?.email) ? "sysmin" : "member"}</p>
      </section>
    {/if}
    {#if openWins.includes("branch")}
      <section class="pane glass manage" data-view="branch">
        <h1>Branch</h1>
        {#if !branchThreadId}
          <p class="empty">Right-click a message to branch off.</p>
        {:else}
          {#each branchMessages as msg}
            <article class="bubble {msg.role}">
              <div class="role">{msg.role}</div>
              <div class="pad">{msg.content}</div>
            </article>
          {/each}
        {/if}
      </section>
    {/if}
    {#if openWins.includes("chat")}
      <section class="pane glass chat-pane">
    <nav class="trail">
      <button type="button" data-ctx="shell" on:click={() => handleCommand("/tree")}>Home</button>
      {#each trailThreads as segment, i}
        <span class="sep">/</span>
        <button
          type="button"
          data-ctx="thread"
          data-id={segment.id}
          class={segment.id === currentThreadId ? 'here' : ''}
          on:click={() => openThread(segment.id)}
        >
          {threadLabel(segment)}
          {#if unreadOnTrail(segment) > 0 && segment.id !== currentThreadId}
            <i class="badge">{unreadOnTrail(segment)}</i>
          {/if}
        </button>
        {#if i === trailThreads.length - 1 && currentThread?.parent_id}
          <span class="child">child</span>
        {/if}
      {/each}
    </nav>

    <div class="body">
      {#if showTree}
        <aside class="tree">
          <div class="tree-h">thread tree</div>
          {#each treeChildren(null) as root}
            <button type="button" data-ctx="thread" data-id={root.id} class={root.id === currentThreadId ? 'here' : ''} on:click={() => openThread(root.id)}>
              {threadLabel(root)}
              {#if unread[root.id]}<i class="badge">{unread[root.id]}</i>{/if}
            </button>
            {#each treeChildren(root.id) as child}
              <button type="button" data-ctx="thread" data-id={child.id} class="indent {child.id === currentThreadId ? 'here' : ''}" on:click={() => openThread(child.id)}>
                {threadLabel(child)} <em>{child.status}</em>
              </button>
              {#each treeChildren(child.id) as grand}
                <button type="button" data-ctx="thread" data-id={grand.id} class="indent2 {grand.id === currentThreadId ? 'here' : ''}" on:click={() => openThread(grand.id)}>
                  {threadLabel(grand)}
                </button>
              {/each}
            {/each}
          {/each}
          {#if treeThreads.length === 0}
            <p class="empty">No threads.</p>
          {/if}
        </aside>
      {/if}

      <section class="lcd" data-ctx="lcd">
        {#if messages.length === 0}
          <div class="idle">No thread yet</div>
        {/if}
        {#each messages as msg, i}
          {#if msg.role === "fork"}
            {@const card = parseFork(msg)}
            {#if card}
              <article class="bubble fork" data-ctx="fork" data-idx={i}>
                <div class="role">fork</div>
                <div class="fork-row">
                  <button type="button" class="fork-main" on:click={() => toggleFork(card.child_id)}>
                    <span class="id">#{shortId(card.child_id)}</span>
                    <span class="st {card.status}">{card.status}</span>
                    <div>{card.title}</div>
                  </button>
                  <button type="button" class="key" on:click={() => chaseFork(card)}>chase</button>
                </div>
                {#if expandedForks[card.child_id] || card.status === "done" || card.status === "failed"}
                  {#if card.result}
                    <div class="result md">{@html formatMessage(card.result)}</div>
                  {:else if card.status === "running"}
                    <p class="muted">running…</p>
                  {/if}
                {/if}
              </article>
            {/if}
          {:else}
            <article class="bubble {msg.role}" data-ctx="message" data-idx={i}>
              <div class="role">{msg.role}</div>
              {#if msg.role === "user"}
                <div class="pad">{msg.content}</div>
              {:else}
                <div class="pad md">{@html formatMessage(msg.content)}</div>
              {/if}
            </article>
          {/if}
        {/each}
      </section>
    </div>
      </section>
    {/if}

  </main>

  {#if openWins.includes("chat")}
  <form class="dock glass" data-ctx="composer" on:submit|preventDefault={handleSend}>
    <div class="dock-row">
      <input
        type="text"
        bind:this={composerEl}
        bind:value={composerInput}
        placeholder="message or /fork /task /theme…"
      />
      <button type="submit" class="send">SEND</button>
    </div>
  </form>
  {/if}

  {#if ctxMenu}
    <div
      class="ctx-menu"
      bind:this={ctxRoot}
      style="left: {ctxMenu.x}px; top: {ctxMenu.y}px"
      role="menu"
      aria-label={ctxMenu.title}
    >
      <header>{ctxMenu.title}</header>
      {#each ctxMenu.spec as item}
        {#if item.sep}
          <div class="ctx-sep"></div>
        {:else}
          <button
            type="button"
            role="menuitem"
            class:checked={item.checked}
            disabled={item.disabled}
            on:click={() => runCtx(item.id)}
          >
            <span>{item.label}</span>
            {#if item.checked}<span class="ctx-mark">●</span>{/if}
          </button>
        {/if}
      {/each}
    </div>
  {/if}

  {#if isSysmin(session?.login, session?.email)}
    <button type="button" class="sysmin-hex" aria-label="Mesh overview" on:click={openSysmin}></button>
    {#if meshOpen}
      <aside class="glass mesh-pop">
        <h1>Mesh</h1>
        {#each cores as core}
          <p class="tag">{core.id} · {core.online ? "online" : "off"}</p>
        {/each}
        <h1>Audit</h1>
        {#each audit as row}
          <p class="tag">{row.kind} {row.detail}</p>
        {/each}
      </aside>
    {/if}
  {/if}
</div>
{/if}

<style>
  .shell {
    position: relative;
    z-index: 1;
    height: 100dvh;
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    color: var(--ink);
  }
  .chrome {
    display: flex;
    align-items: center;
    flex-shrink: 0;
    min-height: 72px;
    padding: 0 8px 0 40px;
    gap: 24px;
    border-bottom: 1px solid var(--hair);
    background: var(--glass);
    backdrop-filter: blur(18px);
    -webkit-backdrop-filter: blur(18px);
    user-select: none;
  }
  .lead {
    display: flex;
    align-items: center;
    gap: 28px;
    padding: 18px 0;
    font-size: 13px;
    min-width: 0;
  }
  .drag {
    flex: 1;
    min-width: 32px;
    align-self: stretch;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--ink);
    flex-shrink: 0;
  }
  .brand-icon {
    display: block;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    object-fit: cover;
    background: #000;
  }
  .brand-copy {
    display: flex;
    flex-direction: column;
    line-height: 1.05;
  }
  .brand-name {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: 0.28em;
  }
  .brand-by {
    font-style: normal;
    font-size: 9px;
    letter-spacing: 0.18em;
    color: var(--muted);
    text-transform: lowercase;
  }
  .proj { position: relative; }
  .proj-key {
    background: none;
    border: 0;
    padding: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
    max-width: 240px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proj-key:hover, .proj-key[aria-expanded="true"] {
    background: var(--panel);
  }
  .proj-menu {
    position: absolute;
    top: calc(100% + 10px);
    left: 0;
    z-index: 8;
    min-width: 280px;
    max-width: 440px;
    max-height: 320px;
    overflow: auto;
    padding: 10px;
    border: 1px solid var(--hair);
    border-radius: 12px;
    background: var(--key);
    box-shadow: 0 16px 40px color-mix(in srgb, var(--void) 18%, transparent);
  }
  .proj-menu button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    color: var(--ink);
    border-radius: 8px;
    padding: 12px 14px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .proj-menu button:hover, .proj-menu button.here { background: var(--panel); }
  .proj-name { font-size: 13px; font-weight: 600; }
  .proj-path {
    font-size: 11px;
    color: var(--muted);
    max-width: 400px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--ink);
    opacity: 0.8;
  }
  .chip em {
    font-style: normal;
    color: var(--muted);
    margin-right: 8px;
    font-size: 11px;
  }
  .truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 200px; }
  .meta {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 0 8px;
  }
  .led {
    font-size: 11px;
    letter-spacing: 0.08em;
    font-weight: 650;
  }
  .led.on { color: var(--petri-lcd); }
  .led.off { color: var(--petri-record); }
  .theme-key {
    min-width: 48px;
    height: 36px;
    border: 1px solid var(--hair);
    background: transparent;
    border-radius: 8px;
    font-size: 11px;
    letter-spacing: 0.08em;
    font-weight: 650;
    color: var(--ink);
  }
  .win {
    display: flex;
    align-self: stretch;
  }
  .win button {
    width: 48px;
    color: var(--muted);
    background: transparent;
    border: 0;
  }
  .win button:hover { background: var(--panel); color: var(--ink); }
  .win .close:hover { background: var(--hot); color: var(--hot-ink); }

  .views {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 12px var(--gutter) 0;
    flex-shrink: 0;
  }
  .views button {
    background: none;
    border: 0;
    color: var(--muted);
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
  }
  .views button.here {
    color: var(--ink);
    background: var(--panel);
    font-weight: 600;
  }
  .manage {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 0 32px;
  }
  .manage h1 {
    font-size: 22px;
    font-weight: 650;
    margin: 0 0 24px;
  }
  .cards {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .cards li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px 16px;
    padding: 16px 18px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--key) 70%, transparent);
  }
  .cards a {
    color: inherit;
    opacity: 0.75;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }
  .tag {
    font-size: 12px;
    color: var(--muted);
  }
  .sched-form {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin: 0 0 24px;
  }
  .sched-form input {
    flex: 1;
    min-width: 160px;
    height: 48px;
    border-radius: 12px;
    border: 1px solid var(--hair);
    background: color-mix(in srgb, var(--key) 88%, transparent);
    color: var(--ink);
    padding: 0 16px;
    font-size: 14px;
  }
  .sched-add { min-width: 88px; height: 48px; }

  .stage {
    flex: 1;
    display: flex;
    flex-direction: column;
    max-width: var(--page);
    width: 100%;
    margin: 0 auto;
    padding: 32px var(--gutter) 16px;
    gap: 24px;
    min-height: 0;
    position: relative;
  }
  .trail {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    color: var(--muted);
  }
  .trail button {
    background: none;
    border: 0;
    color: inherit;
    padding: 6px 4px;
  }
  .trail button.here { color: var(--ink); font-weight: 600; }
  .sep { opacity: 0.35; padding: 0 4px; }
  .child { font-size: 11px; color: var(--muted); }
  .badge {
    font-style: normal;
    display: inline-flex;
    min-width: 18px;
    height: 18px;
    padding: 0 6px;
    margin-left: 6px;
    border-radius: 99px;
    background: color-mix(in srgb, var(--ink) 10%, transparent);
    color: var(--ink);
    font-size: 11px;
    align-items: center;
    justify-content: center;
  }

  .body { flex: 1; display: flex; gap: 28px; min-height: 0; }
  .tree {
    width: 240px;
    flex-shrink: 0;
    border-radius: 16px;
    background: color-mix(in srgb, var(--key) 70%, transparent);
    padding: 20px 16px;
    overflow: auto;
  }
  .tree-h {
    font-size: 11px;
    color: var(--muted);
    margin-bottom: 14px;
  }
  .tree button {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    color: var(--ink);
    font-size: 13px;
    padding: 10px 12px;
    border-radius: 8px;
  }
  .tree button.here { background: var(--panel); }
  .tree .indent { padding-left: 24px; color: var(--muted); }
  .tree .indent2 { padding-left: 36px; }
  .tree em { font-style: normal; opacity: 0.5; }
  .empty { font-size: 13px; color: var(--muted); }

  .lcd {
    flex: 1;
    overflow: auto;
    background: var(--lcd-bg);
    color: var(--lcd-ink);
    border-radius: 20px;
    padding: 36px 40px 48px;
    font-size: 15px;
    line-height: 1.65;
  }
  .idle {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    color: var(--slate);
    letter-spacing: 0.02em;
  }
  .bubble { margin-bottom: 28px; max-width: 72%; }
  .bubble.user { margin-left: auto; }
  .role {
    font-size: 11px;
    color: var(--slate);
    margin-bottom: 8px;
  }
  .pad, .fork {
    background: var(--lcd-pad);
    padding: 16px 20px;
    border-radius: 12px;
  }
  .bubble.user .pad {
    white-space: pre-wrap;
    background: var(--lcd-user);
    color: var(--lcd-user-ink);
  }
  .md :global(.md-h) {
    font-size: 15px;
    margin: 0 0 12px;
    font-weight: 650;
  }
  .md :global(h1.md-h) { font-size: 18px; }
  .md :global(.md-p) { margin: 0 0 12px; }
  .md :global(.md-ul), .md :global(.md-ol) {
    margin: 0 0 12px;
    padding-left: 1.5em;
  }
  .md :global(.md-ul) { list-style: disc; }
  .md :global(.md-ol) { list-style: decimal; }
  .md :global(.md-ul) :global(li),
  .md :global(.md-ol) :global(li) {
    display: list-item;
    margin: 0 0 6px;
  }
  .md :global(.md-ul) :global(li)::marker,
  .md :global(.md-ol) :global(li)::marker {
    color: var(--lcd-ink);
  }
  .md :global(.md-pre) {
    margin: 12px 0;
    padding: 14px 16px;
    background: color-mix(in srgb, var(--void) 45%, transparent);
    border-radius: 10px;
    overflow-x: auto;
    white-space: pre-wrap;
  }
  .md :global(.md-code) {
    font-size: 13px;
    background: color-mix(in srgb, var(--void) 35%, transparent);
    padding: 2px 6px;
    border-radius: 5px;
  }
  .md :global(.md-a) { color: var(--lcd-ink); text-decoration: underline; }
  .md :global(em) { font-style: italic; }
  .md :global(strong) { font-weight: 650; }
  .bubble.system .pad {
    background: color-mix(in srgb, var(--lcd-ink) 8%, var(--lcd-pad));
  }
  .fork-row { display: flex; gap: 12px; align-items: flex-start; }
  .fork-main { flex: 1; text-align: left; background: none; border: 0; color: inherit; }
  .id { color: var(--lcd-ink); opacity: 0.8; }
  .st.done { color: var(--lcd-ink); opacity: 0.7; }
  .st.failed { color: var(--lcd-ink); opacity: 0.5; }
  .st.running { color: var(--lcd-ink); opacity: 0.85; }
  .key {
    border: 1px solid color-mix(in srgb, var(--lcd-ink) 22%, transparent);
    background: transparent;
    color: var(--lcd-ink);
    border-radius: 8px;
    padding: 8px 14px;
    font-size: 11px;
  }
  .result {
    margin: 16px 0 0;
    padding-top: 16px;
    border-top: 1px solid color-mix(in srgb, var(--lcd-ink) 14%, transparent);
    opacity: 0.85;
    white-space: pre-wrap;
    font: inherit;
  }
  .muted { opacity: 0.5; font-size: 13px; }

  .dock {
    flex-shrink: 0;
    z-index: 4;
    padding: 16px var(--gutter) calc(28px + env(safe-area-inset-bottom, 0px));
    background: color-mix(in srgb, var(--shell) 82%, transparent);
  }
  .dock-row {
    max-width: var(--page);
    width: 100%;
    margin: 0 auto;
    display: flex;
    gap: 12px;
  }
  .dock input {
    flex: 1;
    height: 56px;
    border-radius: 12px;
    border: 1px solid var(--hair);
    background: color-mix(in srgb, var(--key) 88%, transparent);
    color: var(--ink);
    padding: 0 20px;
    font-size: 16px;
    outline: none;
  }
  .dock input:focus { border-color: var(--slate); }
  .send {
    min-width: 108px;
    height: 56px;
    border: 0;
    border-radius: 12px;
    background: var(--hot);
    color: var(--hot-ink);
    font-size: 14px;
    font-weight: 650;
    letter-spacing: 0.06em;
  }
  .send:hover { filter: brightness(1.05); }

  .ctx-menu {
    position: fixed;
    z-index: 20;
    min-width: 220px;
    max-width: 300px;
    padding: 8px;
    border: 1px solid var(--hair);
    border-radius: 12px;
    background: var(--key);
    color: var(--ink);
    box-shadow: 0 16px 40px color-mix(in srgb, var(--void) 22%, transparent);
  }
  .ctx-menu header {
    font-size: 11px;
    color: var(--muted);
    padding: 8px 12px 6px;
  }
  .ctx-menu button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    color: var(--ink);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 13px;
  }
  .ctx-menu button:hover, .ctx-menu button:focus {
    background: var(--panel);
    outline: none;
  }
  .ctx-menu button:disabled { opacity: 0.4; }
  .ctx-mark { color: var(--ink); font-size: 8px; }
  .desk {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 12px;
    padding: 12px;
    overflow: auto;
  }
  .pane {
    flex: 1 1 320px;
    min-width: 260px;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: 16px;
    overflow: auto;
  }
  .chat-pane { min-width: 360px; }
  .gate {
    position: relative;
    z-index: 2;
    min-height: 100dvh;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .gate-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 36px 40px;
    border-radius: 20px;
    max-width: 420px;
    text-align: center;
  }
  .gate-card input {
    width: 100%;
    height: 44px;
    border-radius: 10px;
    border: 1px solid var(--hair);
    background: transparent;
    color: var(--ink);
    padding: 0 12px;
  }
  .prompt-box {
    width: 100%;
    min-height: 120px;
    border-radius: 12px;
    border: 1px solid var(--hair);
    background: transparent;
    color: var(--ink);
    padding: 12px;
  }
  .sysmin-hex {
    position: fixed;
    left: 16px;
    bottom: 16px;
    width: 18px;
    height: 20px;
    z-index: 12;
    background: var(--ice);
    clip-path: polygon(50% 0, 100% 25%, 100% 75%, 50% 100%, 0 75%, 0 25%);
    border: 0;
    padding: 0;
  }
  :global(html[data-theme="dark"]) .sysmin-hex { background: #fff; }
  .mesh-pop {
    position: fixed;
    left: 16px;
    bottom: 44px;
    z-index: 12;
    width: 280px;
    max-height: 50vh;
    overflow: auto;
    padding: 16px;
    border-radius: 16px;
  }
  .ctx-sep {
    height: 1px;
    margin: 6px 8px;
    background: var(--hair);
  }

  .lcd::-webkit-scrollbar, .tree::-webkit-scrollbar { width: 8px; }
  .lcd::-webkit-scrollbar-thumb { background: color-mix(in srgb, var(--lcd-ink) 22%, transparent); border-radius: 99px; }
  .tree::-webkit-scrollbar-thumb { background: var(--hair); border-radius: 99px; }

  @media (max-width: 820px) {
    .chrome { padding-left: 20px; gap: 12px; min-height: 64px; }
    .lead { gap: 16px; }
    .drag { display: none; }
    .lead > .chip { display: none; }
    .meta .led { display: none; }
    .proj-key { max-width: 38vw; }
    .stage, .dock, .views { padding-left: 20px; padding-right: 20px; }
    .stage { padding-top: 20px; gap: 16px; }
    .lcd { padding: 24px 22px 32px; border-radius: 16px; }
    .bubble { max-width: 88%; }
  }
  @media (max-width: 520px) {
    .win button:not(.close) { display: none; }
  }
</style>
