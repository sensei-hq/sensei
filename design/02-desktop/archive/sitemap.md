# Desktop App — Site Layout

> SvelteKit 2 + Tauri 2 desktop app. Static adapter (SPA mode).
> All routes under `src/routes/(app)/` share the sidebar layout.

---

## Sidebar

```
+---------------------------+
| [logo] sensei             |
|                           |
| RECENT                    |
|   [S] Solution Name    3  |   <- /s/{id}
|   [P] Project Name        |   <- /p/{id}
|   [S] Another Solution 5  |
|   +2 more                 |
|                           |
| ─────────────────         |
| GLOBAL                    |
|   Overview                |   <- /overview
|   Libraries               |   <- /libraries
|   Tools                   |   <- /tools
|   Catalog                 |   <- /catalog
|   Sessions                |   <- /sessions
|                           |
| ─────────────────         |
|   [status] localhost:7744 |
|   Settings                |   <- /settings
+---------------------------+
```

**Changes from current:** rename "Skills & Plugins" to "Catalog", add Sessions to global nav, drop Benchmarks from nav (move to settings or solution context).

---

## Routes

### `/setup` — Onboarding Wizard

First-run experience. Six steps, progress dots.

| Step | Content |
|------|---------|
| 1. Welcome | Intro, confirm to proceed |
| 2. ACPs | Detect AI coding platforms, configure MCP |
| 3. Folders | Pick directories to scan |
| 4. Repos | Live discovery polling |
| 5. Groups | Variant detection, duplicate merging, solution creation |
| 6. Done | Redirect to active solution or overview |

**Status:** Implemented.

---

### `/overview` — All Projects

Default landing page. Shows every indexed project, grouped by solution membership.

| Section | Content |
|---------|---------|
| Search bar | Filter projects by name |
| Scan button | Trigger folder scan |
| Solution groups | Projects grouped under their solution |
| Standalone projects | Projects not in any solution |
| Project card | Name, path, language breakdown, indexed-at, function/type counts |

**Status:** Implemented.

---

### `/p/{id}` — Project Detail

Single project view (standalone, not inside a solution context).

| Section | Content |
|---------|---------|
| Header | Project name, path, last indexed |
| Summary stats | Functions, types, files, complexity |
| Graph preview | Small force-directed graph of the project |
| Recent sessions | Sessions that touched this project |
| Complexity hotspots | Top functions by cyclomatic complexity |

**Status:** Implemented.

---

### `/libraries` — Library Documentation

Browse and manage indexed library docs.

| Section | Content |
|---------|---------|
| Search | Filter by library name |
| Library list | Name, version, section count, indexed status |
| Detail panel | Sections, content preview |
| Add library | Remote index modal (name + version) |

**Status:** Implemented.

---

### `/tools` — MCP Tools Explorer

Browse and test MCP tools exposed by sensei-mcp.

| Section | Content |
|---------|---------|
| Tool list | Name, description, input schema |
| Simulator | Select a tool, fill parameters, invoke, see result |

**Status:** Implemented.

---

### `/catalog` — Skills, Plugins & Commands

Marketplace browser. Install/remove skills, commands, and view available plugins.

| Section | Content |
|---------|---------|
| Tabs | Skills / Plugins / Commands / Hooks |
| Catalog items | Name, description, installed status |
| Install/remove actions | Per-item toggle |
| Installed badge | Shows what's active |

**Status:** Implemented. Currently at `/catalog` route.

---

### `/sessions` — Session History

Global session list across all projects.

| Section | Content |
|---------|---------|
| Session list | Paginated, sorted by recency |
| Per session | Project name, start/end time, duration, event count, ACP used |
| Filter | By project, by date range, by ACP |

**Status:** Stub (dummy data). Needs real API wiring.

---

### `/sessions/{id}` — Session Detail

Single session timeline.

| Section | Content |
|---------|---------|
| Header | Session ID, project, ACP, duration |
| Event timeline | Chronological list of tool calls, file edits, commands |
| Stats | Files touched, tools invoked, total cost/tokens |

