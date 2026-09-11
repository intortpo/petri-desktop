use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::Emitter;

use crate::security::{agy_command, AgySpawnOpts};
use crate::store::Store;

static LIVE_PIDS: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);

fn live() -> std::sync::MutexGuard<'static, Option<HashMap<String, u32>>> {
    LIVE_PIDS.lock().unwrap()
}

pub fn register_pid(thread_id: &str, pid: u32) {
    let mut g = live();
    g.get_or_insert_with(HashMap::new).insert(thread_id.to_string(), pid);
}

pub fn unregister_pid(thread_id: &str) {
    if let Some(map) = live().as_mut() {
        map.remove(thread_id);
    }
}

/// Kill the agy child for this thread. Argv only — no shell interpolation.
pub fn stop_thread(thread_id: &str) -> Result<String, String> {
    let pid = {
        let mut g = live();
        g.get_or_insert_with(HashMap::new).remove(thread_id)
    };
    let Some(pid) = pid else {
        return Ok(format!("No running agy process for thread {thread_id}"));
    };
    kill_pid(pid)?;
    Ok(format!("Stopped agy pid {pid} for thread {thread_id}"))
}

pub fn kill_pid(pid: u32) -> Result<(), String> {
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .map_err(|e| format!("kill failed: {e}"))?;
    if !status.success() {
        let _ = Command::new("kill").arg("-KILL").arg(pid.to_string()).status();
    }
    Ok(())
}

pub fn run_agy_turn(
    app: tauri::AppHandle,
    thread_id: String,
    agy_id: Option<String>,
    project_path: String,
    mode: String,
    prompt: String,
    db_path: PathBuf,
) -> Result<(), String> {
    let mut cmd = agy_command(
        AgySpawnOpts {
            conversation_id: agy_id.as_deref(),
            mode: &mode,
            prompt: &prompt,
        },
        Path::new(&project_path),
    );
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn agy: {e}"))?;
    register_pid(&thread_id, child.id());
    let stdout = child.stdout.take().expect("Failed to grab stdout");

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut assistant = String::new();
        let mut saved_assistant = false;
        for line in reader.lines() {
            if let Ok(l) = line {
                if let Ok(mut parsed) = serde_json::from_str::<Value>(&l) {
                    if let Some(delta) = parsed
                        .pointer("/step_update/text_delta")
                        .and_then(|v| v.as_str())
                    {
                        assistant.push_str(delta);
                    }
                    if parsed.get("event").and_then(|v| v.as_str()) == Some("result")
                        && !assistant.is_empty()
                        && !saved_assistant
                    {
                        if let Ok(store) = Store::open(&db_path) {
                            let _ = store.add_message(
                                &thread_id,
                                "assistant",
                                &assistant,
                                Some(parsed.to_string()),
                            );
                            saved_assistant = true;
                        }
                    }
                    if let Some(obj) = parsed.as_object_mut() {
                        obj.insert("thread_id".to_string(), json!(thread_id.clone()));
                    }
                    let _ = app.emit("agy-event", parsed);
                } else {
                    let _ = app.emit("agy-log", json!({ "thread_id": thread_id, "line": l }));
                }
            }
        }
        let status = child.wait();
        unregister_pid(&thread_id);
        let wait_failed = status.map(|s| !s.success()).unwrap_or(true);
        if let Ok(store) = Store::open(&db_path) {
            if !assistant.is_empty() && !saved_assistant {
                let _ = store.add_message(&thread_id, "assistant", &assistant, None);
            }
            let had_text = !assistant.is_empty()
                || store
                    .last_assistant_excerpt(&thread_id, 8)
                    .ok()
                    .flatten()
                    .is_some();
            let failed = wait_failed && !had_text;
            let _ = store.complete_child_turn(&thread_id, failed);
            let _ = app.emit(
                "agy-done",
                json!({
                    "thread_id": thread_id,
                    "failed": failed,
                }),
            );
        } else {
            let _ = app.emit(
                "agy-done",
                json!({
                    "thread_id": thread_id,
                    "failed": wait_failed,
                }),
            );
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_kills_registered_sleep_process() {
        let mut child = Command::new("sleep")
            .arg("30")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sleep");
        let pid = child.id();
        let id = format!("test-{}", pid);
        register_pid(&id, pid);
        let msg = stop_thread(&id).unwrap();
        assert!(msg.contains("Stopped"), "{msg}");
        let _ = child.wait();
        let again = stop_thread(&id).unwrap();
        assert!(again.contains("No running"), "{again}");
    }

    #[test]
    fn spawn_opts_do_not_put_keys_on_argv() {
        let argv = crate::security::agy_argv(AgySpawnOpts {
            conversation_id: Some("abc"),
            mode: "ask",
            prompt: "hello",
        });
        assert_eq!(argv[0], "agy");
        assert!(argv.contains(&"--conversation".to_string()));
        assert!(argv.contains(&"abc".to_string()));
        assert!(!crate::security::argv_contains_api_key(&argv));
    }
}
