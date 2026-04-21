# `sensei remove` — Design Spec

**Date:** 2026-04-21
**Status:** Approved
**Scope:** Replace `sensei uninstall` with granular `sensei remove` supporting per-ACP teardown and data-safe removal.

---

## Problem

`sensei uninstall` is a sledgehammer — it deletes everything including session data, indexes, and project artifacts. During development, testing a clean install/uninstall cycle for a single ACP (e.g., Claude Code) destroys all accumulated session history and graph indexes. Users who want to remove sensei also lose project artifacts they may have invested time in (rules, personas, mindsets).

## Goals

1. Per-ACP removal without touching data or other ACPs
2. Full artifact removal that preserves data by default
3. Explicit `--purge` for complete uninstall
4. Daemon does the heavy lifting — CLI and desktop app are thin clients
5. Clean removal of deprecated `uninstall` command and `/api/uninstall` endpoint

---

## CLI Surface

```
sensei remove acp <target>         # target: claude|cursor|windsurf|zed|kiro|opencode|vscode|all
sensei remove all                  # all ACPs + plugin artifacts, preserves data
sensei remove all --purge          # full nuke: data, indexes, project artifacts
```

`sensei uninstall` is removed (not deprecated — just deleted).

### Behavior Matrix

| Command | ACPs | Plugin/hooks | Cmds/skills/agents | Cache | Data (db/graph/config) | Project .sensei/ |
|---------|------|-------------|-------------------|-------|----------------------|-----------------|
| `remove acp claude` | Claude only | No | No | No | No | No |
| `remove acp all` | All | No | No | No | No | No |
| `remove all` | All | Yes | Yes | Yes | **No** | **No** |
| `remove all --purge` | All | Yes | Yes | Yes | **Yes** | **Yes** |

### Symmetry with `init`

| Remove | Init counterpart |
|--------|-----------------|
| `sensei remove acp claude` | `sensei init --acp claude` |
| `sensei remove acp all` | `sensei init --scope user` |
| `sensei remove all` | `sensei init` |

---

## Architecture

CLI is a thin wrapper. Daemon handles all removal logic via HTTP endpoints. This allows the desktop app to call the same endpoints.

```
CLI (thin client)                     Daemon (does the work)
──────────────────                   ────────────────────────
sensei remove acp claude  ──POST──>  /api/acp/remove  {"acps": ["claude-code"]}
sensei remove acp all     ──POST──>  /api/acp/remove  {"acps": []}
sensei remove all         ──POST──>  /api/remove      {"purge": false}
sensei remove all --purge ──POST──>  /api/remove      {"purge": true}
                                     (then CLI stops daemon + rm ~/.sensei/)
```

---

## Daemon Endpoints

### `POST /api/acp/remove`

**Request:**
```json
{"acps": ["claude-code"]}
```
Empty array = all detected ACPs.

**Handler:** Calls `acp::remove_selected(ids)` — new function that removes only the specified ACP configs. Each ACP adapter's `remove()` method handles its own cleanup (plugin uninstall, MCP remove, JSON cleanup).

**Response:**
```json
{
  "acps_removed": ["claude-code"],
  "errors": []
}
```

### `POST /api/remove`

**Request:**
```json
{"purge": false}
```

**Handler:** Calls `installer::remove(purge)` — refactored from `uninstall()`.

**When `purge: false`:**
1. Remove all ACP configs (`acp::remove_selected(&[])`)
2. Remove plugin directory (`~/.claude/plugins/sensei/`)
3. Remove commands, skills, agents from `~/.claude/{commands,skills,agents}/`
4. Remove hook config from `~/.claude/settings.json` (hooks section)
5. Clear marketplace cache (`~/.sensei/cache/`)
6. Do NOT touch: sensei.db, graph/, config.json, projects.json, project .sensei/ dirs

**When `purge: true`:**
1. All of the above, plus:
2. Remove project .sensei/ dirs (from registered projects list)
3. Flush and close the database
4. Respond to the HTTP request (so CLI gets the result)
5. CLI then: stops daemon, deletes `~/.sensei/` directory

**Response:**
```json
{
  "acps_removed": ["claude-code", "cursor"],
  "plugin_removed": true,
  "commands_removed": 20,
  "skills_removed": 9,
  "agents_removed": 8,
  "hooks_removed": true,
  "cache_cleared": true,
  "projects_cleaned": ["/Users/Jerry/Developer/myapp"],
  "errors": []
}
```

### Remove `/api/uninstall`

Delete the endpoint entirely. Remove `uninstall()`, `uninstall_legacy()`, `UninstallRequest`, `UninstallResult` from `installer.rs`. Replace with the new `remove()` function and `RemoveResult`.

---

## Daemon-Side Refactoring

### `acp.rs` — Add `remove_selected()`

```rust
/// Remove specific ACP configs by ID. Empty slice = remove all.
pub fn remove_selected(ids: &[String]) -> Vec<String> {
    let acps = all_acps();
    let targets: Vec<&Box<dyn Acp>> = if ids.is_empty() {
        acps.iter().collect()
    } else {
        acps.iter().filter(|a| ids.contains(&a.id().to_string())).collect()
    };
    targets.iter().filter_map(|acp| {
        if acp.remove() { Some(acp.id().to_string()) } else { None }
    }).collect()
}
```

Existing `acp::unconfigure()` becomes `acp::remove_selected(&[])`. The trait method `unconfigure()` on each ACP adapter is renamed to `remove()`.

### `installer.rs` — Replace uninstall with remove

**Remove:**
- `UninstallRequest`, `UninstallResult`
- `uninstall()`, `uninstall_legacy()`
- `uninstall_user_scope()`, `uninstall_project_scope()`

