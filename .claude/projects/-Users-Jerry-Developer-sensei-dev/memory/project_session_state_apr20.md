---
name: Session state — April 20, 2026
description: Where we are, what's done, what's next. Resume from here.
type: project
---

## What was done (Apr 19-20)

### Issues completed (7 of 8 original open)
- #90 IR pipeline wired (parse_to_ir into indexing)
- #91 Graph node enrichment (docstrings, params, returns)
- #92 GitHub issue templates + catalog (all 28 commands registered)
- #93 Retired 11 absorbed skills
- #95 Extensible mindsets/personas (7 mindsets, 3 personas as individual .md files)
- #96 IMPLEMENTS/EXTENDS edges from IR classes
- #97 Daemon metrics computation (FTR, turn count, rework rate)

### Plugin system
- `sensei init` CLI command — creates .sensei/, .claude/, .mcp.json per repo
- `install-plugin.sh` with gate check (catalog completeness, mindsets, personas)
- Session-start hook injects full mindsets + personas from .sensei/
- CLAUDE.md gate check references .sensei/ paths (works in any repo)
- All 28 commands registered in catalog.json

### Desktop observatory
- **RepoStore** reactive class (`repos.svelte.ts`) — owns all project data, SSE subscription, search, solutions
  - Tested with 7 integration tests against real daemon
  - SSE events update per-repo progress immediately
  - New `RepoQueued` SSE event carries `files_total` for accurate progress bars
- **Shared components** extracted: StatusBadge, FolderInput, RepoListItem, colors.ts
- **Overview/Workspace page** wired to real data (not dummy):
  - Solutions grouped with repos
  - Standalone projects below
  - Unified search/add input bar
  - Progress bar in header
  - Add-to-solution on standalone repos
  - Exclude repos (daemon-side exclusions)
- `/all` retired → redirects to `/overview`
- Dummy data pages exist for: Sessions, Session Detail, Tools, Profiles, Project Dashboard, Project Code, Skills

### Daemon additions
- `POST /api/projects/:id/exclude` — exclusion system with `excluded_paths` table
- `GET /api/exclusions`, `DELETE /api/exclusions/:path`
- `GET /api/metrics/:project` + MCP `get_metrics()` tool
- `compute_metrics()` — FTR, turn count, rework rate, tool adherence
- `RepoQueued` SSE event for accurate progress display
- Scan handler skips excluded paths
- 363 daemon tests passing

### Design docs
- `docs/ideas/24-desktop-observatory.md` — full product vision
- `docs/ideas/24a-observatory-data-audit.md` — data capture gaps + FR tracking
- `docs/ideas/24b-capability-registry.md` — configurable capabilities with workarounds
- `docs/design/02-desktop/observatory-analysis.md` — consolidated system design
- Feature requests submitted: claude-code (#50863, #50926, #50927, #50931), opencode (#23454, #23455), codex (#18600)

## What's next

### Immediate (before building more pages)
1. **Rebuild binaries** — `cargo build --release && bash scripts/link.sh` (RepoQueued event needs new binary)
2. **Restart daemon** — `senseid stop && senseid start`
3. **Verify progress flow** — reset, scan, watch SSE-driven progress with accurate totals

### Next pages to wire (state class → test → shared components → page)
4. **Rename /overview to /workspace** — current page is repo management, not metrics dashboard
5. **Build real /overview** — metrics dashboard (FTR, sessions, rework, tool adherence + actionable insights)
6. **Wire Sessions page** — SessionStore reactive class, real daemon data
7. **Wire Project detail** — real graph data, code intelligence with action recipes

### Architecture decisions documented
- Three-level scope: Global > Solution > Project
- Sessions identified by CWD, belong to projects, aggregated up to solutions
- Mindset-to-agent promotion path (what+why → what+why+how)
- Capability registry with workarounds tied to upstream FRs
- RepoStore pattern: state class → integration test → shared components → page

## Sensei plugin status
- All files in `.sensei/`: mindsets/, personas/, rules.md
- Plugin installed: hooks wired, MCP registered, 28 commands in catalog
- Svelte MCP configured (needs session restart to activate)
- Gate check passes: `bash scripts/install-plugin.sh`
