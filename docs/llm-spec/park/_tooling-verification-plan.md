# Sensei tooling verification — steps (2026-07-12)

**Mandate (Jerry):** verify the sensei MCP/context loop works end-to-end with GENUINE DB-backed results
BEFORE resuming the original build plan. sensei must make the assistant efficient (folder → project →
search/context/rules/memories). Dogfood it. **Always `make install` (release) after `make bump`.**

## Findings (dogfooded against the live daemon)
- Daemon was **v0.2.29**, repo **v0.2.39** — 10 releases stale (bump never installed). Now release v0.2.39 installed + live. ✅
- Data IS indexed: **558,904 nodes**; the sensei repo folders map to project **`ff1ccea2…` ("sensei", 3395 fns/664 types)**. Scanner picked the right context. ✅
- Tools returning GENUINE results (with explicit `project:"sensei"`): `get_project_summary`, `search`, `list_projects`. ✅
- BUGS found:
  - **B1 — stale MCP proxy:** running `sensei-mcp` processes still run the OLD binary (0.2.29); the new binary is on disk but the process wasn't reloaded → some tools 404/behave old. **Needs an MCP-server reload.**
  - **B2 — get_rules:** daemon `GET /api/knowledge/rules` needs `?folder=<abs_path>` (returns real rules with it; 400 without). The MCP tool doesn't pass the folder.
  - **B3 — get_layered_context:** daemon `GET /api/knowledge/context` needs `project_id=<uuid>` (returns real memories); `?project=<name>` → 400. The tool passes a name, no name→id resolution.
  - **B4 — cwd project resolution:** the MCP resolves project from the MCP-server PROCESS cwd (`main.rs:30`), which isn't the repo → "no project resolved." Fragile by design.
  - **B5 — duplicate project:** two "sensei" projects — `2efd4ecf` (0 nodes, what list_projects surfaces) vs `ff1ccea2` (the real index). Dedup needed.

## Steps
1. **Reload the MCP server** so the harness runs the new 0.2.39 `sensei-mcp` binary (fixes B1). [Harness-level — Jerry reloads the sensei MCP / restarts Claude Code, OR kill the stale PIDs to force respawn.]
2. **Fix the MCP proxy param-passing** (in `crates/mcp`): B2 pass `folder` (the resolved project's abs_path or cwd) to `get_rules`; B3 resolve project name→`project_id` and pass `project_id` to `get_layered_context`. Rebuild + `make install-service` + reload.
3. **Folder→project resolution + the workflow Jerry wants (B4):**
   - `find_projects` (or `list_projects` + `under=<path>` filter): list projects whose `abs_path` is under a given folder (default cwd) → "find projects under current folder".
   - `use_project`/pin: a way to set the active project for the session (env var `SENSEI_PROJECT` / a pin file the MCP reads, or per-call `project`), so all tools resolve to it regardless of cwd → "I'm working on sensei".
4. **Dedup the empty `sensei` project** `2efd4ecf` (B5) — merge/prune to the real `ff1ccea2`.
5. **Re-verify EVERY tool** returns genuine DB-backed results: get_rules, get_project_summary, search, get_layered_context, get_patterns, get_project_conventions, get_duplicates, get_commands. Record pass/fail.
6. **Workflow fix (process):** `make ship` = bump + install; autonomous loop installs after every milestone bump; consider a version-change worker (rescan/reanalyze on binary version change).
7. **Full-cycle tests** (real use cases): folder → project resolution → search/context/rules → assert DB-backed non-empty genuine results.
8. **THEN resume the original plan** (sweep queue A–G + Dōjō console/supabase + protocol consolidation).

## Status
- [x] Daemon current (release v0.2.39 installed + live)
- [ ] Step 1 reload MCP · [ ] Step 2 param fixes · [ ] Step 3 resolution+tools · [ ] Step 4 dedup · [ ] Step 5 re-verify all · [ ] Step 6 ship/worker · [ ] Step 7 tests · [ ] Step 8 resume plan
