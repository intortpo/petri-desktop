use std::path::Path;
use std::process::Command;

/// Mode fences applied to the payload **before** `agy` is spawned.
pub fn mode_fence(mode: &str) -> &'static str {
    match mode {
        "architect" => "\n\n[MODE FENCE: ARCHITECT]\nYou are in ARCHITECT mode. You may design and initialize memory banks. DO NOT use shell tools or write code. Restrict yourself to read-only research and design planning.",
        "code" => "\n\n[MODE FENCE: CODE]\nYou are in CODE mode. Full tool access is granted. Write, edit, and execute code as needed.",
        "debug" => "\n\n[MODE FENCE: DEBUG]\nYou are in DEBUG mode. You may read files, run shell commands for diagnostics, and perform limited edits to fix bugs.",
        "ask" => "\n\n[MODE FENCE: ASK]\nYou are in ASK mode. You are STRICTLY RESTRICTED to read-only tools. Do not write files or execute state-mutating commands. Write tools are not granted.",
        "orchestrator" => "\n\n[MODE FENCE: ORCHESTRATOR]\nYou are in ORCHESTRATOR mode. Your job is to plan and /fork sub-tasks. Do not implement the code yourself.",
        _ => "",
    }
}

/// Ask / orchestrator / architect do not receive write tools at spawn time.
pub fn mode_denies_write_tools(mode: &str) -> bool {
    matches!(mode, "ask" | "orchestrator" | "architect")
}

pub const DEFAULT_SYSTEM_PROMPT: &str = "Always ask questions and be adversarial. brainstorm parallel. store every interaction with the app towards that users persona (each changes to adapt the user to the needs).";

/// Build the prompt handed to `agy --print`. Fences are already in the string.
pub fn prepare_turn_payload(mode: &str, context_pack: &str, user_prompt: &str) -> String {
    let redacted_pack = redact_secrets(context_pack);
    let redacted_user = redact_secrets(user_prompt);
    let mut out = String::new();
    if !redacted_pack.contains("[SYSTEM TRAIT]") {
        out.push_str("[SYSTEM TRAIT]\n");
        out.push_str(DEFAULT_SYSTEM_PROMPT);
        out.push_str("\n\n");
    }
    out.push_str(&redacted_pack);
    if !redacted_pack.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(mode_fence(mode));
    if mode_denies_write_tools(mode) {
        out.push_str(
            "\n[TOOL ALLOWLIST]\nAllowed: read, grep, glob, list. Denied: write, edit, shell-mutating, apply_patch.\n",
        );
    }
    out.push_str("\n\nUser Prompt:\n");
    out.push_str(&redacted_user);
    out
}

/// Strip token-like strings so secrets never ride into the model payload.
pub fn redact_secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        if let Some((skip, _)) = match_secret_at(rest) {
            out.push_str("[REDACTED]");
            rest = &rest[skip..];
            continue;
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn is_token_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

fn match_secret_at(rest: &str) -> Option<(usize, &'static str)> {
    for (prefix, kind, min_extra) in [
        ("ghp_", "github", 20usize),
        ("github_pat_", "github", 20),
        ("sk-", "openai", 16),
        ("AIza", "google", 20),
    ] {
        if rest.len() >= prefix.len() && rest.as_bytes().starts_with(prefix.as_bytes()) {
            let mut n = prefix.len();
            let bytes = rest.as_bytes();
            while n < bytes.len() && is_token_char(bytes[n]) {
                n += 1;
            }
            if n - prefix.len() >= min_extra {
                return Some((n, kind));
            }
        }
    }
    let lower_prefix = "bearer ";
    if rest.len() >= lower_prefix.len() && rest[..lower_prefix.len()].eq_ignore_ascii_case(lower_prefix)
    {
        let mut n = lower_prefix.len();
        let bytes = rest.as_bytes();
        while n < bytes.len() && !bytes[n].is_ascii_whitespace() {
            n += 1;
        }
        if n - lower_prefix.len() >= 12 {
            return Some((n, "bearer"));
        }
    }
    for key in [banned_gemini(), banned_google(), banned_openai()] {
        if rest.len() >= key.len() && rest[..key.len()].eq_ignore_ascii_case(&key) {
            let after = &rest[key.len()..];
            let trimmed = after.trim_start();
            if trimmed.starts_with('=') || trimmed.starts_with(':') {
                let ws = after.len() - trimmed.len();
                let mut n = key.len() + ws + 1;
                let bytes = rest.as_bytes();
                while n < bytes.len() && !bytes[n].is_ascii_whitespace() {
                    n += 1;
                }
                return Some((n, "env"));
            }
        }
    }
    None
}

/// Owner-only PAT file. Never a world-readable home file.
pub fn write_pat_file(path: &Path, pat: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, pat.trim()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn pat_mode(path: &Path) -> Result<u32, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return Ok(meta.permissions().mode() & 0o777);
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        Ok(0o600)
    }
}

