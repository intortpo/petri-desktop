use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Thread {
    pub id: String,
    pub project_path: Option<String>,
    pub mode: String,
    pub model: String,
    pub parent_id: Option<String>,
    pub agy_conversation_id: Option<String>,
    pub status: String,
    pub summary: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
    pub agy_event_raw: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: String,
    pub thread_id: String,
    pub child_thread_id: Option<String>,
    pub title: String,
    pub prompt: String,
    pub status: String,
    pub result: Option<String>,
}

pub struct Store {
    pub conn: Mutex<Connection>,
    #[allow(dead_code)]
    db_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForkRecords {
    pub thread: Thread,
    pub task: Task,
    pub card: Message,
}

impl Store {
    pub fn new() -> Result<Self> {
        let paths = crate::paths::MeshPaths::from_env();
        let _ = paths.ensure();
        Self::open(&paths.db())
    }

    pub fn open(db_path: &std::path::Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "busy_timeout", 4000);
        
        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                project_path TEXT,
                mode TEXT NOT NULL,
                model TEXT NOT NULL,
                parent_id TEXT,
                agy_conversation_id TEXT,
                status TEXT NOT NULL,
                summary TEXT
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                agy_event_raw TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (thread_id) REFERENCES threads(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                child_thread_id TEXT,
                title TEXT NOT NULL,
                prompt TEXT NOT NULL,
                status TEXT NOT NULL,
                result TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (thread_id) REFERENCES threads(id)
            )",
            [],
        )?;
        
