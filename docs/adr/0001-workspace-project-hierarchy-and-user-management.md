# 0001: Workspace, Project, and User Management Hierarchy

## Status
Accepted

## Context
As the Petri agent system expanded to handle multiple codebases, team members, background tasks, and AI worker nodes, a flat directory or ad-hoc session structure was insufficient. Users required a clear hierarchy that organizes workspaces, projects, repositories, threads, and background sub-agent tasks, alongside a multi-user management system with Role-Based Access Control (RBAC).

## Decision
1. **Domain Hierarchy Definition**:
   Formalize the domain model as:
   `Workspace → Project → Repo & Branch / Worktree → Thread → Task / Fork`.
   - **Workspace**: Root collaborative environment (`Primary Mesh`).
   - **Project**: High-level application or system container with metadata, default execution mode and model.
   - **Repo**: Connected Git repository with tracking for branch and worktree path.
   - **Thread**: Conversational turn trajectory with durable message history.
   - **Task / Fork**: Sub-turn execution, background run, or autonomous worker invocation.

2. **Persistent Relational SQLite Layer**:
   Store workspaces, users, projects, project members, and project repos in `mesh.db`. Provide reactive stats (thread counts, member counts, repo counts) via relational queries.

3. **RBAC & User Safety Invariants**:
   - Four discrete roles: `sysmin`, `admin`, `member`, `viewer`.
   - System invariant: Prevent deletion of the last remaining `sysmin` user to avert orphan administrative lockouts.

4. **UI Dashboard Integration**:
   - Implement `/projects` page with obsidian frosted dark glass aesthetic (`#06070a`, `#080a0d`), KPI cards, search and status filters, project activation, and modal project creation.
   - Upgrade `/users` page with user KPI bar, role switcher dropdowns, status toggles, user creation/invitation modal, and delete protections.

## Consequences
- Developers and operators can seamlessly switch project context with one click, automatically updating root paths, default modes, and tree views.
- Full offline-first capability via local SQLite, with graceful fallbacks for web/test runners.
- Zero visual or functional regression against existing test suites.