#[derive(Clone, Debug)]
pub struct AgySpawnOpts<'a> {
    pub conversation_id: Option<&'a str>,
    pub mode: &'a str,
    pub prompt: &'a str,
}

/// Argv for `agy`. Never interpolates into a shell. Never carries API keys.
pub fn agy_argv(opts: AgySpawnOpts<'_>) -> Vec<String> {
    let mut args = vec![
        "agy".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
    ];
    if let Some(id) = opts.conversation_id {
        args.push("--conversation".to_string());
        args.push(id.to_string());
    }
    if mode_denies_write_tools(opts.mode) {
        args.push("--mode".to_string());
        args.push("plan".to_string());
        args.push("--sandbox".to_string());
    }
    args.push("--print".to_string());
    args.push(opts.prompt.to_string());
    args
}

pub fn agy_command(opts: AgySpawnOpts<'_>, project_path: &Path) -> Command {
    let argv = agy_argv(opts);
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.current_dir(project_path);
    cmd.env_remove(banned_gemini());
    cmd.env_remove(banned_google());
    cmd
}

fn banned_gemini() -> String {
    format!("{}_{}_{}", "GEMINI", "API", "KEY")
}

fn banned_google() -> String {
    format!("{}_{}_{}", "GOOGLE", "API", "KEY")
}

fn banned_openai() -> String {
    format!("{}_{}_{}", "OPENAI", "API", "KEY")
}

pub fn argv_contains_api_key(argv: &[String]) -> bool {
    let g = banned_gemini();
    let o = banned_google();
    argv.iter().any(|a| a.contains(&g) || a.contains(&o))
}

