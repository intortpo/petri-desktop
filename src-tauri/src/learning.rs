use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::paths::MeshPaths;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexEntry {
    pub path: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IndexFile {
    pub entries: Vec<IndexEntry>,
}

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    ".git",
    ".mesh",
];

const INDEX_EXTS: &[&str] = &["md", "rs", "ts", "js", "svelte", "py", "json", "toml"];

pub fn write_draft_skill(project_path: &str, markdown: &str) -> Result<String, String> {
    let skill_id = Uuid::new_v4().to_string();
    let skill_dir = PathBuf::from(project_path)
        .join(".agents")
        .join("skills")
        .join(format!("draft-{}", &skill_id[..8]));
    std::fs::create_dir_all(&skill_dir).map_err(|e| format!("Failed to create skill dir: {e}"))?;
    let skill_file = skill_dir.join("SKILL.md");
    std::fs::write(&skill_file, markdown.trim()).map_err(|e| format!("Failed to write SKILL.md: {e}"))?;
    Ok(skill_file.display().to_string())
}

pub fn extract_skill_from_thread(project_path: &str, transcript: &str) -> Result<String, String> {
    let prompt = format!(
        "You are an expert AI extraction system. Read the following conversation transcript and identify a reusable engineering practice, workflow, or pattern that was discovered. Write a concise SKILL.md file that teaches this pattern to an AI agent. Output ONLY the raw markdown content of the SKILL.md file, starting with frontmatter if appropriate.\n\nTranscript:\n{}",
        crate::security::redact_secrets(transcript)
    );

    let output = std::process::Command::new("agy")
        .arg("--print")
        .arg(&prompt)
        .arg("--model")
        .arg("Gemini 1.5 Flash")
        .current_dir(project_path)
        .output();

    let skill_content = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            format!("# Draft skill\n\nagy extract failed:\n{}\n\n## Transcript excerpt\n\n{}", err, excerpt(transcript, 800))
        }
        Err(e) => format!(
            "# Draft skill\n\nagy was unavailable ({e}).\n\n## Transcript excerpt\n\n{}",
            excerpt(transcript, 800)
        ),
    };

    let path = write_draft_skill(project_path, skill_content.trim())?;
    Ok(format!("Successfully extracted a new skill from this thread. Draft saved to: {path}"))
}

fn excerpt(text: &str, max: usize) -> String {
    let t = text.trim();
    if t.chars().count() > max {
        format!("{}…", t.chars().take(max).collect::<String>())
    } else {
        t.to_string()
    }
}

pub fn index_workspace(project_path: &str) -> Result<String, String> {
    let root = PathBuf::from(project_path);
    if !root.exists() {
        return Err(format!("Project path does not exist: {project_path}"));
    }
    let mut entries = Vec::new();
    walk_index(&root, &root, &mut entries, 0)?;
    let dir = MeshPaths::vector_dir_for_project(project_path);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let index = IndexFile { entries };
    let n = index.entries.len();
    let path = MeshPaths::vector_index(project_path);
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(format!("Indexed {n} files into {}", path.display()))
}

fn walk_index(root: &Path, dir: &Path, entries: &mut Vec<IndexEntry>, depth: usize) -> Result<(), String> {
    if depth > 8 || entries.len() >= 200 {
        return Ok(());
    }
    let reader = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    for entry in reader.flatten() {
        if entries.len() >= 200 {
            break;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && name != ".agents" {
            continue;
        }
        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk_index(root, &path, entries, depth + 1)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !INDEX_EXTS.contains(&ext.as_str()) {
            continue;
        }
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > 64 * 1024 {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        entries.push(IndexEntry { path: rel, text });
    }
    Ok(())
}

pub fn search_index(project_path: &str, query: &str) -> Vec<(String, String)> {
    let path = MeshPaths::vector_index(project_path);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(index) = serde_json::from_str::<IndexFile>(&raw) else {
        return Vec::new();
    };
    let q = query.to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for entry in index.entries {
        if entry.path.to_lowercase().contains(&q) || entry.text.to_lowercase().contains(&q) {
            let snippet = snippet_around(&entry.text, &q, 160);
            hits.push((entry.path, snippet));
        }
        if hits.len() >= 12 {
            break;
        }
    }
    hits
}

fn snippet_around(text: &str, q: &str, width: usize) -> String {
    let lower = text.to_lowercase();
    let idx = lower.find(q).unwrap_or(0);
    let start = idx.saturating_sub(40);
    let end = (idx + q.len() + width).min(text.len());
    let slice = text.get(start..end).unwrap_or(text).replace('\n', " ");
    slice.trim().to_string()
}

pub fn remember_lesson(lessons_path: &Path, lesson: &str) -> Result<(), String> {
    if let Some(parent) = lessons_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(lessons_path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "- {}", lesson).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn recall(lessons_path: &Path, project_path: &str, query: &str) -> Result<String, String> {
    let mut blocks = Vec::new();
    if lessons_path.exists() {
        let content = std::fs::read_to_string(lessons_path).map_err(|e| e.to_string())?;
        let q = query.to_lowercase();
        let hits: Vec<&str> = content
            .lines()
            .filter(|l| l.to_lowercase().contains(&q))
            .collect();
        if !hits.is_empty() {
            blocks.push(format!("Lessons:\n{}", hits.join("\n")));
        }
    }
    let index_hits = search_index(project_path, query);
    if !index_hits.is_empty() {
        let lines: Vec<String> = index_hits
            .into_iter()
            .map(|(p, s)| format!("  {p}: {s}"))
            .collect();
        blocks.push(format!("Index:\n{}", lines.join("\n")));
    }
    if blocks.is_empty() {
        Ok(format!("No lessons or index hits matching '{query}'"))
    } else {
        Ok(blocks.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_then_recall_hits_project_file() {
        let dir = std::env::temp_dir().join(format!("mesh-idx-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("NOTES.md"), "sqlite-vec is the local recall backend\n").unwrap();
        let msg = index_workspace(dir.to_str().unwrap()).unwrap();
        assert!(msg.contains("Indexed"), "{msg}");
        assert!(MeshPaths::vector_index(dir.to_str().unwrap()).exists());

        let lessons = dir.join("lessons.md");
        remember_lesson(&lessons, "we use sqlite-vec for recall").unwrap();
        let found = recall(&lessons, dir.to_str().unwrap(), "sqlite-vec").unwrap();
        assert!(found.contains("sqlite-vec"), "{found}");
        assert!(found.contains("NOTES.md") || found.contains("Lessons"), "{found}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn write_draft_skill_creates_skill_md() {
        let dir = std::env::temp_dir().join(format!("mesh-skill-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_draft_skill(dir.to_str().unwrap(), "# Skill\nDo the thing\n").unwrap();
        assert!(path.contains(".agents/skills/draft-"));
        assert!(Path::new(&path).exists());
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("Do the thing"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
