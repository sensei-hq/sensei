# 07 — Configuration

> Routes: `/settings`, `/catalog`, `/libraries`

## Purpose

Manage the infrastructure that makes the observatory work: daemon connection, ACP registration, marketplace items, and external library docs. These pages are visited occasionally, not daily.

## Settings (`/settings`)

### Sections

**Display**
- Sidebar max items (how many recent projects/repos to show)

**Daemon**
- Port (default 7744)
- Connection status indicator
- Restart daemon button

**ACPs**
- Detected AI coding platforms with status badges
- Configure / Remove per ACP
- For Claude Code: shows plugin install status (marketplace, commands, skills, hooks, MCP)
- For others: shows MCP config file path and registration status

**Workspace**
- Scanned root folders
- Add / remove scan roots
- Re-scan button (discover new repos in existing roots)

**About**
- Version
- Links to docs, GitHub

### What's Built

Settings page is fully implemented with all sections.

### What Needs Work

- **ACP section** should show richer plugin status for Claude Code (version, marketplace source, installed commands/skills count)
- **Workspace section** could show repo count per scan root

## Catalog (`/catalog`)

### Purpose

Browse and manage marketplace items: skills, plugins, commands, hooks.

This is the **global** marketplace browser — for installing/removing items that ship with the sensei plugin or are available from the marketplace. Different from the project-level skill configuration in the Coaching page, which controls which skills are active per project.

### What the Developer Sees

Tabs: **Skills** / **Plugins** / **Commands** / **Hooks**

Each tab shows a list of items with:
- Name and description
- Installed status (badge)
- Install / Remove action
- Source (marketplace / custom)

### What's Built

Catalog page is implemented with tabs and install/remove actions. Reads from daemon `/api/install/catalog` and `/api/install/installed` endpoints.

### What Needs Work

- **Skill detail** — clicking a skill should show its full content (description, trigger conditions, what it does)
- **Relationship to project skills** — catalog shows what's available globally; the coaching page shows what's active per project. This relationship should be clearer.

## Libraries (`/libraries`)

### Purpose

Browse and manage **external library documentation** indexed for AI context. When a developer is using Svelte, Stripe, or any external library, sensei can index that library's docs so the AI has accurate reference material instead of relying on training data.

This is NOT about project dependencies. It's about making the AI smarter about the tools the developer uses.

### What the Developer Sees

- Search bar to filter by library name
- Library list: name, version, section count, indexed status
- Detail panel: sections, content preview
- "Add library" button: opens modal to specify name + version for remote indexing

### What's Built

Libraries page is fully implemented. Remote indexing modal works. Reads from daemon `/api/libs/` endpoints.

### What Needs Work

- **Usage tracking** — show which libraries are actually referenced in AI sessions (via `get_lib_docs` / `search_lib_docs` tool usage)
- **Auto-suggest** — detect project dependencies and suggest relevant library docs to index
- **Version freshness** — flag when indexed version is behind the version in the project's dependencies

---

## Navigation

These three pages sit in the sidebar's Global section:

```
GLOBAL
  Overview        ← workspace (02-workspace.md)
  Libraries       ← this page
  Tools           ← MCP tools explorer (standalone, no doc needed — shows tool list + simulator)
  Catalog         ← this page
  Sessions        ← session history (04-sessions.md)
```

Settings is at the bottom of the sidebar, separate from the global nav.