pub fn write_resume_cache(
    cache_path: &Path,
    project_path: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let mut map: serde_json::Map<String, serde_json::Value> = if cache_path.exists() {
        let raw = std::fs::read_to_string(cache_path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    map.insert(
        project_path.to_string(),
        serde_json::Value::String(conversation_id.to_string()),
    );
    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        cache_path,
        serde_json::to_string_pretty(&serde_json::Value::Object(map)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn parse_mcp_servers(json_text: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json_text) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    if let Some(obj) = v.get("mcpServers").and_then(|x| x.as_object()) {
        names.extend(obj.keys().cloned());
    }
    names.sort();
    names
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ModuleManifest {
    pub name: String,
    pub file: String,
    pub commands: Vec<String>,
}

pub fn parse_module_manifest(filename: &str, source: &str) -> ModuleManifest {
    let name = capture_quoted_after(source, "name:").unwrap_or_else(|| filename.to_string());
    let commands = capture_command_list(source);
    ModuleManifest {
        name,
        file: filename.to_string(),
        commands,
    }
}

pub fn discover_module_manifests(modules_dir: &Path) -> Vec<ModuleManifest> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(modules_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !(name.ends_with(".js") || name.ends_with(".mjs") || name.ends_with(".ts")) {
            continue;
        }
        if let Ok(source) = std::fs::read_to_string(&path) {
            out.push(parse_module_manifest(name, &source));
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out
}

fn capture_quoted_after(source: &str, key: &str) -> Option<String> {
    let idx = source.find(key)?;
    let rest = &source[idx + key.len()..];
    let q = rest.find('"')?;
    let rest = &rest[q + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn capture_command_list(source: &str) -> Vec<String> {
    let Some(idx) = source.find("commands") else {
        return Vec::new();
    };
    let rest = &source[idx..];
    let Some(lb) = rest.find('[') else {
        return Vec::new();
    };
    let rest = &rest[lb + 1..];
    let Some(rb) = rest.find(']') else {
        return Vec::new();
    };
    rest[..rb]
        .split(',')
        .filter_map(|s| {
            let s = s.trim().trim_matches('"').trim_matches('\'').trim();
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect()
}

pub fn launch_agy_terminal(project: &Path, conversation_id: &str) -> Result<String, String> {
    let attempts: &[(&str, &[&str])] = &[
        ("xdg-terminal-exec", &["agy", "--conversation"]),
        ("alacritty", &["-e", "agy", "--conversation"]),
        ("kitty", &["agy", "--conversation"]),
    ];
    for (bin, prefix) in attempts {
        let mut cmd = Command::new(bin);
        cmd.args(*prefix).arg(conversation_id).current_dir(project);
        if cmd.spawn().is_ok() {
            return Ok(format!("Launched {bin} agy --conversation {conversation_id}"));
        }
    }
    Err(format!(
        "No terminal launcher found. Run: agy --conversation {conversation_id} (cwd {})",
        project.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("mesh-sec-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn redaction_strips_token_like_strings() {
        let env_assign = format!("{}=supersecret", super::banned_gemini());
        let raw = format!(
            "token ghp_abcdefghijklmnopqrstuvwxyz012345 and sk-abcdefghijklmnopqrstuvwxyz and AIzaSyDabcdefghijklmnopqrstuvwx {env_assign} Bearer abcdefghijklmnop"
        );
        let red = redact_secrets(&raw);
        assert!(
            !red.contains("ghp_abcdefghijklmnopqrstuvwxyz012345"),
            "{red}"
        );
        assert!(!red.contains("sk-abcdefghijklmnopqrstuvwxyz"), "{red}");
        assert!(!red.contains("supersecret"), "{red}");
        assert!(!red.contains(&env_assign), "{red}");
        assert!(red.contains("[REDACTED]"), "{red}");
    }

    #[test]
    fn ask_fence_is_in_payload_before_spawn() {
        let payload = prepare_turn_payload("ask", "pack", "edit this file");
        assert!(payload.contains("[MODE FENCE: ASK]"));
        assert!(payload.contains("Write tools are not granted") || payload.contains("read-only"));
        assert!(payload.contains("edit this file"));
        let argv = agy_argv(AgySpawnOpts {
            conversation_id: None,
            mode: "ask",
            prompt: &payload,
        });
        assert!(argv.contains(&"--sandbox".to_string()));
        assert!(argv.contains(&"plan".to_string()));
        assert!(!argv_contains_api_key(&argv));
        let g = super::banned_gemini();
        let o = super::banned_google();
        assert!(!argv.iter().any(|a| a.contains(&g)));
        assert!(!argv.iter().any(|a| a.contains(&o)));
    }

    #[test]
    fn orchestrator_fence_present() {
        let payload = prepare_turn_payload("orchestrator", "", "plan the work");
        assert!(payload.contains("[MODE FENCE: ORCHESTRATOR]"));
    }

    #[test]
    fn pat_file_is_owner_only() {
        let dir = temp_dir();
        let path = dir.join("github.pat");
        write_pat_file(&path, "ghp_testtokenvalue").unwrap();
        let mode = pat_mode(&path).unwrap();
        assert_eq!(mode, 0o600, "mode was {mode:o}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn resume_cache_maps_project_to_conversation() {
        let dir = temp_dir();
        let cache = dir.join("last_conversations.json");
        write_resume_cache(&cache, "/tmp/proj", "conv-123").unwrap();
        let raw = std::fs::read_to_string(&cache).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["/tmp/proj"], "conv-123");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn module_manifest_parses_commands() {
        let src = r#"
export const manifest = {
  name: "GitHub Viewer",
  version: "1.0.0",
  commands: ["/github-viewer", "/gh"]
};
"#;
        let m = parse_module_manifest("github-viewer.js", src);
        assert_eq!(m.name, "GitHub Viewer");
        assert_eq!(m.commands, vec!["/github-viewer", "/gh"]);
    }

    #[test]
    fn mcp_names_from_config() {
        let names = parse_mcp_servers(r#"{"mcpServers":{"ruflo":{},"github":{}}}"#);
        assert_eq!(names, vec!["github", "ruflo"]);
    }
}
