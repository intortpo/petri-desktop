# Petri & EverOS Domain Context

Petri is a desktop agent platform providing durable threads, execution modes, tool fencing, and local-first memory over Antigravity CLI (`agy`) sessions. EverOS provides the local-first, Markdown-native memory operating system layer.

## Language

**EverOS**:
A local-first memory operating system that treats Markdown files as the canonical source of truth while indexing them in SQLite and LanceDB for fast hybrid retrieval.
_Avoid_: Memory vector black-box, cloud-only RAG

**Episode**:
A discrete conversational interaction or session between user and agent, preserved as readable Markdown in `~/.mesh/everos/episodes/`.
_Avoid_: Chat history blob, transcript dump

**Profile**:
Durable user preferences, identity traits, and environmental facts extracted from episodes and maintained in `~/.mesh/everos/profile/`.
_Avoid_: User table, settings row

**Case**:
A historical record of an agent's task-solving trajectory, reasoning steps, tool usages, and outcomes.
_Avoid_: Run log, audit trace

**Skill**:
Reusable procedural engineering knowledge distilled from agent trajectories into standalone `SKILL.md` documents.
_Avoid_: System prompt patch, ad-hoc instructions

**Memory Flush**:
An explicit or turn-boundary synchronization operation that extracts facts and skills from buffered messages, writing them to Markdown and re-indexing.
_Avoid_: Cache write, DB commit

**Sidecar**:
A managed background Python subprocess running alongside the desktop app to execute EverOS extraction, Markdown indexing, and local semantic search.
_Avoid_: External microservice, remote daemon

**Workspace**:
The root environment and team boundary (`Primary Mesh`). Contains projects, team users, and global cluster state.
_Avoid_: Folder, group

**Project**:
A high-level software application, repository grouping, or system container (e.g. `Petri Desktop`). Bound to a filesystem root, linked repositories, assigned team members, and default mode/model configurations.
_Avoid_: Workspace folder, repo mirror

**Hierarchy**:
The structured ownership chain: `Workspace → Project → Repo & Branch / Worktree → Thread → Task / Fork`.
_Avoid_: Flat list, loose directories

**User & Role**:
Identities operating in the mesh governed by Role-Based Access Control (`sysmin`, `admin`, `member`, `viewer`) and status (`active`, `invited`, `suspended`).
_Avoid_: Anonymous caller, global admin toggle
