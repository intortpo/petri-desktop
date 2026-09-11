use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;

pub struct EverosProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
}

impl EverosProcess {
    pub fn spawn(script_path: &PathBuf) -> Result<Self, String> {
        let mut child = Command::new("python3")
            .arg("-u")
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn everos sidecar at {}: {e}", script_path.display()))?;

        let stdin = child.stdin.take().ok_or("Failed to open stdin for everos sidecar")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout for everos sidecar")?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub fn send(&mut self, payload: &Value) -> Result<Value, String> {
        let line = serde_json::to_string(payload).map_err(|e| e.to_string())? + "\n";
        self.stdin.write_all(line.as_bytes()).map_err(|e| format!("write error: {e}"))?;
        self.stdin.flush().map_err(|e| format!("flush error: {e}"))?;

        let mut out = String::new();
        self.stdout.read_line(&mut out).map_err(|e| format!("read error: {e}"))?;
        if out.is_empty() {
            return Err("EverOS sidecar closed output pipe".into());
        }

        let resp: Value = serde_json::from_str(&out).map_err(|e| format!("JSON parse error ({e}): {out}"))?;
        if resp.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            Ok(resp.get("result").cloned().unwrap_or(Value::Null))
        } else {
            let err = resp.get("error").and_then(Value::as_str).unwrap_or("unknown error");
            Err(err.to_string())
        }
    }
}

impl Drop for EverosProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub struct EverosManager {
    script_path: PathBuf,
    proc: Mutex<Option<EverosProcess>>,
}

impl EverosManager {
    pub fn new() -> Self {
        let script = find_sidecar_script();
        Self {
            script_path: script,
            proc: Mutex::new(None),
        }
    }

    fn with_proc<T>(&self, f: impl FnOnce(&mut EverosProcess) -> Result<T, String>) -> Result<T, String> {
        let mut lock = self.proc.lock().map_err(|e| e.to_string())?;
        if lock.is_none() {
            let p = EverosProcess::spawn(&self.script_path)?;
            *lock = Some(p);
        }

        match f(lock.as_mut().unwrap()) {
            Ok(val) => Ok(val),
            Err(e) => {
                // If communication broke, drop child so next attempt respawns cleanly
                *lock = None;
                Err(e)
            }
        }
    }

    pub fn health(&self) -> Result<Value, String> {
        self.with_proc(|p| p.send(&json!({"cmd": "health"})))
    }

    pub fn add_turn(&self, session_id: &str, role: &str, content: &str) -> Result<Value, String> {
        self.with_proc(|p| {
            p.send(&json!({
                "cmd": "add",
                "session_id": session_id,
                "role": role,
                "content": content
            }))
        })
    }

    pub fn remember(&self, lesson: &str, domain: &str) -> Result<Value, String> {
        self.with_proc(|p| {
            p.send(&json!({
                "cmd": "remember",
                "lesson": lesson,
                "domain": domain
            }))
        })
    }

    pub fn flush(&self, session_id: &str) -> Result<Value, String> {
        self.with_proc(|p| {
            p.send(&json!({
                "cmd": "flush",
                "session_id": session_id
            }))
        })
    }

    pub fn search(&self, query: &str, top_k: usize) -> Result<Vec<Value>, String> {
        self.with_proc(|p| {
            let res = p.send(&json!({
                "cmd": "search",
                "query": query,
                "top_k": top_k
            }))?;
            Ok(res.as_array().cloned().unwrap_or_default())
        })
    }

    pub fn get_profile(&self) -> Result<String, String> {
        self.with_proc(|p| {
            let res = p.send(&json!({"cmd": "get_profile"}))?;
            Ok(res.as_str().unwrap_or_default().to_string())
        })
    }
}

fn find_sidecar_script() -> PathBuf {
    // Check candidate locations
    let candidates = [
        PathBuf::from("sidecar/everos_sidecar.py"),
        PathBuf::from("../sidecar/everos_sidecar.py"),
        PathBuf::from("deepagents-app/sidecar/everos_sidecar.py"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    // Fall back to HOME/.mesh/sidecar or default relative path
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".mesh/sidecar/everos_sidecar.py");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("sidecar/everos_sidecar.py")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_health_and_memory_roundtrip() {
        let mgr = EverosManager::new();
        // Skip test if python3 is not accessible
        let health = match mgr.health() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("skipping everos sidecar test: {e}");
                return;
            }
        };
        assert_eq!(health.get("status").and_then(Value::as_str), Some("ok"));

        let rem = mgr.remember("Rust and TypeScript unified memory", "arch").unwrap();
        assert_eq!(rem.get("status").and_then(Value::as_str), Some("ok"));

        let results = mgr.search("unified memory", 3).unwrap();
        assert!(!results.is_empty());
    }
}