        Ok(Store {
            conn: Mutex::new(conn),
            db_path: db_path.to_path_buf(),
        })
    }

    #[allow(dead_code)]
    pub fn db_path(&self) -> &std::path::Path {
        &self.db_path
    }

    pub fn short_title(prompt: &str) -> String {
        let t = prompt.trim();
        if t.chars().count() <= 48 {
            t.to_string()
        } else {
            format!("{}…", t.chars().take(48).collect::<String>())
        }
    }

    /// Child thread + running task + parent fork card. Does not spawn `agy`.
    pub fn fork_from(
        &self,
        parent_id: &str,
        prompt: &str,
        mode: Option<&str>,
    ) -> Result<ForkRecords> {
        let parent = self.get_thread(parent_id)?;
        let child_mode = mode
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                if parent.mode == "orchestrator" {
                    "code".to_string()
                } else {
                    parent.mode.clone()
                }
            });
        let title = Self::short_title(prompt);
        let mut child = self.create_thread(
            parent.project_path.clone(),
            child_mode,
            parent.model.clone(),
            Some(parent_id.to_string()),
        )?;
        self.set_thread_summary(&child.id, &title)?;
        self.set_thread_status(&child.id, "running")?;
        child.summary = Some(title.clone());
        child.status = "running".to_string();
        let task = self.create_task(
            parent_id,
            Some(child.id.clone()),
            &title,
            prompt,
            "running",
        )?;
        let card_body = serde_json::json!({
            "child_id": child.id,
            "task_id": task.id,
            "title": title,
            "status": "running",
            "result": null,
        })
        .to_string();
        let card = self.add_message(parent_id, "fork", &card_body, None)?;
        Ok(ForkRecords {
            thread: child,
            task,
            card,
        })
    }

    pub fn create_thread(&self, project_path: Option<String>, mode: String, model: String, parent_id: Option<String>) -> Result<Thread> {
        let id = Uuid::new_v4().to_string();
        let thread = Thread {
            id: id.clone(),
            project_path,
            mode,
            model,
            parent_id,
            agy_conversation_id: None,
            status: "active".to_string(),
            summary: None,
        };
        
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO threads (id, project_path, mode, model, parent_id, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &thread.id,
                &thread.project_path,
                &thread.mode,
                &thread.model,
                &thread.parent_id,
                &thread.status,
            ),
        )?;
        
        Ok(thread)
    }
    
    pub fn set_agy_conversation_id(&self, thread_id: &str, agy_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE threads SET agy_conversation_id = ?1 WHERE id = ?2",
            (agy_id, thread_id),
        )?;
        Ok(())
    }

    pub fn add_message(&self, thread_id: &str, role: &str, content: &str, raw: Option<String>) -> Result<Message> {
        let id = Uuid::new_v4().to_string();
        let msg = Message {
            id: id.clone(),
            thread_id: thread_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            agy_event_raw: raw.clone(),
        };
        
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (id, thread_id, role, content, agy_event_raw) VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &msg.id,
                &msg.thread_id,
                &msg.role,
                &msg.content,
                &msg.agy_event_raw,
            ),
        )?;
        
        Ok(msg)
    }
    
    pub fn get_messages(&self, thread_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, thread_id, role, content, agy_event_raw FROM messages WHERE thread_id = ?1 ORDER BY created_at ASC")?;
        let msg_iter = stmt.query_map([thread_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                agy_event_raw: row.get(4)?,
            })
        })?;
        
        let mut messages = Vec::new();
        for msg in msg_iter {
            messages.push(msg?);
        }
        Ok(messages)
    }
    
    pub fn get_thread(&self, thread_id: &str) -> Result<Thread> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, project_path, mode, model, parent_id, agy_conversation_id, status, summary FROM threads WHERE id = ?1")?;
        let thread = stmt.query_row([thread_id], |row| {
            Ok(Thread {
                id: row.get(0)?,
                project_path: row.get(1)?,
                mode: row.get(2)?,
                model: row.get(3)?,
                parent_id: row.get(4)?,
                agy_conversation_id: row.get(5)?,
                status: row.get(6)?,
                summary: row.get(7)?,
            })
        })?;
        Ok(thread)
    }

    fn map_thread(row: &rusqlite::Row) -> Result<Thread> {
        Ok(Thread {
            id: row.get(0)?,
            project_path: row.get(1)?,
            mode: row.get(2)?,
            model: row.get(3)?,
            parent_id: row.get(4)?,
            agy_conversation_id: row.get(5)?,
            status: row.get(6)?,
            summary: row.get(7)?,
        })
    }

    pub fn list_threads(&self, project_path: Option<&str>) -> Result<Vec<Thread>> {
        let conn = self.conn.lock().unwrap();
        let sql = if project_path.is_some() {
            "SELECT id, project_path, mode, model, parent_id, agy_conversation_id, status, summary FROM threads WHERE project_path = ?1 ORDER BY rowid DESC"
        } else {
            "SELECT id, project_path, mode, model, parent_id, agy_conversation_id, status, summary FROM threads ORDER BY rowid DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let mut threads = Vec::new();
        if let Some(path) = project_path {
            let iter = stmt.query_map([path], Self::map_thread)?;
            for t in iter {
                threads.push(t?);
            }
        } else {
            let iter = stmt.query_map([], Self::map_thread)?;
            for t in iter {
                threads.push(t?);
            }
        }
        Ok(threads)
    }

    pub fn list_children(&self, parent_id: &str) -> Result<Vec<Thread>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_path, mode, model, parent_id, agy_conversation_id, status, summary FROM threads WHERE parent_id = ?1 ORDER BY rowid ASC",
        )?;
        let iter = stmt.query_map([parent_id], Self::map_thread)?;
        let mut threads = Vec::new();
        for t in iter {
            threads.push(t?);
        }
        Ok(threads)
    }

    pub fn thread_ancestors(&self, thread_id: &str) -> Result<Vec<Thread>> {
        let mut chain = Vec::new();
        let mut current = self.get_thread(thread_id)?;
        loop {
            chain.push(current.clone());
            match current.parent_id.clone() {
                Some(pid) => current = self.get_thread(&pid)?,
                None => break,
            }
            if chain.len() > 32 {
                break;
            }
        }
        chain.reverse();
        Ok(chain)
    }

    pub fn set_thread_summary(&self, thread_id: &str, summary: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE threads SET summary = ?1 WHERE id = ?2",
            (summary, thread_id),
        )?;
        Ok(())
    }

    pub fn set_thread_status(&self, thread_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE threads SET status = ?1 WHERE id = ?2",
            (status, thread_id),
        )?;
        Ok(())
    }

    pub fn set_thread_mode(&self, thread_id: &str, mode: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE threads SET mode = ?1 WHERE id = ?2",
            (mode, thread_id),
        )?;
        Ok(())
    }

    /// Persist a composer `/mode` onto the thread and return the mode that
    /// must be used for fences + `agy` spawn (status bar and sqlite stay aligned).
    pub fn apply_mode(&self, thread_id: &str, requested: Option<&str>) -> Result<String> {
        let thread = self.get_thread(thread_id)?;
        let mode = requested
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .unwrap_or(&thread.mode)
            .to_string();
        if mode != thread.mode {
            self.set_thread_mode(thread_id, &mode)?;
        }
        Ok(mode)
    }

    pub fn find_thread(&self, query: &str) -> Result<Option<Thread>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(None);
        }
        if let Ok(thread) = self.get_thread(q) {
            return Ok(Some(thread));
        }
        let conn = self.conn.lock().unwrap();
        let like = format!("%{}%", q);
        let mut stmt = conn.prepare(
            "SELECT id, project_path, mode, model, parent_id, agy_conversation_id, status, summary
             FROM threads
             WHERE id LIKE ?1 OR IFNULL(summary, '') LIKE ?2
             ORDER BY rowid DESC LIMIT 1",
        )?;
        let mut rows = stmt.query((format!("{}%", q), like))?;
        if let Some(row) = rows.next()? {
            return Ok(Some(Self::map_thread(row)?));
        }
        Ok(None)
    }

    pub fn list_projects(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT project_path FROM threads WHERE project_path IS NOT NULL ORDER BY project_path",
        )?;
        let iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut projects = Vec::new();
        for p in iter {
            projects.push(p?);
        }
        Ok(projects)
    }

    pub fn update_message_content(&self, message_id: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE messages SET content = ?1 WHERE id = ?2",
            (content, message_id),
        )?;
        Ok(())
    }

    pub fn find_fork_message(&self, thread_id: &str, child_id: &str) -> Result<Option<Message>> {
        let messages = self.get_messages(thread_id)?;
        Ok(messages.into_iter().find(|m| {
            m.role == "fork" && m.content.contains(child_id)
        }))
    }

    pub fn create_task(
        &self,
        thread_id: &str,
        child_thread_id: Option<String>,
        title: &str,
        prompt: &str,
        status: &str,
    ) -> Result<Task> {
        let id = Uuid::new_v4().to_string();
        let task = Task {
            id: id.clone(),
            thread_id: thread_id.to_string(),
            child_thread_id,
            title: title.to_string(),
            prompt: prompt.to_string(),
            status: status.to_string(),
            result: None,
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, thread_id, child_thread_id, title, prompt, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &task.id,
                &task.thread_id,
                &task.child_thread_id,
                &task.title,
                &task.prompt,
                &task.status,
            ),
        )?;
        Ok(task)
    }

    fn map_task(row: &rusqlite::Row) -> Result<Task> {
        Ok(Task {
            id: row.get(0)?,
            thread_id: row.get(1)?,
            child_thread_id: row.get(2)?,
            title: row.get(3)?,
            prompt: row.get(4)?,
            status: row.get(5)?,
            result: row.get(6)?,
        })
    }

    pub fn list_tasks(&self, thread_id: &str) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, thread_id, child_thread_id, title, prompt, status, result FROM tasks WHERE thread_id = ?1 ORDER BY rowid ASC",
        )?;
        let iter = stmt.query_map([thread_id], Self::map_task)?;
        let mut tasks = Vec::new();
        for t in iter {
            tasks.push(t?);
        }
        Ok(tasks)
    }

    pub fn get_task_by_child(&self, child_thread_id: &str) -> Result<Option<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, thread_id, child_thread_id, title, prompt, status, result FROM tasks WHERE child_thread_id = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query([child_thread_id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(Self::map_task(row)?));
        }
        Ok(None)
    }

    pub fn update_task(
        &self,
        task_id: &str,
        status: Option<&str>,
        result: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        if let Some(status) = status {
            conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                (status, task_id),
            )?;
        }
        if let Some(result) = result {
            conn.execute(
                "UPDATE tasks SET result = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                (result, task_id),
            )?;
        }
        Ok(())
    }

    pub fn last_assistant_excerpt(&self, thread_id: &str, max_chars: usize) -> Result<Option<String>> {
        let messages = self.get_messages(thread_id)?;
        let last = messages.iter().rev().find(|m| m.role == "assistant");
        Ok(last.map(|m| {
            let t = m.content.trim();
            if t.chars().count() > max_chars {
                format!("{}…", t.chars().take(max_chars).collect::<String>())
            } else {
                t.to_string()
            }
        }))
    }

    fn write_fork_card(&self, parent_id: &str, child_id: &str, task: &Task, status: &str, result: &str) -> Result<()> {
        if let Some(fork_msg) = self.find_fork_message(parent_id, child_id)? {
            let card = serde_json::json!({
                "child_id": child_id,
                "task_id": task.id,
                "title": task.title,
                "status": status,
                "result": result,
            });
            self.update_message_content(&fork_msg.id, &card.to_string())?;
        }
        Ok(())
    }

    pub fn complete_child_turn(&self, thread_id: &str, failed: bool) -> Result<Option<Task>> {
        let child = self.get_thread(thread_id)?;
        if child.parent_id.is_none() {
            return Ok(None);
        }
        let excerpt = if failed {
            "agy child exited with an error".to_string()
        } else {
            self.last_assistant_excerpt(thread_id, 280)?
                .unwrap_or_else(|| "Child turn finished with no assistant text.".to_string())
        };
        let status = if failed { "failed" } else { "done" };
        self.set_thread_status(thread_id, status)?;
        self.set_thread_summary(thread_id, &excerpt)?;

        let Some(mut task) = self.get_task_by_child(thread_id)? else {
            return Ok(None);
        };
        self.update_task(&task.id, Some(status), Some(&excerpt))?;
        task.status = status.to_string();
        task.result = Some(excerpt.clone());

        if let Some(parent_id) = child.parent_id {
            let _ = self.write_fork_card(&parent_id, thread_id, &task, status, &excerpt);
        }
        Ok(Some(task))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (PathBuf, Store) {
        let dir = std::env::temp_dir().join(format!("mesh-store-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("mesh.db");
        let store = Store::open(&db).unwrap();
        (dir, store)
    }

    #[test]
    fn fork_from_creates_child_task_and_parent_card() {
        let (dir, store) = temp_store();
        let parent = store
            .create_thread(
                Some("/tmp/demo-proj".into()),
                "orchestrator".into(),
                "pro".into(),
                None,
            )
            .unwrap();
        let forked = store
            .fork_from(&parent.id, "Fix the failing auth test", None)
            .unwrap();

        assert_eq!(forked.thread.parent_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(forked.thread.status, "running");
        assert_eq!(forked.thread.mode, "code");
        assert_eq!(forked.task.status, "running");
        assert_eq!(forked.task.thread_id, parent.id);
        assert_eq!(forked.task.child_thread_id.as_deref(), Some(forked.thread.id.as_str()));
        assert_eq!(forked.card.role, "fork");
        assert!(forked.card.content.contains(&forked.thread.id));

        store
            .add_message(&forked.thread.id, "assistant", "patched login.rs and tests passed", None)
            .unwrap();
        let done = store.complete_child_turn(&forked.thread.id, false).unwrap().unwrap();
        assert_eq!(done.status, "done");
        assert!(done.result.unwrap().contains("patched login.rs"));

        let card = store
            .find_fork_message(&parent.id, &forked.thread.id)
            .unwrap()
            .unwrap();
        assert!(card.content.contains("\"status\":\"done\""));
        assert!(card.content.contains("patched login.rs"));

        let trail = store.thread_ancestors(&forked.thread.id).unwrap();
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].id, parent.id);
        assert_eq!(trail[1].id, forked.thread.id);

        let found = store.find_thread(&forked.thread.id[..8]).unwrap().unwrap();
        assert_eq!(found.id, forked.thread.id);
        let by_excerpt = store.find_thread("patched login").unwrap().unwrap();
        assert_eq!(by_excerpt.id, forked.thread.id);

        let tasks = store.list_tasks(&parent.id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, "done");

        let projects = store.list_projects().unwrap();
        assert!(projects.iter().any(|p| p == "/tmp/demo-proj"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn apply_mode_ask_on_code_thread_persists_and_drives_ask_fences() {
        let (dir, store) = temp_store();
        let thread = store
            .create_thread(Some("/tmp/mode-proj".into()), "code".into(), "pro".into(), None)
            .unwrap();
        assert_eq!(thread.mode, "code");

        let applied = store.apply_mode(&thread.id, Some("ask")).unwrap();
        assert_eq!(applied, "ask");
        let stored = store.get_thread(&thread.id).unwrap();
        assert_eq!(stored.mode, "ask");

        // Same helpers start_turn uses after apply_mode.
        let payload = crate::security::prepare_turn_payload(&applied, "pack", "edit this file");
        assert!(payload.contains("[MODE FENCE: ASK]"), "{payload}");
        assert!(
            payload.contains("Write tools are not granted") || payload.contains("read-only"),
            "{payload}"
        );
        let argv = crate::security::agy_argv(crate::security::AgySpawnOpts {
            conversation_id: None,
            mode: &applied,
            prompt: &payload,
        });
        assert!(argv.contains(&"--sandbox".to_string()), "{argv:?}");
        assert!(argv.contains(&"plan".to_string()), "{argv:?}");

        let _ = std::fs::remove_dir_all(dir);
    }
}
