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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub root_path: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub last_active_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub root_path: String,
    pub default_mode: String,
    pub default_model: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectWithStats {
    #[serde(flatten)]
    pub project: Project,
    pub thread_count: usize,
    pub member_count: usize,
    pub repo_count: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ProjectMember {
    pub project_id: String,
    pub user_id: String,
    pub role: String,
    pub assigned_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ProjectRepo {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub html_url: Option<String>,
    pub local_path: String,
    pub branch: String,
    pub is_primary: bool,
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
            "CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                slug TEXT NOT NULL UNIQUE,
                description TEXT,
                root_path TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                login TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                email TEXT,
                avatar_url TEXT,
                role TEXT NOT NULL DEFAULT 'member',
                status TEXT NOT NULL DEFAULT 'active',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_active_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                name TEXT NOT NULL,
                slug TEXT NOT NULL UNIQUE,
                description TEXT,
                root_path TEXT NOT NULL,
                default_mode TEXT NOT NULL DEFAULT 'code',
                default_model TEXT NOT NULL DEFAULT 'Gemini 3.1 Pro',
                status TEXT NOT NULL DEFAULT 'active',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (workspace_id) REFERENCES workspaces(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_members (
                project_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'contributor',
                assigned_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (project_id, user_id),
                FOREIGN KEY (project_id) REFERENCES projects(id),
                FOREIGN KEY (user_id) REFERENCES users(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_repos (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                html_url TEXT,
                local_path TEXT NOT NULL,
                branch TEXT NOT NULL DEFAULT 'main',
                is_primary INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY (project_id) REFERENCES projects(id)
            )",
            [],
        )?;

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

        // Seed initial workspace and users if empty
        let ws_count: i64 = conn.query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0)).unwrap_or(0);
        if ws_count == 0 {
            let _ = conn.execute(
                "INSERT INTO workspaces (id, name, slug, description, root_path) VALUES (?1, ?2, ?3, ?4, ?5)",
                ("ws-default", "Primary Mesh", "primary-mesh", "Default local mesh workspace", "."),
            );
        }

        let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0)).unwrap_or(0);
        if user_count == 0 {
            let _ = conn.execute(
                "INSERT INTO users (id, login, display_name, email, role, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                ("u-hideo", "hideo", "Hideo", "intortpo@gmail.com", "sysmin", "active"),
            );
            let _ = conn.execute(
                "INSERT INTO users (id, login, display_name, email, role, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                ("u-intortpo", "intortpo", "Intortpo", "intortpo@gmail.com", "sysmin", "active"),
            );
            let _ = conn.execute(
                "INSERT INTO users (id, login, display_name, role, status) VALUES (?1, ?2, ?3, ?4, ?5)",
                ("u-agy", "agy", "Antigravity Agent", "member", "active"),
            );
        }

        let prj_count: i64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0)).unwrap_or(0);
        if prj_count == 0 {
            let cur_dir = std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| ".".to_string());
            let _ = conn.execute(
                "INSERT INTO projects (id, workspace_id, name, slug, description, root_path, default_mode, default_model) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    "prj-default",
                    "ws-default",
                    "Petri Desktop",
                    "petri-desktop",
                    "Petri AI Pair-Programming Workspace",
                    &cur_dir,
                    "code",
                    "Gemini 3.1 Pro",
                ),
            );
            let _ = conn.execute(
                "INSERT OR IGNORE INTO project_members (project_id, user_id, role) VALUES (?1, ?2, ?3)",
                ("prj-default", "u-hideo", "lead"),
            );
            let _ = conn.execute(
                "INSERT INTO project_repos (id, project_id, name, html_url, local_path, branch, is_primary) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                ("repo-default", "prj-default", "petri-desktop", "https://github.com/intortpo/petri-desktop", &cur_dir, "main", 1),
            );
        }
        
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

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, slug, description, root_path, created_at FROM workspaces ORDER BY name")?;
        let rows = stmt.query_map([], |r| {
            Ok(Workspace {
                id: r.get(0)?,
                name: r.get(1)?,
                slug: r.get(2)?,
                description: r.get(3)?,
                root_path: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_default_workspace(&self) -> Result<Workspace> {
        let list = self.list_workspaces()?;
        if let Some(w) = list.into_iter().next() {
            Ok(w)
        } else {
            let id = "ws-default".to_string();
            let w = Workspace {
                id: id.clone(),
                name: "Primary Mesh".into(),
                slug: "primary-mesh".into(),
                description: Some("Default local mesh workspace".into()),
                root_path: ".".into(),
                created_at: "".into(),
            };
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT OR IGNORE INTO workspaces (id, name, slug, description, root_path) VALUES (?1, ?2, ?3, ?4, ?5)",
                (&w.id, &w.name, &w.slug, &w.description, &w.root_path),
            )?;
            Ok(w)
        }
    }

    pub fn list_users(&self) -> Result<Vec<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, login, display_name, email, avatar_url, role, status, created_at, last_active_at FROM users ORDER BY role = 'sysmin' DESC, login ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(User {
                id: r.get(0)?,
                login: r.get(1)?,
                display_name: r.get(2)?,
                email: r.get(3)?,
                avatar_url: r.get(4)?,
                role: r.get(5)?,
                status: r.get(6)?,
                created_at: r.get(7)?,
                last_active_at: r.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_user(&self, id: &str) -> Result<Option<User>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, login, display_name, email, avatar_url, role, status, created_at, last_active_at FROM users WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |r| {
            Ok(User {
                id: r.get(0)?,
                login: r.get(1)?,
                display_name: r.get(2)?,
                email: r.get(3)?,
                avatar_url: r.get(4)?,
                role: r.get(5)?,
                status: r.get(6)?,
                created_at: r.get(7)?,
                last_active_at: r.get(8)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn create_user(&self, login: &str, display_name: &str, email: Option<&str>, role: &str, status: &str) -> Result<User> {
        let id = format!("u-{}", &Uuid::new_v4().to_string()[..8]);
        let user = User {
            id: id.clone(),
            login: login.trim().to_lowercase(),
            display_name: display_name.trim().to_string(),
            email: email.map(|s| s.trim().to_string()),
            avatar_url: None,
            role: role.trim().to_string(),
            status: status.trim().to_string(),
            created_at: "".into(),
            last_active_at: "".into(),
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (id, login, display_name, email, avatar_url, role, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&user.id, &user.login, &user.display_name, &user.email, &user.avatar_url, &user.role, &user.status),
        )?;
        drop(conn);
        self.get_user(&id).map(|opt| opt.unwrap_or(user))
    }

    pub fn update_user(&self, id: &str, role: &str, status: &str) -> Result<User> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET role = ?1, status = ?2, last_active_at = CURRENT_TIMESTAMP WHERE id = ?3",
            (role, status, id),
        )?;
        drop(conn);
        self.get_user(id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn delete_user(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let sysmin_count: i64 = conn.query_row("SELECT COUNT(*) FROM users WHERE role = 'sysmin'", [], |r| r.get(0))?;
        let is_sysmin: bool = conn.query_row("SELECT role = 'sysmin' FROM users WHERE id = ?1", [id], |r| r.get(0)).unwrap_or(false);
        if is_sysmin && sysmin_count <= 1 {
            return Ok(false);
        }
        let _ = conn.execute("DELETE FROM project_members WHERE user_id = ?1", [id]);
        let changed = conn.execute("DELETE FROM users WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }

    pub fn list_db_projects(&self) -> Result<Vec<ProjectWithStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT p.id, p.workspace_id, p.name, p.slug, p.description, p.root_path, p.default_mode, p.default_model, p.status, p.created_at, p.updated_at,
                    (SELECT COUNT(*) FROM threads t WHERE t.project_path = p.root_path) as thread_count,
                    (SELECT COUNT(*) FROM project_members pm WHERE pm.project_id = p.id) as member_count,
                    (SELECT COUNT(*) FROM project_repos pr WHERE pr.project_id = p.id) as repo_count
             FROM projects p
             ORDER BY p.status = 'active' DESC, p.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            let project = Project {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                name: r.get(2)?,
                slug: r.get(3)?,
                description: r.get(4)?,
                root_path: r.get(5)?,
                default_mode: r.get(6)?,
                default_model: r.get(7)?,
                status: r.get(8)?,
                created_at: r.get(9)?,
                updated_at: r.get(10)?,
            };
            let thread_count: i64 = r.get(11)?;
            let member_count: i64 = r.get(12)?;
            let repo_count: i64 = r.get(13)?;
            Ok(ProjectWithStats {
                project,
                thread_count: thread_count as usize,
                member_count: member_count as usize,
                repo_count: repo_count as usize,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, name, slug, description, root_path, default_mode, default_model, status, created_at, updated_at FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], |r| {
            Ok(Project {
                id: r.get(0)?,
                workspace_id: r.get(1)?,
                name: r.get(2)?,
                slug: r.get(3)?,
                description: r.get(4)?,
                root_path: r.get(5)?,
                default_mode: r.get(6)?,
                default_model: r.get(7)?,
                status: r.get(8)?,
                created_at: r.get(9)?,
                updated_at: r.get(10)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn create_project(
        &self,
        workspace_id: &str,
        name: &str,
        slug: &str,
        description: Option<&str>,
        root_path: &str,
        default_mode: Option<&str>,
        default_model: Option<&str>,
    ) -> Result<Project> {
        let id = format!("prj-{}", &Uuid::new_v4().to_string()[..8]);
        let mode = default_mode.unwrap_or("code");
        let model = default_model.unwrap_or("Gemini 3.1 Pro");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, workspace_id, name, slug, description, root_path, default_mode, default_model)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (&id, workspace_id, name, slug, &description, root_path, mode, model),
        )?;
        drop(conn);
        self.get_project(&id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn update_project(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        root_path: &str,
        default_mode: &str,
        default_model: &str,
        status: &str,
    ) -> Result<Project> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, root_path = ?3, default_mode = ?4, default_model = ?5, status = ?6, updated_at = CURRENT_TIMESTAMP WHERE id = ?7",
            (name, &description, root_path, default_mode, default_model, status, id),
        )?;
        drop(conn);
        self.get_project(id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn archive_project(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute("UPDATE projects SET status = 'archived', updated_at = CURRENT_TIMESTAMP WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }

    pub fn delete_project(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM project_members WHERE project_id = ?1", [id]);
        let _ = conn.execute("DELETE FROM project_repos WHERE project_id = ?1", [id]);
        let changed = conn.execute("DELETE FROM projects WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }

    #[allow(dead_code)]
    pub fn assign_project_member(&self, project_id: &str, user_id: &str, role: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO project_members (project_id, user_id, role, assigned_at) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)",
            (project_id, user_id, role),
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_project_member(&self, project_id: &str, user_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            (project_id, user_id),
        )?;
        Ok(())
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

    #[test]
    fn user_management_crud_and_last_sysmin_protection() {
        let (dir, store) = temp_store();

        // Seeding checks
        let users = store.list_users().unwrap();
        assert!(users.len() >= 3);
        assert!(users.iter().any(|u| u.login == "hideo" && u.role == "sysmin"));
        assert!(users.iter().any(|u| u.login == "intortpo" && u.role == "sysmin"));

        // Create user
        let new_user = store
            .create_user("testuser", "Test User", Some("test@example.com"), "member", "active")
            .unwrap();
        assert_eq!(new_user.login, "testuser");
        assert_eq!(new_user.role, "member");

        // Update user
        let updated = store.update_user(&new_user.id, "admin", "suspended").unwrap();
        assert_eq!(updated.role, "admin");
        assert_eq!(updated.status, "suspended");

        // Delete user
        let deleted = store.delete_user(&new_user.id).unwrap();
        assert!(deleted);

        // Try to delete sysmin users:
        let hideo = users.iter().find(|u| u.login == "hideo").unwrap();
        let intortpo = users.iter().find(|u| u.login == "intortpo").unwrap();

        // First deletion should succeed because there's 2 sysmins
        let del1 = store.delete_user(&hideo.id).unwrap();
        assert!(del1);

        // Second deletion must return false because intortpo is the LAST sysmin
        let del2 = store.delete_user(&intortpo.id).unwrap();
        assert!(!del2, "Must prevent deleting the last sysmin");
        assert!(store.get_user(&intortpo.id).unwrap().is_some());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn project_management_crud_and_relational_stats() {
        let (dir, store) = temp_store();

        let ws = store.get_default_workspace().unwrap();
        assert_eq!(ws.id, "ws-default");

        let initial_projects = store.list_db_projects().unwrap();
        assert!(!initial_projects.is_empty());

        let created = store
            .create_project(
                &ws.id,
                "New Project",
                "new-project",
                Some("Test project"),
                "/tmp/test-project",
                Some("code"),
                Some("Gemini 3.1 Pro"),
            )
            .unwrap();
        assert_eq!(created.name, "New Project");
        assert_eq!(created.slug, "new-project");
        assert_eq!(created.status, "active");

        let updated = store
            .update_project(
                &created.id,
                "Updated Project",
                Some("Updated description"),
                "/tmp/test-project-updated",
                "dev",
                "Gemini 3.1 Pro",
                "active",
            )
            .unwrap();
        assert_eq!(updated.name, "Updated Project");

        let archived = store.archive_project(&created.id).unwrap();
        assert!(archived);

        let deleted = store.delete_project(&created.id).unwrap();
        assert!(deleted);

        let _ = std::fs::remove_dir_all(dir);
    }
}
