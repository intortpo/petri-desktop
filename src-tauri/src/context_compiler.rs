use std::path::PathBuf;
use std::process::Command;

pub fn build_context_pack(project_path: &str, mode: &str, prompt: &str) -> String {
    let mut pack = String::new();
    let root = PathBuf::from(project_path);

    pack.push_str(&format!("# Petri Context Pack (Mode: {})\n\n", mode));

    // 1. Core Constitutions
    let agents_md = root.join("AGENTS.md");
    if agents_md.exists() {
        if let Ok(content) = std::fs::read_to_string(&agents_md) {
            pack.push_str("## AGENTS.md\n");
            pack.push_str(&content);
            pack.push_str("\n\n");
        }
    }

    let gemini_md = root.join("GEMINI.md");
    if gemini_md.exists() {
        if let Ok(content) = std::fs::read_to_string(&gemini_md) {
            pack.push_str("## GEMINI.md\n");
            pack.push_str(&content);
            pack.push_str("\n\n");
        }
    }

    // 2. Memory Bank
    let memory_bank = root.join("memory-bank");
    if memory_bank.exists() {
        let active_context = memory_bank.join("activeContext.md");
        if let Ok(content) = std::fs::read_to_string(&active_context) {
            pack.push_str("## Active Context\n");
            pack.push_str(&content);
            pack.push_str("\n\n");
        }
        
        let decision_log = memory_bank.join("decisionLog.md");
        if let Ok(content) = std::fs::read_to_string(&decision_log) {
            // Ideally we slice this, but for now take the last 2000 chars
            pack.push_str("## Recent Decisions\n");
            let sliced = if content.len() > 2000 {
                &content[content.len()-2000..]
            } else {
                &content
            };
            pack.push_str(sliced);
            pack.push_str("\n\n");
        }
    }

    // 3. Working Set (git status)
    let git_status = Command::new("git")
        .current_dir(project_path)
        .arg("status")
        .arg("--short")
        .output();
        
    if let Ok(output) = git_status {
        let status_str = String::from_utf8_lossy(&output.stdout);
        if !status_str.trim().is_empty() {
            pack.push_str("## Working Set (Git Status)\n```\n");
            pack.push_str(&status_str);
            pack.push_str("```\n\n");
        }
    }

    // Write it to disk for transparency
    let mesh_dir = root.join(".mesh");
    if !mesh_dir.exists() {
        let _ = std::fs::create_dir_all(&mesh_dir);
    }
    let pack_file = mesh_dir.join("context-pack.md");
    let _ = std::fs::write(pack_file, &pack);
    let _ = prompt;

    pack
}
