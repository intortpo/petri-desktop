<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import SilkBg from './lib/SilkBg.svelte';
  import petriIcon from './assets/petri-icon.png';
  import petriLogo from './assets/petri-logo.png';
  import { applyTheme, parseThemeArg, readPref, type ThemePref } from './lib/theme';
  import { DEFAULT_PACK_ID, PACKS } from './lib/palettes';
  import { DEFAULT_SYSTEM_PROMPT, isSysmin } from './lib/persona';
  import { recallPersona, rememberTurn } from './lib/reasoningbank';
  import {
    checkEverosHealth,
    addEverosTurn,
    flushEveros,
    searchEveros,
    rememberEveros,
    getEverosProfile,
    formatEverosRecall,
  } from './lib/everos';
  import {
    WIN_IDS,
    WIN_LABEL,
    WIN_ICONS,
    WIN_DESCRIPTIONS,
    allowedWins,
    activateTab,
    closeTab,
    cycleLayout,
    defaultOpen,
    ensureWin,
    isDesktop,
    toggleWin,
    type WinId,
    type LayoutMode,
  } from './lib/windows';
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
    FIXTURE_PROJECTS,
    FIXTURE_REPOS,
    FIXTURE_USERS,
    ScheduleBook,
    cancelSchedule,
    createSchedule,
    filterProjects,
    filterUsers,
    listProjects,
    listRepos,
    listSchedules,
    listUsers,
    type ListResult,
    type ProjectRecord,
    type RepoRecord,
    type Schedule,
    type UserRecord,
  } from './lib/manage';
  import {
    PAGE_TO_WIN,
    applySlashPick,
    extraSlashItems,
    filterSlash,
    type SlashItem,
  } from './lib/slash';
  import {
    MODEL_CHOICES,
    VALID_MODES,
    isValidMode,
    runDesignCommand,
  } from './lib/architect';

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
  let showModes = false;
  let showModels = false;
  let projectList: string[] = [];
  let projRoot: HTMLDivElement | null = null;
  let modeRoot: HTMLDivElement | null = null;
  let modelRoot: HTMLDivElement | null = null;
  let view: "chat" | "repos" | "users" | "schedule" = "chat";
  let openWins: WinId[] = defaultOpen(typeof window !== "undefined" ? isDesktop(window.innerWidth) : true);
  let activeWin: WinId = "chat";
  let splitWin: WinId = "workspace";
  let splitOpen = false;
  let layoutMode: LayoutMode = "tabs";
  let addTabMenuOpen = false;
  let repoFilter = "";
  let workspaceFilter = "";
  let session: { login: string; email: string } | null = null;
  let gateNotice = "";
  let patDraft = "";
  let deviceLogin: { user_code: string; verification_uri: string } | null = null;
  let meshOpen = false;
  let cores: { id: string; online: boolean }[] = [];
  let meshPeers: { name: string; address: string; online: boolean }[] = [];
  let audit: { kind: string; detail: string; created_at?: string }[] = [];
  let systemPrompt = DEFAULT_SYSTEM_PROMPT;
  $: isSysminUser = isSysmin(session?.login, session?.email);
  $: visibleWinIds = allowedWins(isSysminUser);
  $: filteredRepos = repoResult.items.filter(
    (r) => !repoFilter || r.name.toLowerCase().includes(repoFilter.toLowerCase())
  );
  $: filteredTree = workspaceTree.filter(
    (f) => !workspaceFilter || f.name.toLowerCase().includes(workspaceFilter.toLowerCase())
  );
  let branchThreadId = "";
  let branchMessages: any[] = [];
  let workspaceTree: { name: string; path: string; kind: string }[] = [];
  let repoResult: ListResult<RepoRecord> = listRepos([]);
  let userResult: ListResult<UserRecord> = listUsers([]);

  // Project Management state
  let dbProjects: ProjectRecord[] = [];
  let projectSearch = "";
  let projectStatusFilter = "all";
  let showNewProjectModal = false;
  let newProjectName = "";
  let newProjectKey = "";
  let newProjectDesc = "";
  let newProjectPath = "";
  let newProjectBranch = "main";
  let projectNotice = "";

  $: filteredDbProjects = filterProjects(dbProjects, projectSearch, projectStatusFilter);
  $: projectStatusCounts = {
    total: dbProjects.length,
    active: dbProjects.filter((p) => p.status === "active").length,
    archived: dbProjects.filter((p) => p.status === "archived").length,
  };

  // User Management state
  let userSearch = "";
  let userRoleFilter = "all";
  let userStatusFilter = "all";
  let showNewUserModal = false;
  let newUserLogin = "";
  let newUserName = "";
  let newUserEmail = "";
  let newUserRole = "member";
  let newUserStatus = "active";
  let userNotice = "";

  $: filteredDbUsers = filterUsers(userResult.items, userSearch, userRoleFilter, userStatusFilter);
  $: userRoleCounts = {
    total: userResult.items.length,
    sysmin: userResult.items.filter((u) => u.role === "sysmin").length,
    admin: userResult.items.filter((u) => u.role === "admin").length,
    member: userResult.items.filter((u) => u.role === "member").length,
    viewer: userResult.items.filter((u) => u.role === "viewer").length,
  };

  let scheduleBook = new ScheduleBook();
  let schedules: Schedule[] = [];
  let schedTitle = "";
  let schedWhen = "";
  let schedInterval = "";
  let schedNotice = "";
  let composerEl: HTMLInputElement | null = null;
  let extraSlash: SlashItem[] = [];
  let slashIndex = 0;
  $: slashHits = filterSlash(composerInput, extraSlash);
  $: if (slashIndex >= slashHits.length) slashIndex = 0;
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

  function toggleModes() {
    showModes = !showModes;
    showModels = false;
    showProjects = false;
  }

  function toggleModels() {
    showModels = !showModels;
    showModes = false;
    showProjects = false;
  }

  async function selectMode(mode: string) {
    showModes = false;
    if (!isValidMode(mode) || mode === currentMode) return;
    await handleCommand(`/mode ${mode}`);
  }

  function selectModel(model: string) {
    showModels = false;
    const next = String(model || "").trim();
    if (!next) return;
    currentModel = next;
    if (currentThread) currentThread = { ...currentThread, model: next };
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

  async function refreshProjects() {
    try {
      const rows = await invoke<ProjectRecord[]>("list_db_projects");
      const res = listProjects(rows);
      if (res.ok && res.items.length) {
        dbProjects = res.items;
      } else {
        dbProjects = listProjects(FIXTURE_PROJECTS).items;
      }
    } catch {
      dbProjects = listProjects(FIXTURE_PROJECTS).items;
    }
  }

  async function handleCreateProject() {
    if (!newProjectName.trim()) {
      projectNotice = "Project name is required.";
      return;
    }
    const path = newProjectPath.trim() || currentProject;
    const key = newProjectKey.trim() || newProjectName.trim().toLowerCase().replace(/\s+/g, "-");
    try {
      await invoke("create_db_project", {
        name: newProjectName.trim(),
        key,
        description: newProjectDesc.trim() || null,
        path,
        defaultBranch: newProjectBranch.trim() || "main",
      });
      projectNotice = "Project created successfully.";
      showNewProjectModal = false;
      newProjectName = "";
      newProjectKey = "";
      newProjectDesc = "";
      newProjectPath = "";
      newProjectBranch = "main";
      await refreshProjects();
      await loadProjects();
    } catch (e) {
      projectNotice = `Failed to create project: ${e}`;
    }
  }

  async function handleArchiveProject(id: string) {
    try {
      await invoke("archive_db_project", { id });
      await refreshProjects();
    } catch (e) {
      projectNotice = `Failed to archive project: ${e}`;
    }
  }

  async function handleDeleteProject(id: string) {
    try {
      await invoke("delete_db_project", { id });
      await refreshProjects();
      await loadProjects();
    } catch (e) {
      projectNotice = `Failed to delete project: ${e}`;
    }
  }

  async function activateDbProject(proj: ProjectRecord) {
    currentProject = proj.root_path;
    if (proj.default_mode && isValidMode(proj.default_mode)) {
      currentMode = proj.default_mode;
    }
    if (proj.default_model) {
      currentModel = proj.default_model;
    }
    await loadTree();
    activateWindow("chat");
    system(`Switched to project ${proj.name} (${proj.root_path})`);
  }

  async function refreshUsers() {
    try {
      const rows = await invoke<UserRecord[]>("list_db_users");
      const res = listUsers(rows);
      if (res.ok && res.items.length) {
        userResult = res;
        return;
      }
    } catch {}

    try {
      const me = await githubJson("/user");
      const rec = me && typeof me === "object" ? (me as { login?: string; email?: string }) : {};
      const rows = rec.login
        ? [{
            login: rec.login,
            role: isSysmin(rec.login, rec.email) ? "sysmin" : "member",
            status: "active",
          }]
        : [];
      userResult = listUsers(rows);
      if (userResult.ok && userResult.items.length) return;
    } catch {}
    userResult = listUsers(FIXTURE_USERS);
  }

  async function handleCreateUser() {
    if (!newUserLogin.trim()) {
      userNotice = "Username / login is required.";
      return;
    }
    try {
      await invoke("create_db_user", {
        login: newUserLogin.trim(),
        displayName: newUserName.trim() || null,
        email: newUserEmail.trim() || null,
        role: newUserRole,
        status: newUserStatus,
      });
      userNotice = "User created successfully.";
      showNewUserModal = false;
      newUserLogin = "";
      newUserName = "";
      newUserEmail = "";
      newUserRole = "member";
      newUserStatus = "active";
      await refreshUsers();
    } catch (e) {
      userNotice = `Failed to create user: ${e}`;
    }
  }

  async function handleUpdateUserRole(user: UserRecord, role: string) {
    try {
      await invoke("update_db_user", {
        id: user.id || `u-${user.login}`,
        role,
        status: user.status,
      });
      await refreshUsers();
    } catch (e) {
      userNotice = `Failed to update role: ${e}`;
    }
  }

  async function handleUpdateUserStatus(user: UserRecord, status: string) {
    try {
      await invoke("update_db_user", {
        id: user.id || `u-${user.login}`,
        role: user.role,
        status,
      });
      await refreshUsers();
    } catch (e) {
      userNotice = `Failed to update status: ${e}`;
    }
  }

  async function handleDeleteUser(user: UserRecord) {
    if (user.role === "sysmin" && userRoleCounts.sysmin <= 1) {
      userNotice = "Cannot delete the last sysmin user.";
      return;
    }
    try {
      await invoke("delete_db_user", { id: user.id || `u-${user.login}` });
      await refreshUsers();
    } catch (e) {
      userNotice = `Failed to delete user: ${e}`;
    }
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
    refreshProjects();
    refreshUsers();
    setupListeners();
    openWins = defaultOpen(isDesktop(window.innerWidth));

    const onPtr = (e: PointerEvent) => {
      const t = e.target as Node | null;
      if (showProjects && projRoot && t && !projRoot.contains(t)) showProjects = false;
      if (showModes && modeRoot && t && !modeRoot.contains(t)) showModes = false;
      if (showModels && modelRoot && t && !modelRoot.contains(t)) showModels = false;
      if (ctxMenu && ctxRoot && t && !ctxRoot.contains(t)) ctxMenu = null;
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        showProjects = false;
        showModes = false;
        showModels = false;
        ctxMenu = null;
        addTabMenuOpen = false;
      }
      if (e.altKey && !e.ctrlKey && !e.metaKey) {
        const num = parseInt(e.key, 10);
        if (num >= 1 && num <= visibleWinIds.length) {
          e.preventDefault();
          activateWindow(visibleWinIds[num - 1]);
        } else if (e.key.toLowerCase() === "s" || e.key.toLowerCase() === "l") {
          e.preventDefault();
          toggleSplit();
        }
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
    await refreshSlashExtras();
  }

  async function refreshSlashExtras() {
    let skills: string[] = [];
    let mcp: string[] = [];
    try {
      skills = await invoke<string[]>("list_skills", { projectPath: currentProject });
    } catch {}
    try {
      const listed = await invoke<string>("list_mcp", { projectPath: currentProject });
      mcp = (listed.match(/^- (.+)$/gm) || []).map((line) => line.replace(/^- /, "").trim());
    } catch {}
    extraSlash = extraSlashItems({
      skills,
      mcp,
      plugins: Object.keys(moduleCommands),
    });
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

  async function handleLoginClick() {
    gateNotice = "";
    try {
      const pat = await invoke<string>("get_github_pat");
      if (pat?.trim()) {
        const msg = await invoke<string>("verify_github_pat", { pat: pat.trim() });
        const login = (msg.match(/as\s+(\S+)/i) || [])[1] || "user";
        session = { login, email: "" };
        await invoke("save_github_session", { login, email: "" }).catch(() => {});
        await loadModules();
        await handleCommand("/new");
        return;
      }
    } catch {}
    await startGithubDevice();
  }

  function handleGatePaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData("text")?.trim();
    if (text && (text.startsWith("ghp_") || text.startsWith("github_pat_") || text.length >= 35)) {
      patDraft = text;
      loginWithPat();
    }
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
      deviceLogin = await invoke<{
        device_code: string;
        user_code: string;
        verification_uri: string;
      }>("github_device_start");
      if (deviceLogin?.verification_uri) {
        try {
          await invoke("plugin:opener|open_url", { url: deviceLogin.verification_uri });
        } catch {
          window.open(deviceLogin.verification_uri, "_blank");
        }
      }
    } catch (e) {
      gateNotice = String(e);
    }
  }

  function setPack(id: string) {
    packId = id;
    themeResolved = applyTheme(themePref, id);
  }

  function activateWindow(id: WinId) {
    if (!visibleWinIds.includes(id)) return;
    activeWin = id;
    if (!openWins.includes(id)) {
      openWins = [...openWins, id];
    }
    if (id === "projects") refreshProjects();
    if (id === "repos" as never || id === "github") refreshRepos();
    if (id === "users") refreshUsers();
    if (id === "workspace") loadTree();
    if (id === "schedule") openView("schedule");
  }

  function closeWindow(id: WinId) {
    const res = closeTab(openWins, activeWin, id);
    openWins = res.open;
    activeWin = res.active;
    if (splitWin === id) {
      splitWin = openWins.find((w) => w !== "chat") || "workspace";
    }
  }

  function toggleLayoutMode() {
    toggleSplit();
  }

  function toggleSplit() {
    splitOpen = !splitOpen;
  }

  function openAllApps() {
    if (isSysminUser) {
      openWins = [...WIN_IDS];
      splitOpen = true;
    }
  }

  function toggleWindow(id: WinId) {
    activateWindow(id);
  }

  function isPaneVisible(id: WinId): boolean {
    if (id === activeWin) return true;
    if (splitOpen && id === "chat") return true;
    return false;
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
      meshPeers = await invoke("list_mesh_peers");
    } catch {
      meshPeers = [];
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
        await rememberEveros(lesson, "petri");
        system("Lesson remembered (Mesh & EverOS).");
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
    } else if (cmd === "/plugin") {
      system("Usage: /plugin install <url>");
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
      if (isValidMode(mode)) {
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
        system(`Unknown mode. Valid modes: ${VALID_MODES.join(", ")}`);
      }
    } else if (cmd.startsWith("/rlm ") || cmd === "/rlm") {
      const task = cmd.replace("/rlm", "").trim() || "idle";
      try {
        const h = await invoke<{ id: string; status: string; task: string }>("rlm_spawn", { task });
        system(`rlm ${h.id.slice(0, 8)} ${h.status} · ${h.task}`);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd.startsWith("/refine ")) {
      const rest = cmd.slice(8).trim();
      const sp = rest.indexOf(" ");
      const kind = sp > 0 ? rest.slice(0, sp) : "skill";
      const body = sp > 0 ? rest.slice(sp + 1) : rest;
      try {
        const path = await invoke<string>("apply_refine_cmd", {
          kind,
          evidence: body.slice(0, 180),
          body,
        });
        system(`refine wrote ${path}`);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/everos" || cmd === "/everos status") {
      try {
        const health = await checkEverosHealth();
        system(`EverOS Memory Status:\n• Status: ${health.status}\n• Engine: ${health.engine}\n• Storage Root: ${health.storage_root}\n• Memory Count: ${health.memory_count}\n• Capabilities: Markdown-native (${health.capabilities.markdown_native}), FTS (${health.capabilities.fts_search})`);
      } catch (e) {
        system(`Error checking EverOS: ${e}`);
      }
    } else if (cmd.startsWith("/everos search ")) {
      const query = cmd.replace("/everos search ", "").trim();
      try {
        const hits = await searchEveros(query, 5);
        system(`EverOS Search Results for "${query}":\n${formatEverosRecall(hits)}`);
      } catch (e) {
        system(`Error searching EverOS: ${e}`);
      }
    } else if (cmd === "/everos flush") {
      try {
        const res = await flushEveros(currentThreadId || "default");
        system(`EverOS Flush: ${res.flushed ? 'Success' : 'No turns pending'}\n${res.episode_path ? 'Episode: ' + res.episode_path : ''}`);
      } catch (e) {
        system(`Error flushing EverOS: ${e}`);
      }
    } else if (cmd.startsWith("/everos add ")) {
      const fact = cmd.replace("/everos add ", "").trim();
      try {
        await rememberEveros(fact, "user");
        system(`Added memory fact to EverOS: ${fact}`);
      } catch (e) {
        system(`Error adding EverOS memory: ${e}`);
      }
    } else if (cmd === "/everos profile") {
      try {
        const prof = await getEverosProfile();
        system(`EverOS Profile:\n${prof}`);
      } catch (e) {
        system(`Error getting EverOS profile: ${e}`);
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
    } else if (cmd === "/plan" || cmd.startsWith("/plan ")) {
      const arg = cmd === "/plan" ? projectName(currentProject) : cmd.replace("/plan ", "").trim();
      system(runDesignCommand("/plan", arg || projectName(currentProject)) || "");
    } else if (cmd === "/grill-me" || cmd.startsWith("/grill-me ")) {
      const arg = cmd === "/grill-me" ? projectName(currentProject) : cmd.replace("/grill-me ", "").trim();
      system(runDesignCommand("/grill-me", arg) || "");
    } else if (cmd === "/adr" || cmd.startsWith("/adr ")) {
      const arg = cmd === "/adr" ? projectName(currentProject) : cmd.replace("/adr ", "").trim();
      system(runDesignCommand("/adr", arg) || "");
    } else if (cmd === "/context" || cmd.startsWith("/context ")) {
      const arg = cmd === "/context" ? "" : cmd.replace("/context ", "").trim();
      system(runDesignCommand("/context", arg) || "");
    } else if (cmd === "/stop") {
      try {
        const result = await invoke<string>("stop_turn", { threadId: currentThreadId });
        system(result);
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (cmd === "/mcp" || cmd.startsWith("/mcp ")) {
      try {
        const result = await invoke<string>("list_mcp", { projectPath: currentProject });
        const want = cmd.slice(5).trim();
        if (!want) {
          system(result);
        } else {
          const lines = result.split("\n").filter((l) => l.toLowerCase().includes(want.toLowerCase()));
          system(lines.length ? lines.join("\n") : `${result}\n(no server matching ${want})`);
        }
      } catch (e) {
        system(`Error: ${e}`);
      }
    } else if (PAGE_TO_WIN[cmd]) {
      const id = PAGE_TO_WIN[cmd] as WinId;
      if (!visibleWinIds.includes(id)) {
        system(`Access restricted: ${id} requires sysmin privileges.`);
        return;
      }
      openWins = ensureWin(openWins, id);
      activeWin = id;
      if (id === "projects") await refreshProjects();
      else if (id === "github") await openView("repos");
      else if (id === "users") await openView("users");
      else if (id === "schedule") await openView("schedule");
      else if (id === "workspace") await loadTree();
    } else if (cmd === "/apps") {
      if (isSysminUser) {
        openWins = [...WIN_IDS];
        layoutMode = "grid";
        system(`Opened all apps (${WIN_IDS.length}): ${WIN_IDS.map((w) => WIN_LABEL[w]).join(", ")}`);
      } else {
        openWins = [...visibleWinIds];
        system(`Opened member apps: ${openWins.map((w) => WIN_LABEL[w]).join(", ")}`);
      }
    } else if (cmd === "/layout" || cmd.startsWith("/layout ")) {
      const mode = cmd.replace("/layout", "").trim() as LayoutMode;
      if (mode === "tabs" || mode === "split" || mode === "grid") {
        layoutMode = mode;
        system(`Layout switched to ${mode} mode.`);
      } else {
        layoutMode = cycleLayout(layoutMode);
        system(`Layout mode: ${layoutMode}`);
      }
    } else if (cmd === "/tab next") {
      const idx = openWins.indexOf(activeWin);
      const nextId = openWins[(idx + 1) % openWins.length];
      activateWindow(nextId);
      system(`Active tab: ${WIN_LABEL[nextId]}`);
    } else if (cmd === "/tab prev") {
      const idx = openWins.indexOf(activeWin);
      const prevId = openWins[(idx - 1 + openWins.length) % openWins.length];
      activateWindow(prevId);
      system(`Active tab: ${WIN_LABEL[prevId]}`);
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
    if (currentThreadId) {
      addEverosTurn(currentThreadId, "user", text).catch(() => {});
    }

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

  function pickSlash(item: SlashItem) {
    composerInput = applySlashPick(item);
    slashIndex = 0;
    tick().then(() => {
      composerEl?.focus();
      const pos = composerInput.length;
      composerEl?.setSelectionRange(pos, pos);
    });
  }

  function onComposerKey(e: KeyboardEvent) {
    if (!slashHits.length) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      slashIndex = (slashIndex + 1) % slashHits.length;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      slashIndex = (slashIndex - 1 + slashHits.length) % slashHits.length;
    } else if (e.key === "Tab" || e.key === "Enter") {
      e.preventDefault();
      pickSlash(slashHits[slashIndex]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      composerInput = "";
    }
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
<div class="gate" on:paste={handleGatePaste}>
  <div class="glass gate-card">
    <div class="gate-card-bg" aria-hidden="true">
      <img src={petriLogo} alt="" />
    </div>

    <div class="gate-hex-hero">
      <img class="gate-hive-hero" src={petriIcon} alt="HiVE" width="190" height="190" />
    </div>

    <button type="button" class="gate-login-btn" on:click={handleLoginClick} aria-label="Login">
      <svg class="gh-icon" viewBox="0 0 24 24" width="28" height="28" fill="currentColor" aria-hidden="true">
        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
      </svg>
      <span>Login</span>
    </button>

    {#if deviceLogin}
      <div class="device-code-chip" title="GitHub Device Code">{deviceLogin.user_code}</div>
    {/if}
    {#if gateNotice}
      <p class="empty gate-err">{gateNotice}</p>
      <input
        class="gate-pat-fallback"
        bind:value={patDraft}
        on:keydown={(e) => e.key === 'Enter' && loginWithPat()}
        placeholder="Paste PAT and press Enter"
        aria-label="GitHub PAT"
      />
    {/if}
  </div>
</div>
{:else}
<div class="shell dash-shell" role="application" on:contextmenu={onContext}>
  <!-- LEFT SIDEBAR -->
  <aside class="dash-sidebar glass" aria-label="Dashboard Sidebar">
    <div class="sidebar-top">
      <div class="brand" title="HIVE by petri">
        <div class="brand-hex">
          <img class="brand-icon" src={petriIcon} alt="" width="36" height="40" />
        </div>
        <span class="brand-copy">
          <strong class="brand-name">HIVE</strong>
          <em class="brand-by">by petri</em>
        </span>
      </div>

      <div class="proj sidebar-proj" data-ctx="project" bind:this={projRoot}>
        <button
          type="button"
          class="chip proj-key"
          aria-expanded={showProjects}
          aria-haspopup="listbox"
          on:click={toggleProjects}
          title="Project: {projectName(currentProject)}"
        >
          <em>proj</em> <span class="proj-label">{projectName(currentProject)}</span>
        </button>
        {#if showProjects}
          <div class="proj-menu glass" role="listbox" aria-label="Projects">
            <header>RECENT PROJECTS</header>
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
    </div>

    <!-- Navigation Section -->
    <nav class="dash-nav" aria-label="Cockpit Navigation">
      <div class="nav-section-title">COCKPIT</div>
      {#each visibleWinIds as id}
        <button
          type="button"
          class="dash-nav-btn"
          class:active={activeWin === id}
          on:click={() => activateWindow(id)}
          title="{WIN_LABEL[id]} — {WIN_DESCRIPTIONS[id]}"
        >
          <svg class="nav-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <path fill="currentColor" d={WIN_ICONS[id]} />
          </svg>
          <span class="nav-text">{WIN_LABEL[id]}</span>
          {#if id === "github" && repoResult.items.length > 0}
            <span class="nav-count">{repoResult.items.length}</span>
          {:else if id === "projects" && dbProjects.length > 0}
            <span class="nav-count">{dbProjects.length}</span>
          {:else if id === "workspace" && workspaceTree.length > 0}
            <span class="nav-count">{workspaceTree.length}</span>
          {:else if id === "schedule" && schedules.length > 0}
            <span class="nav-count">{schedules.length}</span>
          {:else if id === "users" && userResult.items.length > 0}
            <span class="nav-count">{userResult.items.length}</span>
          {/if}
        </button>
      {/each}
    </nav>

    <!-- Sidebar Footer -->
    <div class="sidebar-footer">
      <div class="meta">
        <span data-ctx="auth" class="led {authStatus.includes('Unauthenticated') ? 'off' : 'on'}">{authStatus.includes('Unauthenticated') ? 'OFF' : 'OK'}</span>
        <span data-ctx="mesh" class="led {tailscaleStatus.includes('Offline') ? 'off' : 'on'}">MESH</span>
        <button type="button" class="theme-key" data-ctx="theme" on:click={cycleTheme} title="Theme">{themePref === 'system' ? 'SYS' : themePref === 'light' ? 'DAY' : 'NITE'}</button>
      </div>
      {#if session}
        <div class="sidebar-user" title="{session.login} ({isSysminUser ? 'sysmin' : 'member'})">
          <span class="user-avatar">{session.login.charAt(0).toUpperCase()}</span>
          <div class="user-info">
            <span class="user-login">{session.login}</span>
            <span class="user-badge">{isSysminUser ? "sysmin" : "member"}</span>
          </div>
        </div>
      {/if}
    </div>
  </aside>

  <!-- MAIN DASHBOARD CONTENT AREA -->
  <div class="dash-main">
    <!-- DASHBOARD TOPBAR -->
    <header class="chrome dash-topbar">
      <div class="lead">
        <h2 class="view-title">{WIN_LABEL[activeWin] || "Cockpit"}</h2>
        <div class="proj" data-ctx="mode" bind:this={modeRoot}>
          <button
            type="button"
            class="chip proj-key"
            aria-expanded={showModes}
            aria-haspopup="listbox"
            on:click={toggleModes}
          >
            <em>mode</em> {currentMode}
          </button>
          {#if showModes}
            <div class="proj-menu glass" role="listbox" aria-label="Modes">
              {#each VALID_MODES as mode}
                <button
                  type="button"
                  role="option"
                  aria-selected={mode === currentMode}
                  class:here={mode === currentMode}
                  on:click={() => selectMode(mode)}
                >
                  <span class="proj-name">{mode}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="proj" data-ctx="model" bind:this={modelRoot}>
          <button
            type="button"
            class="chip proj-key truncate"
            aria-expanded={showModels}
            aria-haspopup="listbox"
            on:click={toggleModels}
          >
            <em>model</em> {currentModel}
          </button>
          {#if showModels}
            <div class="proj-menu glass" role="listbox" aria-label="Models">
              {#each MODEL_CHOICES as model}
                <button
                  type="button"
                  role="option"
                  aria-selected={model === currentModel}
                  class:here={model === currentModel}
                  on:click={() => selectModel(model)}
                >
                  <span class="proj-name">{model}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <div data-tauri-drag-region class="drag"></div>

      <div class="topbar-actions">
        {#if activeWin !== "chat"}
          <button
            type="button"
            class="split-toggle-btn"
            class:here={splitOpen}
            title={splitOpen ? "Close split chat pane" : "Split with Chat side-by-side"}
            on:click={toggleSplit}
          >
            <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
              <rect x="2" y="3" width="12" height="10" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" />
              <line x1="8" y1="3" x2="8" y2="13" stroke="currentColor" stroke-width="1.2" />
            </svg>
            <span class="split-label">{splitOpen ? "Single" : "Split Chat"}</span>
          </button>
        {/if}

        <div class="win">
          <button type="button" on:click={() => windowAction("min")} aria-label="Minimize">–</button>
          <button type="button" on:click={() => windowAction("max")} aria-label="Maximize">□</button>
          <button type="button" class="close" on:click={() => windowAction("close")} aria-label="Close">×</button>
        </div>
      </div>
    </header>

    <!-- DASHBOARD VIEWPORT -->
    <main class="dash-viewport" class:has-split={splitOpen && activeWin !== "chat"}>
      {#if activeWin !== "chat"}
        <section class="dash-page active-page" data-view={activeWin}>
          {#if activeWin === "projects"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["projects"]} />
                </svg>
                <h1>Projects</h1>
                <span class="pane-badge">{dbProjects.length}</span>
              </div>
              <div class="pane-actions">
                <input
                  type="text"
                  class="pane-filter"
                  placeholder="Filter projects…"
                  bind:value={projectSearch}
                  aria-label="Filter projects"
                />
                <select class="pane-select" bind:value={projectStatusFilter} aria-label="Project status filter">
                  <option value="all">All statuses</option>
                  <option value="active">Active</option>
                  <option value="archived">Archived</option>
                </select>
                <button type="button" class="key action-btn" on:click={() => (showNewProjectModal = true)}>+ New Project</button>
                <button type="button" class="pane-btn" title="Refresh projects" on:click={refreshProjects}>⟳</button>
              </div>
            </header>
            <div class="pane-content">
              <!-- KPI Stats Bar -->
              <div class="kpi-bar">
                <div class="kpi-item">
                  <span class="kpi-label">TOTAL PROJECTS</span>
                  <span class="kpi-val">{projectStatusCounts.total}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">ACTIVE</span>
                  <span class="kpi-val text-active">{projectStatusCounts.active}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">ARCHIVED</span>
                  <span class="kpi-val">{projectStatusCounts.archived}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">WORKSPACE</span>
                  <span class="kpi-val mono-val">Primary Mesh</span>
                </div>
              </div>

              {#if projectNotice}
                <div class="notice-bar">{projectNotice}</div>
              {/if}

              <!-- Create Project Drawer / Modal -->
              {#if showNewProjectModal}
                <div class="modal-card">
                  <div class="modal-head">
                    <h3>Create New Project</h3>
                    <button type="button" class="pane-btn close" on:click={() => (showNewProjectModal = false)}>✕</button>
                  </div>
                  <form class="modal-form" on:submit|preventDefault={handleCreateProject}>
                    <div class="form-row">
                      <label>
                        <span>Project Name *</span>
                        <input type="text" bind:value={newProjectName} placeholder="e.g. Mobile Client" required />
                      </label>
                      <label>
                        <span>Key / Slug</span>
                        <input type="text" bind:value={newProjectKey} placeholder="e.g. mobile-client" />
                      </label>
                    </div>
                    <label>
                      <span>Description</span>
                      <input type="text" bind:value={newProjectDesc} placeholder="Short overview of this project" />
                    </label>
                    <div class="form-row">
                      <label>
                        <span>Root Path</span>
                        <input type="text" bind:value={newProjectPath} placeholder={currentProject} />
                      </label>
                      <label>
                        <span>Default Branch</span>
                        <input type="text" bind:value={newProjectBranch} placeholder="main" />
                      </label>
                    </div>
                    <div class="modal-foot">
                      <button type="button" class="key btn-secondary" on:click={() => (showNewProjectModal = false)}>Cancel</button>
                      <button type="submit" class="key btn-primary">Create Project</button>
                    </div>
                  </form>
                </div>
              {/if}

              {#if dbProjects.length === 0}
                <p class="empty">No projects found. Create a project to begin.</p>
              {:else if filteredDbProjects.length === 0}
                <p class="empty">No matching projects.</p>
              {:else}
                <div class="project-grid">
                  {#each filteredDbProjects as proj}
                    <div class="project-card" class:is-active={proj.root_path === currentProject}>
                      <div class="project-card-head">
                        <div class="project-title-row">
                          <span class="project-name">{proj.name}</span>
                          <span class="project-key">[{proj.slug}]</span>
                          {#if proj.root_path === currentProject}
                            <span class="badge-current">CURRENT</span>
                          {/if}
                        </div>
                        <span class="tag tag-{proj.status}">{proj.status}</span>
                      </div>

                      <p class="project-desc">{proj.description || "No description provided."}</p>

                      <div class="project-path-row">
                        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true" fill="currentColor">
                          <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/>
                        </svg>
                        <code class="project-path" title={proj.root_path}>{proj.root_path}</code>
                      </div>

                      <div class="project-metrics">
                        <span class="metric-pill" title="Connected Repositories">
                          <svg viewBox="0 0 24 24" width="13" height="13" aria-hidden="true" fill="currentColor">
                            <path d="M6 2a3 3 0 0 0-3 3v14a3 3 0 0 0 5.83 1H16a3 3 0 0 0 3-3v-4.17A3 3 0 0 0 21 10a3 3 0 0 0-3-3 3 3 0 0 0-2 2.83V14H8.83A3 3 0 0 0 6 12V5a3 3 0 0 0-3-3h3zm0 18a1 1 0 1 1 0-2 1 1 0 0 1 0 2zm12-10a1 1 0 1 1 0-2 1 1 0 0 1 0 2z"/>
                          </svg>
                          {proj.repo_count || 1} repos
                        </span>
                        <span class="metric-pill" title="Active Threads">
                          <svg viewBox="0 0 24 24" width="13" height="13" aria-hidden="true" fill="currentColor">
                            <path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H6l-2 2V4h16v12z"/>
                          </svg>
                          {proj.thread_count || 0} threads
                        </span>
                        <span class="metric-pill" title="Project Members">
                          <svg viewBox="0 0 24 24" width="13" height="13" aria-hidden="true" fill="currentColor">
                            <path d="M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5z"/>
                          </svg>
                          {proj.member_count || 1} members
                        </span>
                        <span class="metric-pill mode-pill">mode: {proj.default_mode}</span>
                      </div>

                      <div class="project-actions">
                        {#if proj.root_path !== currentProject}
                          <button type="button" class="key action-btn" on:click={() => activateDbProject(proj)}>
                            Activate
                          </button>
                        {:else}
                          <span class="key action-active">Active</span>
                        {/if}
                        {#if proj.status === "active"}
                          <button type="button" class="key-subtle" on:click={() => handleArchiveProject(proj.id)}>
                            Archive
                          </button>
                        {:else}
                          <button type="button" class="key-subtle" on:click={() => handleArchiveProject(proj.id)}>
                            Restore
                          </button>
                        {/if}
                        <button type="button" class="key-danger" title="Delete project" on:click={() => handleDeleteProject(proj.id)}>
                          Delete
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

          {:else if activeWin === "workspace"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["workspace"]} />
                </svg>
                <h1>Workspace</h1>
                <span class="pane-badge">{filteredTree.length}</span>
              </div>
              <div class="pane-actions">
                <input
                  type="text"
                  class="pane-filter"
                  placeholder="Filter files…"
                  bind:value={workspaceFilter}
                  aria-label="Filter workspace files"
                />
                <button type="button" class="pane-btn" title="Refresh files" on:click={loadTree}>⟳</button>
              </div>
            </header>
            <div class="pane-content">
              {#if workspaceTree.length === 0}
                <p class="empty">{currentProject}</p>
              {:else if filteredTree.length === 0}
                <p class="empty">No matching files.</p>
              {:else}
                <ul class="cards">
                  {#each filteredTree as node}
                    <li><strong>{node.name}</strong><span class="tag">{node.kind}</span></li>
                  {/each}
                </ul>
              {/if}
            </div>

          {:else if activeWin === "github"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["github"]} />
                </svg>
                <h1>GitHub repos</h1>
                <span class="pane-badge">{filteredRepos.length}</span>
              </div>
              <div class="pane-actions">
                <input
                  type="text"
                  class="pane-filter"
                  placeholder="Filter repos…"
                  bind:value={repoFilter}
                  aria-label="Filter repositories"
                />
                <button type="button" class="pane-btn" title="Refresh repositories" on:click={refreshRepos}>⟳</button>
              </div>
            </header>
            <div class="pane-content">
              {#if !repoResult.ok}
                <p class="empty">Could not load repos.</p>
              {:else if repoResult.items.length === 0}
                <p class="empty">No repos.</p>
              {:else if filteredRepos.length === 0}
                <p class="empty">No matching repos.</p>
              {:else}
                <ul class="cards">
                  {#each filteredRepos as repo}
                    <li>
                      <strong>{repo.name}</strong>
                      <span class="tag">{repo.visibility}</span>
                      {#if repo.branch}<span class="tag">{repo.branch}</span>{/if}
                      {#if repo.worktree}<span class="tag">{repo.worktree}</span>{/if}
                      {#if repo.commit}<span class="tag">{repo.commit}</span>{/if}
                      {#if repo.html_url}
                        <a class="md-a" href={repo.html_url} rel="noopener noreferrer">{repo.html_url}</a>
                      {/if}
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>

          {:else if activeWin === "users"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["users"]} />
                </svg>
                <h1>Users</h1>
                <span class="pane-badge">{userResult.items.length}</span>
              </div>
              <div class="pane-actions">
                <input
                  type="text"
                  class="pane-filter"
                  placeholder="Filter users…"
                  bind:value={userSearch}
                  aria-label="Filter users"
                />
                <select class="pane-select" bind:value={userRoleFilter} aria-label="User role filter">
                  <option value="all">All roles</option>
                  <option value="sysmin">Sysmin</option>
                  <option value="admin">Admin</option>
                  <option value="member">Member</option>
                  <option value="viewer">Viewer</option>
                </select>
                <select class="pane-select" bind:value={userStatusFilter} aria-label="User status filter">
                  <option value="all">All statuses</option>
                  <option value="active">Active</option>
                  <option value="invited">Invited</option>
                  <option value="suspended">Suspended</option>
                </select>
                <button type="button" class="key action-btn" on:click={() => (showNewUserModal = true)}>+ Add User</button>
                <button type="button" class="pane-btn" title="Refresh users" on:click={refreshUsers}>⟳</button>
              </div>
            </header>
            <div class="pane-content">
              <!-- KPI Stats Bar -->
              <div class="kpi-bar">
                <div class="kpi-item">
                  <span class="kpi-label">TOTAL USERS</span>
                  <span class="kpi-val">{userRoleCounts.total}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">SYSMIN</span>
                  <span class="kpi-val text-sysmin">{userRoleCounts.sysmin}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">ADMIN</span>
                  <span class="kpi-val text-admin">{userRoleCounts.admin}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">MEMBER</span>
                  <span class="kpi-val">{userRoleCounts.member}</span>
                </div>
                <div class="kpi-item">
                  <span class="kpi-label">VIEWER</span>
                  <span class="kpi-val">{userRoleCounts.viewer}</span>
                </div>
              </div>

              {#if userNotice}
                <div class="notice-bar">{userNotice}</div>
              {/if}

              <!-- Modal / Form for New User -->
              {#if showNewUserModal}
                <div class="modal-card">
                  <div class="modal-head">
                    <h3>Invite / Add User</h3>
                    <button type="button" class="pane-btn close" on:click={() => (showNewUserModal = false)}>✕</button>
                  </div>
                  <form class="modal-form" on:submit|preventDefault={handleCreateUser}>
                    <div class="form-row">
                      <label>
                        <span>Username / Login *</span>
                        <input type="text" bind:value={newUserLogin} placeholder="e.g. jdoe" required />
                      </label>
                      <label>
                        <span>Display Name</span>
                        <input type="text" bind:value={newUserName} placeholder="e.g. Jane Doe" />
                      </label>
                    </div>
                    <label>
                      <span>Email</span>
                      <input type="email" bind:value={newUserEmail} placeholder="jane@example.com" />
                    </label>
                    <div class="form-row">
                      <label>
                        <span>Role</span>
                        <select bind:value={newUserRole}>
                          <option value="sysmin">Sysmin (Full Control)</option>
                          <option value="admin">Admin (Project Ops)</option>
                          <option value="member">Member (Write & Execute)</option>
                          <option value="viewer">Viewer (Read-Only)</option>
                        </select>
                      </label>
                      <label>
                        <span>Status</span>
                        <select bind:value={newUserStatus}>
                          <option value="active">Active</option>
                          <option value="invited">Invited</option>
                          <option value="suspended">Suspended</option>
                        </select>
                      </label>
                    </div>
                    <div class="modal-foot">
                      <button type="button" class="key btn-secondary" on:click={() => (showNewUserModal = false)}>Cancel</button>
                      <button type="submit" class="key btn-primary">Create User</button>
                    </div>
                  </form>
                </div>
              {/if}

              {#if !userResult.ok}
                <p class="empty">Could not load users.</p>
              {:else if userResult.items.length === 0}
                <p class="empty">No users.</p>
              {:else if filteredDbUsers.length === 0}
                <p class="empty">No matching users.</p>
              {:else}
                <ul class="cards">
                  {#each filteredDbUsers as user}
                    <li>
                      <div class="user-row-ident">
                        <span class="avatar-circle role-{user.role}">{user.login.slice(0, 2).toUpperCase()}</span>
                        <div class="user-row-text">
                          <strong>{user.display_name || user.login}</strong>
                          <span class="user-subtext">@{user.login}{#if user.email} · {user.email}{/if}</span>
                        </div>
                      </div>
                      <span class="tag">{user.role}</span>
                      <span class="tag">{user.status}</span>
                      <div class="user-row-actions">
                        <select
                          class="pane-select-inline"
                          value={user.role}
                          on:change={(e) => handleUpdateUserRole(user, e.currentTarget.value)}
                          disabled={!isSysminUser}
                          aria-label="Change role"
                        >
                          <option value="sysmin">sysmin</option>
                          <option value="admin">admin</option>
                          <option value="member">member</option>
                          <option value="viewer">viewer</option>
                        </select>
                        <select
                          class="pane-select-inline"
                          value={user.status}
                          on:change={(e) => handleUpdateUserStatus(user, e.currentTarget.value)}
                          disabled={!isSysminUser}
                          aria-label="Change status"
                        >
                          <option value="active">active</option>
                          <option value="invited">invited</option>
                          <option value="suspended">suspended</option>
                        </select>
                        {#if isSysminUser}
                          <button
                            type="button"
                            class="del-user-btn"
                            title="Delete user"
                            on:click={() => handleDeleteUser(user)}
                          >
                            ✕
                          </button>
                        {/if}
                      </div>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>

          {:else if activeWin === "schedule"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["schedule"]} />
                </svg>
                <h1>Scheduled tasks</h1>
                <span class="pane-badge">{schedules.length}</span>
              </div>
            </header>
            <div class="pane-content">
              <form class="sched-form" on:submit|preventDefault={submitSchedule}>
                <input bind:value={schedTitle} placeholder="title" />
                <input bind:value={schedWhen} placeholder="run at" />
                <input bind:value={schedInterval} placeholder="interval" />
                <button type="submit" class="key sched-add">Add</button>
              </form>
              {#if schedNotice}<p class="empty">{schedNotice}</p>{/if}
              {#if schedules.length === 0}
                <p class="empty">No scheduled tasks.</p>
              {:else}
                <ul class="cards">
                  {#each schedules as row}
                    <li>
                      <strong>{row.title}</strong>
                      <span class="tag">{row.runAt || row.interval}</span>
                      <button type="button" class="key" on:click={() => dropSchedule(row.id)}>Cancel</button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>

          {:else if activeWin === "settings"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["settings"]} />
                </svg>
                <h1>Settings</h1>
              </div>
            </header>
            <div class="pane-content">
              <p class="tag">Theme pack</p>
              <ul class="cards">
                {#each PACKS as pack}
                  <li>
                    <button type="button" class:here={packId === pack.id} on:click={() => setPack(pack.id)}>{pack.name}</button>
                  </li>
                {/each}
              </ul>
              {#if isSysminUser}
                <p class="tag">System prompt</p>
                <textarea class="prompt-box" bind:value={systemPrompt} rows="5"></textarea>
              {/if}
            </div>

          {:else if activeWin === "profile"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["profile"]} />
                </svg>
                <h1>Profile</h1>
              </div>
            </header>
            <div class="pane-content">
              <p><strong>{session?.login}</strong></p>
              <p class="tag">{session?.email || "github"}</p>
              <p class="tag">{isSysminUser ? "sysmin" : "member"}</p>
            </div>

          {:else if activeWin === "branch"}
            <header class="pane-head">
              <div class="pane-brand">
                <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                  <path fill="currentColor" d={WIN_ICONS["branch"]} />
                </svg>
                <h1>Branch</h1>
                {#if branchThreadId}<span class="pane-badge">#{shortId(branchThreadId)}</span>{/if}
              </div>
            </header>
            <div class="pane-content">
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
            </div>
          {/if}
        </section>
      {/if}

      <!-- Chat View -->
      {#if activeWin === "chat" || splitOpen}
        <section class="dash-page chat-page" data-view="chat">
          <header class="pane-head chat-head">
            <div class="pane-brand">
              <svg class="pane-icon" viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                <path fill="currentColor" d={WIN_ICONS["chat"]} />
              </svg>
              <h1>Chat</h1>
              {#if currentThread}
                <span class="pane-badge">{currentThread.model}</span>
              {/if}
            </div>
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
            <div class="pane-actions">
              <button type="button" class="pane-btn" title="Toggle thread tree" on:click={() => (showTree = !showTree)}>☷</button>
            </div>
          </header>

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

          <form class="dock glass" data-ctx="composer" on:submit|preventDefault={handleSend}>
            {#if slashHits.length}
              <div class="slash-menu" role="listbox" aria-label="Slash commands">
                {#each slashHits as item, i}
                  <button
                    type="button"
                    role="option"
                    class:here={i === slashIndex}
                    aria-selected={i === slashIndex}
                    on:click={() => pickSlash(item)}
                  >
                    <span>{item.cmd}</span>
                    <span class="tag">{item.hint}</span>
                  </button>
                {/each}
              </div>
            {/if}
            <div class="dock-row">
              <input
                type="text"
                bind:this={composerEl}
                bind:value={composerInput}
                placeholder="message or /fork /task /theme /plan…"
                autocomplete="off"
                on:keydown={onComposerKey}
              />
              <button type="submit" class="send">SEND</button>
            </div>
          </form>
        </section>
      {/if}
    </main>
  </div>

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

  {#if isSysminUser}
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
    flex-direction: row;
    overflow: hidden;
    color: var(--ink);
    background: #06070a;
  }
  .dash-sidebar {
    width: 250px;
    min-width: 250px;
    max-width: 250px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #080a0d;
    border-right: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
    user-select: none;
    z-index: 5;
  }
  .sidebar-top {
    padding: 20px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--ink);
    flex-shrink: 0;
  }
  .brand-hex {
    width: 38px;
    height: 42px;
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    flex-shrink: 0;
    filter: drop-shadow(0 2px 6px rgba(0, 0, 0, 0.6));
  }
  .brand-icon {
    display: block;
    width: 100%;
    height: 100%;
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
  .sidebar-proj {
    position: relative;
    width: 100%;
  }
  .sidebar-proj .proj-key {
    width: 100%;
    max-width: 100%;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.07);
    padding: 8px 12px;
    border-radius: 8px;
  }
  .proj-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dash-nav {
    flex: 1;
    overflow-y: auto;
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .nav-section-title {
    font-size: 10px;
    font-weight: 650;
    letter-spacing: 0.12em;
    color: var(--muted);
    padding: 4px 10px 8px;
    text-transform: uppercase;
  }
  .dash-nav-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    background: transparent;
    border: 1px solid transparent;
    color: #94a3b8;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
    text-align: left;
  }
  .dash-nav-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: #f8fafc;
  }
  .dash-nav-btn.active {
    background: #11151f;
    color: #fff;
    font-weight: 600;
    border-color: rgba(255, 255, 255, 0.14);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06), 0 2px 8px rgba(0, 0, 0, 0.4);
  }
  .nav-icon {
    flex-shrink: 0;
    opacity: 0.7;
    transition: opacity 0.15s;
  }
  .dash-nav-btn.active .nav-icon {
    opacity: 1;
    color: #fff;
  }
  .nav-text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nav-count {
    font-size: 11px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--muted);
    padding: 1px 6px;
    border-radius: 99px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .dash-nav-btn.active .nav-count {
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
  }
  .sidebar-footer {
    padding: 14px 14px 18px;
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: #07090d;
  }
  .sidebar-footer .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0;
  }
  .sidebar-user {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 8px;
  }
  .user-avatar {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: #1b202c;
    color: #f1f5f9;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }
  .user-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.1;
  }
  .user-login {
    font-size: 12px;
    font-weight: 500;
    color: #f1f5f9;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .user-badge {
    font-size: 9px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
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
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    background: #090b0e;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6);
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

  /* Main Dashboard Area & Topbar */
  .dash-main {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    background: #06070a;
  }
  .dash-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
    height: 56px;
    min-height: 56px;
    padding: 0 16px 0 24px;
    gap: 16px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: #080a0d;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    user-select: none;
    z-index: 4;
  }
  .dash-topbar .lead {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0;
    font-size: 13px;
    min-width: 0;
  }
  .view-title {
    font-size: 16px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: #f8fafc;
    margin: 0;
    white-space: nowrap;
  }
  .topbar-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .split-toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  .split-toggle-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
    border-color: rgba(255, 255, 255, 0.16);
  }
  .split-toggle-btn.here {
    background: #181d26;
    color: #fff;
    border-color: rgba(255, 255, 255, 0.2);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
  }
  .split-label {
    white-space: nowrap;
  }

  /* Viewport & Pages */
  .dash-viewport {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow: hidden;
    padding: 14px;
    gap: 14px;
    background: #06070a;
  }
  .dash-viewport.has-split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .dash-page {
    flex: 1;
    height: 100%;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: #080a0e;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    overflow: hidden;
    position: relative;
  }
  .active-page {
    z-index: 1;
  }
  .chat-page {
    z-index: 2;
  }
  .chat-page .body {
    flex: 1;
    min-height: 0;
    padding: 16px;
    display: flex;
    gap: 16px;
    overflow: hidden;
  }
  .chat-page .chat-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(10, 13, 18, 0.85);
    backdrop-filter: blur(12px);
    flex-shrink: 0;
  }

  .manage {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .pane-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 24px 36px;
  }
  .pane-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
    background: rgba(10, 13, 18, 0.85);
    backdrop-filter: blur(12px);
    flex-shrink: 0;
  }
  .pane-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .pane-brand h1 {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
    margin: 0;
    white-space: nowrap;
  }
  .pane-icon {
    flex-shrink: 0;
    color: var(--muted);
  }
  .pane-badge {
    font-size: 11px;
    background: rgba(255, 255, 255, 0.08);
    color: var(--muted);
    padding: 2px 8px;
    border-radius: 99px;
  }
  .pane-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .pane-btn {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
    color: var(--muted);
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .pane-btn:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }
  .pane-btn.close:hover {
    background: var(--hot, #e05252);
    color: #fff;
    border-color: transparent;
  }
  .pane-filter {
    height: 28px;
    border-radius: 6px;
    background: #06070a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--ink);
    padding: 0 10px;
    font-size: 12px;
    outline: none;
    width: 140px;
  }
  .pane-filter:focus {
    border-color: rgba(255, 255, 255, 0.25);
  }
  .prompt-box {
    width: 100%;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: #090a0d;
    color: #f1f5f9;
    padding: 12px 16px;
    font-size: 13px;
    font-family: inherit;
    outline: none;
    resize: vertical;
    box-sizing: border-box;
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
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: #0d1015;
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
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: #090a0d;
    color: #f1f5f9;
    padding: 0 16px;
    font-size: 14px;
  }
  .sched-add { min-width: 88px; height: 48px; }

  /* KPI & Dashboard Metrics Bar */
  .kpi-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 20px;
  }
  .kpi-item {
    background: #080a0d;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 10px 16px;
    display: flex;
    flex-direction: column;
    min-width: 120px;
  }
  .kpi-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .kpi-val {
    font-size: 18px;
    font-weight: 700;
    color: #f1f5f9;
    margin-top: 4px;
  }
  .kpi-val.text-active { color: #38bdf8; }
  .kpi-val.text-sysmin { color: #a855f7; }
  .kpi-val.text-admin { color: #38bdf8; }
  .kpi-val.mono-val {
    font-size: 13px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: #94a3b8;
    line-height: 24px;
  }

  /* Projects Grid */
  .project-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }
  .project-card {
    background: #0a0c10;
    border: 1px solid rgba(255, 255, 255, 0.07);
    border-radius: 12px;
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: border-color 0.2s, background 0.2s;
  }
  .project-card:hover {
    border-color: rgba(255, 255, 255, 0.14);
    background: #0c0e14;
  }
  .project-card.is-active {
    border-color: rgba(56, 189, 248, 0.4);
    background: #0b0f17;
  }
  .project-card-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }
  .project-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .project-name {
    font-size: 15px;
    font-weight: 600;
    color: #f1f5f9;
  }
  .project-key {
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--muted);
  }
  .badge-current {
    font-size: 10px;
    font-weight: 700;
    background: rgba(56, 189, 248, 0.15);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.3);
    padding: 2px 6px;
    border-radius: 4px;
    letter-spacing: 0.04em;
  }
  .project-desc {
    font-size: 13px;
    color: var(--muted);
    margin: 0;
    line-height: 1.4;
  }
  .project-path-row {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
  }
  .project-path {
    background: #06070a;
    border: 1px solid rgba(255, 255, 255, 0.06);
    padding: 3px 8px;
    border-radius: 6px;
    color: #94a3b8;
    font-size: 11px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    text-overflow: ellipsis;
    overflow: hidden;
    white-space: nowrap;
    max-width: 100%;
  }
  .project-metrics {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    margin-top: auto;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }
  .metric-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: #94a3b8;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    padding: 3px 8px;
    border-radius: 6px;
  }
  .mode-pill {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    color: #a855f7;
    background: rgba(168, 85, 247, 0.08);
    border-color: rgba(168, 85, 247, 0.18);
  }
  .project-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
  }

  /* Modals & Forms */
  .modal-card {
    background: #090b0f;
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 12px;
    padding: 20px;
    margin-bottom: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.6);
  }
  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .modal-head h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: #f1f5f9;
  }
  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .form-row {
    display: flex;
    gap: 12px;
  }
  .modal-form label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    font-size: 12px;
    color: var(--muted);
  }
  .modal-form input,
  .modal-form select {
    height: 38px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: #06070a;
    color: #f1f5f9;
    padding: 0 12px;
    font-size: 13px;
    outline: none;
    font-family: inherit;
  }
  .modal-form input:focus,
  .modal-form select:focus {
    border-color: rgba(255, 255, 255, 0.25);
  }
  .modal-foot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  /* Controls & Selects */
  .pane-select {
    height: 28px;
    border-radius: 6px;
    background: #06070a;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--ink);
    padding: 0 8px;
    font-size: 12px;
    outline: none;
    font-family: inherit;
    cursor: pointer;
  }
  .pane-select-inline {
    height: 26px;
    border-radius: 6px;
    background: #07090d;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--ink);
    padding: 0 6px;
    font-size: 12px;
    outline: none;
    font-family: inherit;
    cursor: pointer;
  }
  .action-btn {
    height: 28px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 500;
  }
  .action-active {
    height: 28px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 600;
    opacity: 0.6;
    cursor: default;
    background: rgba(255, 255, 255, 0.04);
  }
  .key-subtle {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: var(--muted);
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .key-subtle:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }
  .key-danger {
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #f87171;
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.15s;
  }
  .key-danger:hover {
    background: rgba(239, 68, 68, 0.16);
  }
  .btn-secondary {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--muted);
    padding: 0 14px;
    height: 34px;
    font-size: 13px;
  }
  .btn-primary {
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #fff;
    padding: 0 16px;
    height: 34px;
    font-size: 13px;
    font-weight: 500;
  }

  /* User Row Elements */
  .user-row-ident {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 180px;
  }
  .user-row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .user-subtext {
    font-size: 12px;
    color: var(--muted);
  }
  .avatar-circle {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #f1f5f9;
    flex-shrink: 0;
  }
  .avatar-circle.role-sysmin {
    background: rgba(168, 85, 247, 0.15);
    border-color: rgba(168, 85, 247, 0.3);
    color: #c084fc;
  }
  .avatar-circle.role-admin {
    background: rgba(56, 189, 248, 0.15);
    border-color: rgba(56, 189, 248, 0.3);
    color: #38bdf8;
  }
  .user-row-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .del-user-btn {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #f87171;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 11px;
    transition: background 0.15s;
  }
  .del-user-btn:hover {
    background: rgba(239, 68, 68, 0.2);
  }
  .notice-bar {
    background: rgba(56, 189, 248, 0.08);
    border: 1px solid rgba(56, 189, 248, 0.25);
    color: #7dd3fc;
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 13px;
    margin-bottom: 16px;
  }
  .tag-sysmin { color: #c084fc; }
  .tag-admin { color: #38bdf8; }
  .tag-member { color: #94a3b8; }
  .tag-viewer { color: #64748b; }
  .tag-active { color: #34d399; }
  .tag-invited { color: #a78bfa; }
  .tag-suspended { color: #f87171; }

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
    border: 1px solid rgba(255, 255, 255, 0.06);
    background: #0a0c10;
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
    background: #07080b;
    border: 1px solid rgba(255, 255, 255, 0.06);
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
    background: #0e1117;
    border: 1px solid rgba(255, 255, 255, 0.05);
    padding: 16px 20px;
    border-radius: 12px;
  }
  .bubble.user .pad {
    white-space: pre-wrap;
    background: #141820;
    border: 1px solid rgba(255, 255, 255, 0.08);
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
    background: #090b0e;
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 10px;
    overflow-x: auto;
    white-space: pre-wrap;
  }
  .md :global(.md-code) {
    font-size: 13px;
    background: #0d1015;
    border: 1px solid rgba(255, 255, 255, 0.06);
    padding: 2px 6px;
    border-radius: 5px;
  }
  .md :global(.md-a) { color: var(--lcd-ink); text-decoration: underline; }
  .md :global(em) { font-style: italic; }
  .md :global(strong) { font-weight: 650; }
  .bubble.system .pad {
    background: #11141c;
    border: 1px solid rgba(255, 255, 255, 0.06);
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
    position: relative;
    flex-shrink: 0;
    z-index: 4;
    padding: 12px 18px calc(14px + env(safe-area-inset-bottom, 0px));
    background: #080a0d !important;
    border-top: 1px solid rgba(255, 255, 255, 0.08);
  }
  .slash-menu {
    position: absolute;
    left: 18px;
    right: 18px;
    bottom: calc(100% - 4px);
    max-height: 260px;
    overflow: auto;
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    background: #090b0e;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6);
  }
  .slash-menu button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    color: var(--ink);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 13px;
  }
  .slash-menu button:hover, .slash-menu button.here {
    background: var(--panel);
  }
  .dock-row {
    width: 100%;
    margin: 0 auto;
    display: flex;
    gap: 10px;
  }
  .dock input {
    flex: 1;
    height: 50px;
    border-radius: 11px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: #090a0d;
    color: var(--ink);
    padding: 0 16px;
    font-size: 15px;
    outline: none;
  }
  .dock input:focus { border-color: var(--slate); }
  .send {
    min-width: 96px;
    height: 50px;
    border: 0;
    border-radius: 11px;
    background: var(--hot);
    color: var(--hot-ink);
    font-size: 13px;
    font-weight: 650;
    letter-spacing: 0.06em;
    cursor: pointer;
  }
  .send:hover { filter: brightness(1.05); }

  .ctx-menu {
    position: fixed;
    z-index: 20;
    min-width: 220px;
    max-width: 300px;
    padding: 8px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    background: #090b0e;
    color: var(--ink);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6);
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
  .gate {
    position: relative;
    z-index: 2;
    min-height: 100dvh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: clamp(24px, 5vw, 64px);
  }
  .gate-card {
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 32px;
    padding: clamp(52px, 8vw, 88px) clamp(40px, 8vw, 96px);
    border-radius: 32px;
    max-width: 540px;
    width: 100%;
    text-align: center;
    background: rgba(10, 12, 16, 0.94) !important;
    border: 1px solid rgba(255, 255, 255, 0.12) !important;
    box-shadow: 0 32px 80px rgba(0, 0, 0, 0.85), inset 0 1px 0 rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(32px);
    -webkit-backdrop-filter: blur(32px);
  }
  .gate-card-bg {
    position: absolute;
    inset: -30px;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    overflow: hidden;
    z-index: 0;
  }
  .gate-card-bg img {
    width: 130%;
    height: 130%;
    object-fit: contain;
    opacity: 0.09;
    filter: blur(1.5px);
    user-select: none;
    pointer-events: none;
  }
  .gate-hex-hero {
    position: relative;
    z-index: 1;
    width: clamp(170px, 25vw, 220px);
    height: clamp(190px, 28vw, 250px);
    clip-path: polygon(50% 0%, 100% 25%, 100% 75%, 50% 100%, 0% 75%, 0% 25%);
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    filter: drop-shadow(0 20px 48px rgba(0, 0, 0, 0.95)) drop-shadow(0 0 1px rgba(255, 255, 255, 0.35));
    transition: transform 300ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  .gate-card:hover .gate-hex-hero {
    transform: scale(1.02);
  }
  .gate-hive-hero {
    width: 100%;
    height: 100%;
    object-fit: cover;
    user-select: none;
    pointer-events: none;
  }
  .gate-login-btn {
    position: relative;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    width: 100%;
    min-height: 76px;
    padding: 0 40px;
    border-radius: 20px;
    background: linear-gradient(180deg, rgba(28, 34, 46, 0.95) 0%, rgba(12, 15, 20, 0.98) 100%);
    border: 1px solid rgba(255, 255, 255, 0.16);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.18), inset 0 -1px 0 rgba(0, 0, 0, 0.6), 0 16px 36px rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    color: #f8fafc;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: 0.08em;
    cursor: pointer;
    transition: transform 120ms cubic-bezier(0.16, 1, 0.3, 1),
                background 160ms cubic-bezier(0.16, 1, 0.3, 1),
                border-color 160ms cubic-bezier(0.16, 1, 0.3, 1),
                box-shadow 160ms cubic-bezier(0.16, 1, 0.3, 1);
    user-select: none;
  }
  .gate-login-btn:hover {
    background: linear-gradient(180deg, rgba(38, 46, 62, 0.96) 0%, rgba(16, 20, 28, 0.98) 100%);
    border-color: rgba(255, 255, 255, 0.32);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.28), inset 0 -1px 0 rgba(0, 0, 0, 0.7), 0 20px 48px rgba(0, 0, 0, 0.85);
  }
  .gate-login-btn:active {
    transform: translateY(2px) scale(0.988);
    background: linear-gradient(180deg, rgba(10, 12, 16, 0.99) 0%, rgba(20, 24, 34, 0.99) 100%);
    box-shadow: inset 0 3px 8px rgba(0, 0, 0, 0.9), 0 4px 12px rgba(0, 0, 0, 0.5);
  }
  .gate-login-btn .gh-icon {
    width: 30px;
    height: 30px;
    flex-shrink: 0;
  }
  .device-code-chip {
    position: relative;
    z-index: 1;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.22em;
    color: #f1f5f9;
    padding: 16px 32px;
    border-radius: 16px;
    background: #080a0e;
    border: 1px solid rgba(255, 255, 255, 0.16);
    box-shadow: inset 0 2px 6px rgba(0, 0, 0, 0.6);
  }
  .gate-err {
    position: relative;
    z-index: 1;
    color: #f87171;
    font-size: 14px;
    margin: 0;
  }
  .gate-pat-fallback {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 48px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.14);
    background: #090a0d;
    color: var(--ink);
    padding: 0 16px;
    font-size: 15px;
    text-align: center;
  }
  .prompt-box {
    width: 100%;
    min-height: 120px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: #090a0d;
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
    background: #090b0e;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .ctx-sep {
    height: 1px;
    margin: 6px 8px;
    background: var(--hair);
  }

  .lcd::-webkit-scrollbar, .tree::-webkit-scrollbar { width: 8px; }
  .lcd::-webkit-scrollbar-thumb { background: color-mix(in srgb, var(--lcd-ink) 22%, transparent); border-radius: 99px; }
  .tree::-webkit-scrollbar-thumb { background: var(--hair); border-radius: 99px; }

  @media (pointer: coarse), (max-width: 1024px) {
    .gate-card {
      max-width: 92vw;
      padding: clamp(48px, 10vw, 96px) clamp(24px, 8vw, 64px);
    }
    .gate-hex-hero {
      width: clamp(180px, 30vw, 240px);
      height: clamp(200px, 34vw, 270px);
    }
    .gate-login-btn {
      min-height: 84px;
      font-size: 24px;
      border-radius: 22px;
    }
    .gate-login-btn .gh-icon {
      width: 36px;
      height: 36px;
    }
    .dash-nav-btn {
      min-height: 48px;
      font-size: 14px;
    }
    .dock input {
      height: 64px;
      font-size: 18px;
    }
    .send {
      min-width: 120px;
      height: 64px;
      font-size: 16px;
    }
    .theme-key, .proj-key {
      min-height: 48px;
      font-size: 13px;
    }
  }
  @media (max-width: 860px) {
    .dash-sidebar {
      width: 68px;
      min-width: 68px;
      max-width: 68px;
    }
    .brand-copy, .nav-text, .nav-count, .nav-section-title, .user-info, .sidebar-proj em, .proj-label {
      display: none;
    }
    .sidebar-top {
      padding: 16px 8px;
      align-items: center;
    }
    .sidebar-proj .proj-key {
      padding: 8px;
      justify-content: center;
    }
    .dash-nav {
      padding: 12px 8px;
      align-items: center;
    }
    .dash-nav-btn {
      justify-content: center;
      padding: 10px;
    }
    .sidebar-footer {
      padding: 10px 8px;
      align-items: center;
    }
    .sidebar-footer .meta {
      flex-direction: column;
      gap: 8px;
    }
    .sidebar-user {
      justify-content: center;
      padding: 6px;
    }
    .dash-viewport.has-split {
      grid-template-columns: 1fr;
    }
    .dash-topbar {
      padding: 0 12px 0 16px;
      gap: 10px;
    }
    .lead {
      gap: 10px;
    }
    .proj-key {
      max-width: 140px;
    }
    .lcd {
      padding: 24px 20px 32px;
      border-radius: 16px;
    }
    .bubble {
      max-width: 88%;
    }
  }
  @media (max-width: 520px) {
    .win button:not(.close) { display: none; }
    .view-title { font-size: 14px; }
  }
</style>