**Add:**

```rust
#[derive(Deserialize, Default)]
pub struct RemoveRequest {
    #[serde(default)]
    pub purge: bool,
}

#[derive(Serialize, Default)]
pub struct RemoveResult {
    pub acps_removed: Vec<String>,
    pub plugin_removed: bool,
    pub commands_removed: u32,
    pub skills_removed: u32,
    pub agents_removed: u32,
    pub hooks_removed: bool,
    pub cache_cleared: bool,
    pub projects_cleaned: Vec<String>,
    pub errors: Vec<String>,
}
```

**`remove(req)` function:**

```rust
pub fn remove(req: &RemoveRequest) -> RemoveResult {
    let mut result = RemoveResult::default();

    // 1. Remove all ACP configs
    result.acps_removed = crate::acp::remove_selected(&[]);

    // 2. Remove plugin artifacts
    remove_plugin_artifacts(&mut result);

    // 3. Clear cache
    remove_cache(&mut result);

    // 4. If purge: remove project .sensei/ dirs
    if req.purge {
        remove_registered_projects(&mut result);
        // Note: CLI handles stopping daemon + deleting ~/.sensei/ after receiving response
    }

    result
}
```

Split current `uninstall_user_scope()` into focused helpers:
- `remove_plugin_artifacts()` — plugin dir, commands, skills, agents, hooks config
- `remove_cache()` — marketplace cache
- `remove_registered_projects()` — iterate registered projects, delete .sensei/ in each

### `routes.rs` — Wire new endpoints

- Add `POST /api/acp/remove` → `acp_remove(Json(body))`
- Add `POST /api/remove` → `remove(Json(body))`
- Delete `POST /api/uninstall` route and handler

---

## CLI Changes

### `main.rs` — Replace Uninstall with Remove

**Remove:** `Commands::Uninstall` variant, `uninstall()` function.

**Add:**
```rust
/// Remove sensei configuration
Remove {
    /// What to remove: "acp" or "all"
    target: String,
    /// ACP name (for "acp" target): claude, cursor, windsurf, zed, kiro, opencode, vscode, all
    name: Option<String>,
    /// Also remove data (sessions, indexes, project artifacts)
    #[arg(long)]
    purge: bool,
},
```

**`remove()` function:**

```
sensei remove acp claude  → target="acp", name=Some("claude")
sensei remove acp all     → target="acp", name=Some("all")  
sensei remove all         → target="all", name=None, purge=false
sensei remove all --purge → target="all", name=None, purge=true
```

For `target="acp"`:
- Map friendly name to ACP ID (claude → claude-code, etc.)
- Call `POST /api/acp/remove {"acps": [id]}`
- Print result

For `target="all"`:
- Call `POST /api/remove {"purge": purge}`
- Print result
- If purge: stop daemon, delete `~/.sensei/`

**ACP name mapping:**

| CLI name | ACP ID |
|----------|--------|
| claude | claude-code |
| desktop | claude-desktop |
| cursor | cursor |
| windsurf | windsurf |
| zed | zed |
| kiro | kiro |
| opencode | opencode |
| vscode | vscode |
| all | (empty — unconfigure all) |

---

## Purge Sequence (CLI)

```
1. CLI sends POST /api/remove {"purge": true}
2. Daemon:
   a. Removes all ACP configs
   b. Removes plugin artifacts (commands, skills, agents, hooks)
   c. Clears cache
   d. Removes project .sensei/ dirs from registered projects
   e. Flushes + closes DB
   f. Returns response
3. CLI receives response, prints summary
4. CLI calls `sensei stop` (stops daemon, PID file gone)
5. CLI deletes ~/.sensei/ directory
6. CLI prints "Sensei fully removed."
```

---

## Desktop App Impact (Follow-up Issue)

The desktop app (`apps/desktop`) currently may call the old `/api/uninstall` endpoint. Changes needed:

1. **Settings page** — Add granular removal options matching the new endpoints:
   - Per-ACP toggle (remove/add) via `/api/acp/remove` and `/api/acp/configure`
   - "Remove sensei" button via `/api/remove {"purge": false}`
   - "Full uninstall" with confirmation dialog via `/api/remove {"purge": true}` (app handles daemon stop + data dir cleanup)

2. **Update API client** — Remove `/api/uninstall` calls, replace with new endpoints.

3. **Confirmation UX** — Purge should require explicit confirmation (type project name or similar) since it destroys session history.

**Create a GitHub issue for this after CLI implementation.**

---

## Test Plan

### Unit tests (installer.rs)

- `remove_plugin_artifacts` removes commands/skills/agents/hooks, preserves data
- `remove_cache` clears cache dir
- `remove` with `purge: false` preserves ~/.sensei/sensei.db
- `remove` with `purge: true` removes project .sensei/ dirs
- `remove_registered_projects` only removes .sensei/ from registered paths

### Unit tests (acp.rs)

- `remove_selected` with specific ID removes only that ACP
- `remove_selected` with empty slice removes all
- `remove_selected` with unknown ID returns empty (no error)

### Integration tests (CLI)

- `sensei remove acp claude` + `sensei init --acp claude` round-trip preserves session data
- `sensei remove all` preserves `~/.sensei/sensei.db`
- `sensei remove all --purge` results in empty `~/.sensei/` (after daemon stop)

---

## Files Summary

### Modify
```
crates/sensei-cli/src/main.rs          — Replace Uninstall with Remove command
crates/senseid/src/installer.rs        — Replace uninstall functions with remove functions
crates/senseid/src/acp.rs              — Add remove_selected()
crates/senseid/src/api/routes.rs       — Wire new endpoints, remove /api/uninstall
```

### No new files needed — this is a refactor of existing code.
