use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use tauri::State;

mod agy_session;
mod context_compiler;
mod learning;
mod paths;
mod security;
mod skills;
mod store;

use paths::{persist_last_thread_at, MeshPaths};
use security::{
    agy_argv, discover_module_manifests, launch_agy_terminal, parse_mcp_servers, prepare_turn_payload,
    write_pat_file, write_resume_cache, AgySpawnOpts, ModuleManifest,
};
use store::Store;

struct AppState {
    store: Mutex<Store>,
    paths: MeshPaths,
}

fn default_project() -> String {
    std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string())
}

fn persist_last(paths: &MeshPaths, thread_id: &str, project_path: &str) {
    let _ = persist_last_thread_at(&paths.last_thread(), thread_id, project_path);
}

fn db_path(state: &AppState) -> PathBuf {
    state.paths.db()
}

#[tauri::command]
fn get_auth_status() -> Result<String, String> {
    let accounts = MeshPaths::google_accounts();
    let adc = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".config/gcloud/application_default_credentials.json");
    if accounts.exists() {
        Ok("Authenticated (Antigravity CLI Auth)".to_string())
    } else if adc.exists() {
        Ok("Authenticated (Google ADC)".to_string())
    } else {
        Ok("Unauthenticated".to_string())
    }
}

#[tauri::command]
fn auth_doctor() -> Result<String, String> {
    let mut lines = Vec::new();
    let agy = Command::new("agy").arg("--version").output();
    match agy {
        Ok(out) if out.status.success() => {
            lines.push(format!(
                "agy: {} ({})",
                which_agy(),
                String::from_utf8_lossy(&out.stdout).trim()
            ));
        }
        Ok(out) => lines.push(format!(
            "agy: present but --version failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(_) => lines.push("agy: MISSING on PATH".to_string()),
    }
    let accounts = MeshPaths::google_accounts();
    lines.push(format!(
        "google_accounts.json: {}",
        if accounts.exists() {
            accounts.display().to_string()
        } else {
            "missing".into()
        }
    ));
    let cache = MeshPaths::default_resume_cache();
    if cache.exists() {
        let n = std::fs::read_to_string(&cache)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&s).ok())
            .map(|m| m.len())
            .unwrap_or(0);
        lines.push(format!("last_conversations: {n} workspace(s)"));
    } else {
        lines.push("last_conversations: missing".into());
    }
    let home = std::env::var("HOME").unwrap_or_default();
    for dir in [
        PathBuf::from(&home).join(".gemini/config/skills"),
        PathBuf::from(&home).join(".mesh/skills"),
    ] {
        let n = std::fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
        lines.push(format!("skills {}: {n}", dir.display()));
    }
    match Command::new("agy").args(["plugin", "list"]).output() {
        Ok(out) => {
            let body = if out.status.success() {
                String::from_utf8_lossy(&out.stdout)
            } else {
                String::from_utf8_lossy(&out.stderr)
            };
            lines.push(format!("plugins:\n{}", body.trim()));
        }
        Err(e) => lines.push(format!("plugins: agy plugin list failed ({e})")),
    }
    Ok(lines.join("\n"))
}

fn which_agy() -> String {
    Command::new("which")
        .arg("agy")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "agy".into())
}

#[tauri::command]
fn get_tailscale_status() -> Result<String, String> {
    match Command::new("tailscale").arg("status").output() {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
        Ok(_) => Err("Tailscale is down or not installed".to_string()),
        Err(e) => Err(format!("Failed to execute tailscale: {e}")),
    }
}

fn modules_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join("modules")
}

#[tauri::command]
fn list_modules(project_path: Option<String>) -> Result<Vec<String>, String> {
    let dir = modules_dir(&project_path.unwrap_or_else(default_project));
    let mut modules = Vec::new();
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
        return Ok(modules);
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    modules.push(name.to_string());
                }
            }
        }
    }
    modules.sort();
    Ok(modules)
}

#[tauri::command]
fn list_module_manifests(project_path: Option<String>) -> Result<Vec<ModuleManifest>, String> {
    let dir = modules_dir(&project_path.unwrap_or_else(default_project));
    Ok(discover_module_manifests(&dir))
}

#[tauri::command]
fn save_github_pat(state: State<'_, AppState>, pat: String) -> Result<(), String> {
    write_pat_file(&state.paths.pat_file(), &pat)
}