**Status:** Stub (dummy data). Needs real API wiring.

---

### `/settings` — App Settings

| Section | Content |
|---------|---------|
| Display | Theme, sidebar max items |
| Daemon | Port, connection status, restart |
| ACPs | Detected platforms, configure/remove per ACP |
| Workspace | Scanned folders, re-scan |
| About | Version, links |

**Status:** Implemented.

---

## Solution Routes — `/s/{id}/`

Tabbed layout. Tabs: **Overview**, **Architecture**, **Sessions**, **Repos**, **Skills**.

### `/s/{id}` — Solution Overview

Dashboard for a multi-repo solution.

| Section | Content |
|---------|---------|
| Stats row | Total repos, functions, types, avg complexity |
| Cross-repo analysis | Shared libraries, dependency overlap |
| Graph preview | Combined graph across repos |
| Recent sessions | Sessions touching any repo in the solution |

**Status:** Implemented.

---

### `/s/{id}/arch` — Architecture

Code graph visualization with multiple views.

| View | Content |
|------|---------|
| Structural | Force-directed graph, communities, high-complexity nodes |
| Doc drift | Functions where docs are stale vs implementation |

**Status:** Implemented (structural + doc drift). Deployment view planned.

---

### `/s/{id}/sessions` — Solution Sessions

Sessions scoped to repos in this solution.

| Section | Content |
|---------|---------|
| Session list | Filtered to this solution's repos |
| Same columns as global sessions | |

**Status:** Stub. Shares implementation with `/sessions` once wired.

---

### `/s/{id}/repos` — Repo Management

Add/remove repos from this solution.

| Section | Content |
|---------|---------|
| Current repos | List with role badges (backend, frontend, etc.) |
| Available repos | Indexed repos not yet in this solution |
| Add/remove actions | |

**Status:** Partially implemented.

---

### `/s/{id}/skills` — Solution Skills

Skills scoped to this solution (project-level overrides).

| Section | Content |
|---------|---------|
| Installed skills | Active for this solution |
| Available skills | From catalog, not yet enabled |
| Enable/disable toggle | Per-skill |

**Status:** Stub. Needs catalog API wiring.

---

### `/s/{id}/p/{pid}` — Project Detail (in solution context)

Same as `/p/{id}` but with solution breadcrumbs and back-navigation to the solution.

**Status:** Implemented.

---

## Routes to Remove

These are redirects or dead ends that should be deleted:

| Route | Reason |
|-------|--------|
| `/home` | Redirects to `/all` which redirects to `/overview` |
| `/all` | Redirects to `/overview` |
| `/projects` | Redirects to `/overview` |
| `/ideas` | Redirects to `/all` |
| `/graph` | Redirects to `/all` |
| `/profiles` | Uses dummy data, no backend, unclear purpose |
| `/acp` | Functionality covered by `/settings` ACPs section |
| `/benchmarks` | No backend, move to future milestone |
| `/indexer` | Functionality better placed in `/settings` or solution context |
| `/s/{id}/trace` | Duplicate of `/s/{id}/arch` |
| `/s/{id}/sources` | Same content as `/s/{id}` overview |
| `/s/{id}/indexer` | Move to settings or inline in solution overview |
| `/s/{id}/p/{pid}/code` | Stub, merge into `/s/{id}/p/{pid}` when ready |

---

## Data Flow

```
Tauri (native shell)
  └── SvelteKit SPA (static adapter)
        └── senseiApi(port) → HTTP → senseid daemon (localhost:7744)
              ├── /api/projects     → project CRUD + scanning
              ├── /api/solutions    → solution CRUD
              ├── /api/graph/       → code graph queries
              ├── /api/index/       → indexing + SSE progress
              ├── /api/sessions     → session history
              ├── /api/libs/        → library docs
              ├── /api/mcp/         → tool invocation
              ├── /api/acp/         → ACP detection + config
              ├── /api/install/     → marketplace catalog
              └── /api/config       → user preferences
```
