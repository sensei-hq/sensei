# 02 — Desktop Observatory

Tauri + SvelteKit desktop app. The developer's window into how their AI-assisted development is going.

---

## What is the Observatory?

The desktop is not a code editor, not a project manager, not a dashboard of vanity metrics. It is a **development session observatory** — a system that watches how you work with AI coding assistants and helps you get better at it.

It answers one question at three levels: **"How am I doing?"**

| Level | Question | Example |
|-------|----------|---------|
| **Global** | How is my AI-assisted development going overall? | "My FTR is 82% this week, up from 70% last week" |
| **Project** | How is this product/system progressing? | "acme-platform had 3 sessions with corrections — all in auth module" |
| **Repo** | How is this specific repo doing? | "acme-api has 2 god nodes causing rework" |

Every insight shown must connect to **quality (FTR), efficiency (turns/tokens), or correctness (pattern compliance)**. If it doesn't serve one of these, it doesn't belong.

---

## Core Insights

### First Try Right (FTR)

The primary metric. What percentage of sessions complete without corrections or rework?

- **High FTR** means sensei is providing the right context, the right rules, and the right guardrails.
- **Low FTR** means something is wrong — missing patterns, stale docs, unclear rules, or the wrong skills are active.

When FTR drops, the observatory surfaces **why**: which sessions failed, what corrections were made, which modules are causing trouble. This is how the developer (and sensei) learn.

### Learning from Mistakes

The observatory's most valuable function is closing the feedback loop:

```
Session happens → Events captured → Metrics computed → Patterns surfaced
     ↓                                                        ↓
FTR drops in auth module                     "3 sessions corrected auth flow"
     ↓                                                        ↓
Coaching: "Auth module has no integration     Action: add auth persona,
test pattern. Sessions keep failing on         create test pattern,
edge cases the AI doesn't know about."        update session-start rules
     ↓
Next session: AI loads auth persona,
follows test pattern → FTR recovers
```

This is not passive observation. The observatory identifies what causes problems and suggests what to change — which mindsets to apply, which patterns to extract, which rules to add.

### Quality Signals

Beyond FTR, the observatory tracks:

| Signal | What it reveals | Action when bad |
|--------|----------------|-----------------|
| **Pattern compliance** | Is new code following established patterns? | Extract missing pattern, add to rules |
| **Test coverage delta** | Are tests being written alongside features? | Adjust skills (enable test-gen) |
| **Doc drift** | Are docs stale relative to code changes? | Flag in session-start, enable drift detection |
| **Complexity hotspots** | Where are the god nodes and tight clusters? | Refactoring targets, warn AI about blast radius |
| **Token efficiency** | Is context being wasted on irrelevant files? | Tune context packs, adjust what session-start loads |

---

## User Actions

The observatory is not view-only. Every insight connects to an action:

| Insight | Action |
|---------|--------|
| FTR dropping in a module | Add a persona for that module, extract patterns from successful sessions |
| God node causing rework | Mark as refactoring target, warn in session-start context |
| Skill not triggering | Check activation conditions, reconfigure for this project's stage |
| Stale docs causing confusion | Queue doc update, enable drift detection skill |
| New team member struggling | Share project profiles, enable coaching skills |
| Pattern extracted from successful session | Promote to project rule, share across repos |

---

## Managing Work

### Projects

A project is one or more repos forming a product. The default is a single repo — most projects start as exactly one repo. When multiple repos are detected as related, they are auto-merged into a single project:

- **Parent folder** — repos under the same directory (e.g., `~/Developer/acme/acme-api` + `~/Developer/acme/acme-ui`)
- **Name prefix** — repos sharing a common stem (`acme-api`, `acme-ui`, `acme-shared`)
- **Monorepo detection** — workspace configs (npm workspaces, Cargo workspace, etc.)

The user can always **split** a repo out of a project or **merge** repos together manually.

Example: `acme-api` + `acme-ui` + `design-system` = project "Acme Platform" (3 repos).

- **Create** from the overview page or setup wizard (auto-detected from name patterns, monorepo workspaces)
- **Configure** roles per repo (backend, frontend, library, docs, infra)
- **Observe** aggregated metrics across all repos in the project
- **Manage** skills, profiles, and rules at the project level — they cascade to repos

### Repos

A single git repo or folder. Sessions always belong to a repo (identified by CWD).

- **Scan** folders to discover repos
- **Index** to extract code graph (functions, types, edges, communities)
- **View** complexity, god nodes, dead code, community structure
- **Override** project-level profiles with repo-specific rules

### Libraries

External documentation indexed for AI context (e.g., Svelte docs, Stripe API reference).

- **Browse** and search indexed library docs
- **Add** new libraries by name + version (remote indexing)
- **Use** via `get_lib_docs` / `search_lib_docs` MCP tools in AI sessions

---

## User Journey

### Day 1 — Getting Started

1. **Open app** → Setup wizard
2. **Configure ACPs** → detect Claude Code, Cursor, etc. and register MCP
3. **Scan folders** → discover git repos
4. **Group repos** → auto-detect projects, confirm groupings, assign roles
5. **Land on overview** → see all repos, first index starts

### Day N — Observing and Improving

1. **Open app** → sidebar shows recent projects and repos
2. **Check project dashboard** → FTR trend, recent sessions, quality signals
3. **Spot a problem** → "FTR dropped in auth module, 3 corrections this week"
4. **Drill in** → sessions page shows which sessions, what went wrong
5. **Take action** → add auth persona, extract pattern, update rules
6. **Verify** → next sessions show improvement, FTR recovers

### Ongoing — Configuration and Management

- **Catalog** → browse marketplace, install/remove skills and commands
- **Settings** → daemon port, ACP configuration, workspace management
- **Libraries** → add external docs the AI should reference
- **Profiles** → adjust mindsets, personas, rules per project/repo

---

## Pages

Each page is documented in its own file with: purpose, what the user sees, insights provided, actions available, and what needs to be built.

| # | Page | Doc | Purpose |
|---|------|-----|---------|
| 1 | Setup Wizard | [01-onboarding.md](./01-onboarding.md) | Day 1: scan, discover, group, configure |
| 2 | Overview | [02-workspace.md](./02-workspace.md) | All repos, scanning, project management |
| 3 | Project Dashboard | [03-observatory.md](./03-observatory.md) | The core: quality metrics, FTR, coaching |
| 4 | Sessions | [04-sessions.md](./04-sessions.md) | Session history, event analysis, learning from mistakes |
| 5 | Codebase | [05-codebase.md](./05-codebase.md) | Architecture, graphs, traceability, repo detail |
| 6 | Coaching | [06-coaching.md](./06-coaching.md) | Profiles, patterns, skills, guided recommendations |
| 7 | Configuration | [07-configuration.md](./07-configuration.md) | Settings, catalog, libraries, ACPs |

### Reference

| Doc | Description |
|-----|-------------|
| [archive/](./archive/) | Previous design explorations (observatory-analysis, ux-redesign, views, gap-analysis) |