#[tauri::command]
fn get_github_pat(state: State<'_, AppState>) -> Result<String, String> {
    std::fs::read_to_string(state.paths.pat_file()).map_err(|e| e.to_string())
}

#[tauri::command]
fn verify_github_pat(pat: String) -> Result<String, String> {
    let output = Command::new("curl")
        .arg("-s")
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", pat))
        .arg("https://api.github.com/user")
        .output();
    match output {
        Ok(out) => {
            let res = String::from_utf8_lossy(&out.stdout).to_string();
            if res.contains("\"login\":") {
                if let Some(login_line) = res.lines().find(|l| l.contains("\"login\":")) {
                    let username = login_line.split('"').nth(3).unwrap_or("User");
                    Ok(format!("Authenticated as {username}"))
                } else {
                    Ok("Valid PAT".to_string())
                }
            } else {
                Err("Invalid PAT or API error".to_string())
            }
        }
        Err(e) => Err(format!("Failed to execute curl: {e}")),
    }
}

fn spawn_turn(
    app: tauri::AppHandle,
    db: PathBuf,
    thread_id: String,
    agy_id: Option<String>,
    project: String,
    mode: String,
    pack: String,
    user: String,
) -> Result<(), String> {
    let payload = prepare_turn_payload(&mode, &pack, &user);
    debug_assert!({
        let argv = agy_argv(AgySpawnOpts {
            conversation_id: agy_id.as_deref(),
            mode: &mode,
            prompt: &payload,
        });
        !crate::security::argv_contains_api_key(&argv)
    });
    agy_session::run_agy_turn(app, thread_id, agy_id, project, mode, payload, db)
}

#[tauri::command]
fn start_turn(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    thread_id: String,
    prompt: String,
    mode: Option<String>,
    persona: Option<String>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mode = store
        .apply_mode(&thread_id, mode.as_deref())
        .map_err(|e| e.to_string())?;
    let thread = store.get_thread(&thread_id).map_err(|e| e.to_string())?;
    store
        .add_message(&thread_id, "user", &prompt, None)
        .map_err(|e| e.to_string())?;
    let project = thread.project_path.clone().unwrap_or_else(default_project);
    persist_last(&state.paths, &thread_id, &project);
    let mut pack = context_compiler::build_context_pack(&project, &mode, &prompt);
    if let Some(sys) = system_prompt.as_deref().filter(|s| !s.is_empty()) {
        pack = format!("[SYSTEM TRAIT]\n{sys}\n\n{pack}");
    }
    if let Some(p) = persona.as_deref().filter(|s| !s.is_empty()) {
        pack = format!("{p}\n\n{pack}");
    }
    let db = db_path(&state);
    spawn_turn(
        app,
        db,
        thread_id,
        thread.agy_conversation_id,
        project,
        mode,
        pack,
        prompt,
    )
}

