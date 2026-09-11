use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize, Clone, Debug)]
pub struct RlmHandle {
    pub id: String,
    pub status: String,
    pub task: String,
}

#[derive(Default)]
pub struct RlmRegistry {
    jobs: HashMap<String, RlmHandle>,
}

impl RlmRegistry {
    pub fn spawn_handle(&mut self, task: String) -> RlmHandle {
        let id = Uuid::new_v4().to_string();
        let handle = RlmHandle {
            id: id.clone(),
            status: "running".into(),
            task,
        };
        self.jobs.insert(id, handle.clone());
        handle
    }

    pub fn get(&self, id: &str) -> Option<RlmHandle> {
        self.jobs.get(id).cloned()
    }

    pub fn finish(&mut self, id: &str, ok: bool) {
        if let Some(h) = self.jobs.get_mut(id) {
            h.status = if ok { "done".into() } else { "failed".into() };
        }
    }
}

/// Spawn returns immediately with status=running. Child flips to done later.
pub fn spawn_rlm(reg: Arc<Mutex<RlmRegistry>>, task: String) -> RlmHandle {
    let handle = {
        let mut g = reg.lock().expect("rlm lock");
        g.spawn_handle(task)
    };
    let id = handle.id.clone();
    let reg2 = Arc::clone(&reg);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        if let Ok(mut g) = reg2.lock() {
            g.finish(&id, true);
        }
    });
    handle
}

pub const BASE_SYSTEM_TRAIT: &str = "Always ask questions and be adversarial. brainstorm parallel. store every interaction with the app towards that users persona (each changes to adapt the user to the needs).";

/// Write a harness CRUD edit. Never overwrites the immutable base system trait file.
pub fn apply_refine(
    harness_dir: &Path,
    kind: &str,
    evidence: &str,
    body: &str,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(harness_dir).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let path = harness_dir.join(format!("refine-{id}.md"));
    let kind = kind.trim();
    if kind == "system-trait" || kind == "system_prompt" {
        return Err("base system trait is immutable".into());
    }
    let text = format!(
        "---\nkind: {kind}\nevidence: {}\n---\n\n{body}\n",
        evidence.replace('\n', " ")
    );
    if text.contains("IMMUTABLE_BASE_OVERWRITE") {
        return Err("refusing to clobber base trait".into());
    }
    std::fs::write(&path, &text).map_err(|e| e.to_string())?;
    let written = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if written.trim() == BASE_SYSTEM_TRAIT {
        let _ = std::fs::remove_file(&path);
        return Err("refine must not overwrite the base system-trait string".into());
    }
    if !written.contains(evidence.trim()) {
        return Err("evidence snippet missing from harness file".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn spawn_returns_running_handle_before_child_finishes() {
        let reg = Arc::new(Mutex::new(RlmRegistry::default()));
        let h = spawn_rlm(Arc::clone(&reg), "scan repo".into());
        assert_eq!(h.status, "running");
        assert!(!h.id.is_empty());
        let still = reg.lock().unwrap().get(&h.id).unwrap();
        assert_eq!(still.status, "running");
    }

    #[test]
    fn refine_writes_evidence_and_does_not_replace_base_trait() {
        let dir = std::env::temp_dir().join(format!("refine-{}", Uuid::new_v4()));
        let path = apply_refine(&dir, "skill", "tests passed on login.rs", "prefer table-driven PAT checks").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("tests passed on login.rs"));
        assert_ne!(body.trim(), BASE_SYSTEM_TRAIT);
        assert!(!body.trim().eq(BASE_SYSTEM_TRAIT));
        let err = apply_refine(&dir, "system-trait", "x", BASE_SYSTEM_TRAIT).unwrap_err();
        assert!(err.contains("immutable"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
