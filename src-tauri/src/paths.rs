use std::path::{Path, PathBuf};

/// Control-plane directories. Production uses `$HOME/.mesh`; tests inject a temp root.
#[derive(Clone, Debug)]
pub struct MeshPaths {
    pub root: PathBuf,
}

impl MeshPaths {
    pub fn from_env() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            root: PathBuf::from(home).join(".mesh"),
        }
    }

    #[allow(dead_code)]
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(self.memory_dir())?;
        Ok(())
    }

    pub fn db(&self) -> PathBuf {
        self.root.join("mesh.db")
    }

    pub fn last_thread(&self) -> PathBuf {
        self.root.join("last_thread.json")
    }

    pub fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }

    pub fn lessons(&self) -> PathBuf {
        self.memory_dir().join("lessons.md")
    }

    pub fn pat_file(&self) -> PathBuf {
        self.root.join("github.pat")
    }

    #[allow(dead_code)]
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn vector_dir_for_project(project_path: &str) -> PathBuf {
        PathBuf::from(project_path).join(".mesh").join("vector")
    }

    pub fn vector_index(project_path: &str) -> PathBuf {
        Self::vector_dir_for_project(project_path).join("index.json")
    }

    pub fn gemini_home() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".gemini")
    }

    pub fn default_resume_cache() -> PathBuf {
        Self::gemini_home()
            .join("antigravity-cli")
            .join("cache")
            .join("last_conversations.json")
    }

    pub fn default_mcp_config() -> PathBuf {
        Self::gemini_home()
            .join("antigravity-cli")
            .join("mcp_config.json")
    }

    pub fn google_accounts() -> PathBuf {
        Self::gemini_home().join("google_accounts.json")
    }
}

pub fn persist_last_thread_at(
    path: &Path,
    thread_id: &str,
    project_path: &str,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::json!({
        "thread_id": thread_id,
        "project_path": project_path,
    });
    std::fs::write(path, body.to_string())
}