#[tauri::command]
fn set_thread_mode(
    state: State<'_, AppState>,
    thread_id: String,
    mode: String,
) -> Result<String, String> {
    let store = state.store.lock().unwrap();
    store
        .apply_mode(&thread_id, Some(&mode))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_skills(project_path: String) -> Result<Vec<String>, String> {
    Ok(skills::discover_skills(&project_path))
}

#[tauri::command]
fn install_plugin(url: String) -> Result<String, String> {
    skills::install_plugin(&url)
}

#[tauri::command]
fn learn_from_thread(state: State<'_, AppState>, thread_id: String) -> Result<String, String> {
    let store = state.store.lock().unwrap();
    let thread = store.get_thread(&thread_id).map_err(|e| e.to_string())?;
    let msgs = store.get_messages(&thread_id).map_err(|e| e.to_string())?;
    let mut transcript = String::new();
    for msg in msgs {
        transcript.push_str(&format!("{}: {}\n", msg.role, msg.content));
    }
    let project = thread.project_path.clone().unwrap_or_else(default_project);
    learning::extract_skill_from_thread(&project, &transcript)
}

#[tauri::command]
fn index_workspace(project_path: String) -> Result<String, String> {
    learning::index_workspace(&project_path)
}

#[tauri::command]
fn update_memory_bank(app: tauri::AppHandle, state: State<'_, AppState>, project_path: String) -> Result<(), String> {
    let prompt = "Analyze the recent changes in this project and update memory-bank/activeContext.md, memory-bank/progress.md, and memory-bank/decisionLog.md accordingly. Create the memory-bank/ directory if it does not exist.".to_string();
    let pack = context_compiler::build_context_pack(&project_path, "architect", &prompt);
    spawn_turn(
        app,
        db_path(&state),
        "umb-thread".to_string(),
        None,
        project_path,
        "architect".to_string(),
        pack,
        prompt,
    )
}

#[tauri::command]
fn remember_lesson(
    state: State<'_, AppState>,
    lesson: String,
    _project_path: Option<String>,
) -> Result<(), String> {
    learning::remember_lesson(&state.paths.lessons(), &lesson)
}

#[tauri::command]
fn recall_knowledge(
    state: State<'_, AppState>,
    query: String,
    project_path: Option<String>,
) -> Result<String, String> {
    let project = project_path.unwrap_or_else(default_project);
    learning::recall(&state.paths.lessons(), &project, &query)
}

#[tauri::command]
fn create_thread(
    state: State<'_, AppState>,
    project_path: Option<String>,
    mode: String,
    model: String,
) -> Result<store::Thread, String> {
    let store = state.store.lock().unwrap();
    let thread = store
        .create_thread(project_path.clone(), mode, model, None)
        .map_err(|e| e.to_string())?;
    let project = project_path.unwrap_or_else(default_project);
    persist_last(&state.paths, &thread.id, &project);
    Ok(thread)
}

#[tauri::command]
fn save_agy_id(state: State<'_, AppState>, thread_id: String, agy_id: String) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    store
        .set_agy_conversation_id(&thread_id, &agy_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_message(
    state: State<'_, AppState>,
    thread_id: String,
    role: String,
    content: String,
    raw: Option<String>,
) -> Result<store::Message, String> {
    let store = state.store.lock().unwrap();
    store
        .add_message(&thread_id, &role, &content, raw)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_messages(state: State<'_, AppState>, thread_id: String) -> Result<Vec<store::Message>, String> {
    let store = state.store.lock().unwrap();
    store.get_messages(&thread_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_last_thread(state: State<'_, AppState>) -> Result<Option<store::Thread>, String> {
    let path = state.paths.last_thread();
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let Some(id) = v.get("thread_id").and_then(|x| x.as_str()) else {
        return Ok(None);
    };
    let store = state.store.lock().unwrap();
    match store.get_thread(id) {
        Ok(t) => Ok(Some(t)),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
fn get_thread(state: State<'_, AppState>, thread_id: String) -> Result<store::Thread, String> {
    let store = state.store.lock().unwrap();
    store.get_thread(&thread_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_threads(
    state: State<'_, AppState>,
    project_path: Option<String>,
) -> Result<Vec<store::Thread>, String> {
    let store = state.store.lock().unwrap();
    store
        .list_threads(project_path.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_child_threads(state: State<'_, AppState>, parent_id: String) -> Result<Vec<store::Thread>, String> {
    let store = state.store.lock().unwrap();
    store.list_children(&parent_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn thread_trail(state: State<'_, AppState>, thread_id: String) -> Result<Vec<store::Thread>, String> {
    let store = state.store.lock().unwrap();
    store.thread_ancestors(&thread_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn find_thread(state: State<'_, AppState>, query: String) -> Result<Option<store::Thread>, String> {
    let store = state.store.lock().unwrap();
    store.find_thread(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_projects(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let store = state.store.lock().unwrap();
    store.list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_tasks(state: State<'_, AppState>, thread_id: String) -> Result<Vec<store::Task>, String> {
    let store = state.store.lock().unwrap();
    store.list_tasks(&thread_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_task(
    state: State<'_, AppState>,
    thread_id: String,
    title: String,
    prompt: String,
) -> Result<store::Task, String> {
    let store = state.store.lock().unwrap();
    store
        .create_task(&thread_id, None, &title, &prompt, "pending")
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn fork_thread(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    parent_id: String,
    prompt: String,
    mode: Option<String>,
    start: Option<bool>,
) -> Result<store::ForkRecords, String> {
    let store = state.store.lock().unwrap();
    let forked = store
        .fork_from(&parent_id, &prompt, mode.as_deref())
        .map_err(|e| e.to_string())?;
    let project = forked
        .thread
        .project_path
        .clone()
        .unwrap_or_else(default_project);
    persist_last(&state.paths, &parent_id, &project);
    if start.unwrap_or(true) {
        store
            .add_message(&forked.thread.id, "user", &prompt, None)
            .map_err(|e| e.to_string())?;
        let pack = context_compiler::build_context_pack(&project, &forked.thread.mode, &prompt);
        spawn_turn(
            app,
            db_path(&state),
            forked.thread.id.clone(),
            None,
            project,
            forked.thread.mode.clone(),
            pack,
            prompt,
        )?;
    }
    Ok(forked)
}

#[tauri::command]
fn resume_thread(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    thread_id: String,
    prompt: Option<String>,
    mode: Option<String>,
) -> Result<(), String> {
    let store = state.store.lock().unwrap();
    let mode = store
        .apply_mode(&thread_id, mode.as_deref())
        .map_err(|e| e.to_string())?;
    let thread = store.get_thread(&thread_id).map_err(|e| e.to_string())?;
    let project = thread.project_path.clone().unwrap_or_else(default_project);
    persist_last(&state.paths, &thread_id, &project);
    let text = prompt.unwrap_or_else(|| "Continue the last turn from this conversation.".to_string());
    store
        .add_message(&thread_id, "user", &text, None)
        .map_err(|e| e.to_string())?;
    let pack = context_compiler::build_context_pack(&project, &mode, &text);
    spawn_turn(
        app,
        db_path(&state),
        thread_id,
        thread.agy_conversation_id,
        project,
        mode,
        pack,
        text,
    )
}

#[tauri::command]
fn export_thread(state: State<'_, AppState>, thread_id: String) -> Result<String, String> {
    let store = state.store.lock().unwrap();
    let thread = store.get_thread(&thread_id).map_err(|e| e.to_string())?;
    let Some(agy_id) = thread.agy_conversation_id.clone() else {
        return Err("This thread has no agy conversation id yet. Send a turn first.".to_string());
    };
    let project = thread.project_path.clone().unwrap_or_else(default_project);
    write_resume_cache(&MeshPaths::default_resume_cache(), &project, &agy_id)?;
    persist_last(&state.paths, &thread_id, &project);
    Ok(format!(
        "Wrote agy resume cache for {thread_id}\n  conversation: {agy_id}\n  cwd: {project}"
    ))
}

#[tauri::command]
fn open_agy(state: State<'_, AppState>, thread_id: String) -> Result<String, String> {
    let note = export_thread(state, thread_id.clone())?;
    let inner = Store::new().map_err(|e| e.to_string())?;
    let thread = inner.get_thread(&thread_id).map_err(|e| e.to_string())?;
    let Some(agy_id) = thread.agy_conversation_id.clone() else {
        return Err("No agy conversation id on this thread.".to_string());
    };
    let project = thread.project_path.unwrap_or_else(default_project);
    match launch_agy_terminal(std::path::Path::new(&project), &agy_id) {
        Ok(launched) => Ok(format!("{note}\n{launched}")),
        Err(hint) => Ok(format!("{note}\n{hint}")),
    }
}

#[tauri::command]
fn stop_turn(thread_id: String) -> Result<String, String> {
    agy_session::stop_thread(&thread_id)
}

#[tauri::command]
fn list_mcp(project_path: Option<String>) -> Result<String, String> {
    let mut names = Vec::new();
    let global = MeshPaths::default_mcp_config();
    if let Ok(raw) = std::fs::read_to_string(&global) {
        names.extend(parse_mcp_servers(&raw));
    }
    if let Some(project) = project_path {
        let local = PathBuf::from(project).join(".mcp.json");
        if let Ok(raw) = std::fs::read_to_string(local) {
            names.extend(parse_mcp_servers(&raw));
        }
    }
    names.sort();
    names.dedup();
    if names.is_empty() {
        Ok("No MCP servers in agy mcp_config.json or project .mcp.json".to_string())
    } else {
        Ok(format!("MCP servers:\n- {}", names.join("\n- ")))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct GithubSession {
    login: String,
    email: String,
}

#[derive(serde::Serialize)]
struct DeviceStart {
    user_code: String,
    verification_uri: String,
}

#[derive(serde::Serialize)]
struct DirNode {
    name: String,
    path: String,
    kind: String,
}

#[derive(serde::Serialize)]
struct CoreStatus {
    id: String,
    online: bool,
}

#[derive(serde::Serialize)]
struct AuditRow {
    kind: String,
    detail: String,
}

fn session_path(state: &AppState) -> PathBuf {
    state.paths.root.join("github.session.json")
}

#[tauri::command]
fn github_session(state: State<'_, AppState>) -> Result<Option<GithubSession>, String> {
    let path = session_path(&state);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&raw).ok())
}

#[tauri::command]
fn save_github_session(state: State<'_, AppState>, login: String, email: String) -> Result<(), String> {
    let path = session_path(&state);
    let body = serde_json::to_string(&GithubSession { login, email }).map_err(|e| e.to_string())?;
    std::fs::write(path, body).map_err(|e| e.to_string())
}

#[tauri::command]
fn github_device_start() -> Result<DeviceStart, String> {
    let client_id = std::env::var("PETRI_GITHUB_CLIENT_ID").map_err(|_| {
        "PETRI_GITHUB_CLIENT_ID is not set. Use a GitHub PAT to enter, or set the OAuth client id.".to_string()
    })?;
    let output = Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-H",
            "Accept: application/json",
            "-d",
            &format!("client_id={client_id}&scope=read:user user:email repo"),
            "https://github.com/login/device/code",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    let body = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| body.to_string())?;
    Ok(DeviceStart {
        user_code: v.get("user_code").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        verification_uri: v
            .get("verification_uri")
            .and_then(|x| x.as_str())
            .unwrap_or("https://github.com/login/device")
            .to_string(),
    })
}

#[tauri::command]
fn list_workspace_tree(path: String) -> Result<Vec<DirNode>, String> {
    let skip = ["node_modules", "target", "dist", ".git", ".mesh"];
    let mut out = Vec::new();
    let rd = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || skip.iter().any(|s| *s == name) {
            continue;
        }
        let kind = if ent.path().is_dir() { "dir" } else { "file" };
        out.push(DirNode {
            path: ent.path().display().to_string(),
            name,
            kind: kind.into(),
        });
        if out.len() >= 80 {
            break;
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
    Ok(out)
}

#[tauri::command]
fn list_cores() -> Result<Vec<CoreStatus>, String> {
    let adc = PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".config/gcloud/application_default_credentials.json");
    let gcloud = MeshPaths::google_accounts().exists() || adc.exists();
    Ok(vec![
        CoreStatus { id: "gcloud".into(), online: gcloud },
        CoreStatus { id: "laptop1".into(), online: true },
        CoreStatus { id: "laptop2".into(), online: false },
        CoreStatus { id: "phone1".into(), online: false },
        CoreStatus { id: "phone2".into(), online: false },
        CoreStatus { id: "phone3".into(), online: false },
    ])
}

#[tauri::command]
fn list_audit() -> Result<Vec<AuditRow>, String> {
    Ok(vec![AuditRow {
        kind: "boot".into(),
        detail: "hive shell".into(),
    }])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = MeshPaths::from_env();
    let _ = paths.ensure();
    tauri::Builder::default()
        .manage(AppState {
            store: Mutex::new(Store::open(&paths.db()).expect("Failed to init Store")),
            paths,
        })
        .invoke_handler(tauri::generate_handler![
            get_auth_status,
            auth_doctor,
            get_tailscale_status,
            list_modules,
            list_module_manifests,
            save_github_pat,
            get_github_pat,
            verify_github_pat,
            start_turn,
            set_thread_mode,
            create_thread,
            save_agy_id,
            save_message,
            get_messages,
            update_memory_bank,
            remember_lesson,
            recall_knowledge,
            list_skills,
            install_plugin,
            learn_from_thread,
            index_workspace,
            get_last_thread,
            get_thread,
            list_threads,
            list_child_threads,
            thread_trail,
            find_thread,
            list_projects,
            list_tasks,
            create_task,
            fork_thread,
            resume_thread,
            export_thread,
            open_agy,
            stop_turn,
            list_mcp,
            github_session,
            save_github_session,
            github_device_start,
            list_workspace_tree,
            list_cores,
            list_audit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
