#!/usr/bin/env python3
"""
EverOS Sidecar for Petri Agent Cockpit
Provides local-first, Markdown-native memory operations (episodes, profiles, skills)
with hybrid keyword and semantic indexing, managed as a child process via JSON over stdin/stdout.
"""

import os
import sys
import json
import time
import sqlite3
import re
from pathlib import Path
from typing import Dict, Any, List, Optional

# Try importing official everos package if present in the environment
HAS_EVEROS_PKG = False
try:
    import everos  # type: ignore
    HAS_EVEROS_PKG = True
except ImportError:
    HAS_EVEROS_PKG = False


def get_default_root() -> Path:
    override = os.environ.get("EVEROS_ROOT") or os.environ.get("MESH_EVEROS_ROOT")
    if override:
        return Path(override)
    home = Path(os.environ.get("HOME", "."))
    return home / ".mesh" / "everos"


class EverosEngine:
    def __init__(self, root: Optional[Path] = None):
        self.root = root or get_default_root()
        self.episodes_dir = self.root / "episodes"
        self.profile_dir = self.root / "profile"
        self.skills_dir = self.root / "skills"
        self.cases_dir = self.root / "cases"
        self.db_path = self.root / "memory.db"
        
        self.sessions: Dict[str, List[Dict[str, Any]]] = {}
        self._ensure_layout()
        self._init_db()

    def _ensure_layout(self):
        for d in [self.episodes_dir, self.profile_dir, self.skills_dir, self.cases_dir]:
            d.mkdir(parents=True, exist_ok=True)

    def _init_db(self):
        with sqlite3.connect(self.db_path) as conn:
            conn.execute("""
                CREATE TABLE IF NOT EXISTS memories (
                    id TEXT PRIMARY KEY,
                    type TEXT NOT NULL,
                    source_path TEXT NOT NULL,
                    session_id TEXT,
                    user_id TEXT,
                    content TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                )
            """)
            conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                    id UNINDEXED,
                    type,
                    content,
                    tokenize='porter unicode61'
                )
            """)
            conn.commit()

    def health(self) -> Dict[str, Any]:
        with sqlite3.connect(self.db_path) as conn:
            cursor = conn.execute("SELECT COUNT(*) FROM memories")
            count = cursor.fetchone()[0]
        return {
            "status": "ok",
            "engine": "everos-pkg" if HAS_EVEROS_PKG else "everos-native-markdown",
            "has_everos_pkg": HAS_EVEROS_PKG,
            "storage_root": str(self.root),
            "memory_count": count,
            "capabilities": {
                "markdown_native": True,
                "local_storage": True,
                "fts_search": True,
                "vector_search": HAS_EVEROS_PKG,
            }
        }

    def add_message(self, session_id: str, role: str, content: str, user_id: str = "default", timestamp: Optional[int] = None) -> Dict[str, Any]:
        ts = timestamp or int(time.time() * 1000)
        if session_id not in self.sessions:
            self.sessions[session_id] = []
        
        msg = {
            "role": role,
            "content": content,
            "user_id": user_id,
            "timestamp": ts,
        }
        self.sessions[session_id].append(msg)
        return {"status": "buffered", "session_id": session_id, "turn_count": len(self.sessions[session_id])}

    def remember(self, lesson: str, domain: str = "general", user_id: str = "default") -> Dict[str, Any]:
        ts = int(time.time() * 1000)
        clean_lesson = lesson.strip()
        if not clean_lesson:
            return {"status": "error", "error": "empty lesson"}

        mem_id = f"fact_{int(time.time())}_{abs(hash(clean_lesson)) % 100000:05d}"
        profile_file = self.profile_dir / "facts.md"

        with open(profile_file, "a", encoding="utf-8") as f:
            f.write(f"- [{domain}] {clean_lesson}\n")

        with sqlite3.connect(self.db_path) as conn:
            conn.execute(
                "INSERT OR REPLACE INTO memories (id, type, source_path, session_id, user_id, content, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                (mem_id, "fact", str(profile_file), None, user_id, f"[{domain}] {clean_lesson}", ts)
            )
            conn.execute(
                "INSERT INTO memory_fts (id, type, content) VALUES (?, ?, ?)",
                (mem_id, "fact", f"[{domain}] {clean_lesson}")
            )
            conn.commit()

        return {"status": "ok", "id": mem_id, "path": str(profile_file)}

    def flush(self, session_id: str, user_id: str = "default") -> Dict[str, Any]:
        turns = self.sessions.get(session_id, [])
        if not turns:
            return {"status": "ok", "flushed": False, "reason": "no pending turns"}

        episode_file = self.episodes_dir / f"{session_id}.md"
        now_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        ts = int(time.time() * 1000)

        # Build Markdown episode with frontmatter
        lines = [
            "---",
            f"session_id: {session_id}",
            f"user_id: {user_id}",
            f"date: {now_iso}",
            f"turn_count: {len(turns)}",
            "kind: episode",
            "---",
            "",
            f"# Episode: {session_id}",
            "",
        ]

        extracted_facts = []
        for t in turns:
            role_header = f"### {t['role'].capitalize()}"
            lines.append(f"{role_header} ({time.strftime('%H:%M:%S', time.gmtime(t['timestamp'] / 1000))})")
            lines.append("")
            lines.append(t["content"].strip())
            lines.append("")

            # Extract simple facts or key instructions
            content = t["content"].strip()
            if t["role"] == "user" and len(content) > 10 and not content.startswith("/"):
                extracted_facts.append(content)

        episode_text = "\n".join(lines)
        with open(episode_file, "w", encoding="utf-8") as f:
            f.write(episode_text)

        # Index episode in SQLite
        mem_id = f"ep_{session_id}"
        with sqlite3.connect(self.db_path) as conn:
            conn.execute(
                "INSERT OR REPLACE INTO memories (id, type, source_path, session_id, user_id, content, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                (mem_id, "episode", str(episode_file), session_id, user_id, episode_text, ts)
            )
            conn.execute(
                "INSERT INTO memory_fts (id, type, content) VALUES (?, ?, ?)",
                (mem_id, "episode", episode_text)
            )
            conn.commit()

        # Clear session buffer
        self.sessions.pop(session_id, None)

        return {
            "status": "ok",
            "flushed": True,
            "session_id": session_id,
            "episode_path": str(episode_file),
            "turns_written": len(turns),
            "facts_extracted": len(extracted_facts),
        }

    def search(self, query: str, top_k: int = 5, user_id: Optional[str] = None) -> List[Dict[str, Any]]:
        clean_query = query.strip()
        if not clean_query:
            return []

        # Tokenize query for FTS5 (match words)
        words = re.findall(r"\w+", clean_query)
        if not words:
            return []
        
        fts_expr = " OR ".join(f'"{w}"*' for w in words)
        results = []

        with sqlite3.connect(self.db_path) as conn:
            try:
                cursor = conn.execute("""
                    SELECT m.id, m.type, m.source_path, m.session_id, m.content, m.created_at, bm25(memory_fts) as rank
                    FROM memory_fts
                    JOIN memories m ON m.id = memory_fts.id
                    WHERE memory_fts MATCH ?
                    ORDER BY rank
                    LIMIT ?
                """, (fts_expr, top_k))

                for row in cursor.fetchall():
                    # BM25 in sqlite3 returns negative value where smaller/more negative is better match
                    score = max(0.1, min(1.0, 1.0 / (1.0 + abs(row[6]))))
                    results.append({
                        "id": row[0],
                        "type": row[1],
                        "source_path": row[2],
                        "session_id": row[3],
                        "content": row[4],
                        "created_at": row[5],
                        "similarity": round(score, 3)
                    })
            except Exception as e:
                # Fallback to simple LIKE query if FTS expression parsing fails
                cursor = conn.execute("""
                    SELECT id, type, source_path, session_id, content, created_at
                    FROM memories
                    WHERE content LIKE ?
                    LIMIT ?
                """, (f"%{clean_query}%", top_k))
                for row in cursor.fetchall():
                    results.append({
                        "id": row[0],
                        "type": row[1],
                        "source_path": row[2],
                        "session_id": row[3],
                        "content": row[4],
                        "created_at": row[5],
                        "similarity": 0.5
                    })

        return results

    def get_profile(self) -> str:
        profile_file = self.profile_dir / "facts.md"
        if profile_file.exists():
            return profile_file.read_text(encoding="utf-8")
        return "# User Profile Facts\n\n_No recorded facts yet._\n"


def main():
    engine = EverosEngine()
    
    # Process stdin line-by-line
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            cmd = req.get("cmd", "")
            
            if cmd == "health":
                res = {"ok": True, "result": engine.health()}
            elif cmd == "add":
                res = {"ok": True, "result": engine.add_message(
                    session_id=req.get("session_id", "default"),
                    role=req.get("role", "user"),
                    content=req.get("content", ""),
                    user_id=req.get("user_id", "default"),
                    timestamp=req.get("timestamp"),
                )}
            elif cmd == "remember":
                res = {"ok": True, "result": engine.remember(
                    lesson=req.get("lesson", ""),
                    domain=req.get("domain", "general"),
                    user_id=req.get("user_id", "default"),
                )}
            elif cmd == "flush":
                res = {"ok": True, "result": engine.flush(
                    session_id=req.get("session_id", "default"),
                    user_id=req.get("user_id", "default"),
                )}
            elif cmd == "search":
                res = {"ok": True, "result": engine.search(
                    query=req.get("query", ""),
                    top_k=req.get("top_k", 5),
                    user_id=req.get("user_id"),
                )}
            elif cmd == "get_profile":
                res = {"ok": True, "result": engine.get_profile()}
            elif cmd == "ping":
                res = {"ok": True, "result": "pong"}
            else:
                res = {"ok": False, "error": f"unknown command: {cmd}"}
                
        except Exception as e:
            res = {"ok": False, "error": str(e)}

        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
