# Desktop UX Redesign — Solution-Centric Navigation

## Problem

The current desktop has 7 pages that all show variations of the same repo list (Projects, Indexer, Graph, Libraries, Ideas, Sessions, ACP). Users jump between pages to see different facets of the same repos. There's no higher-level concept — everything is flat and repetitive.

## Key User Insights

1. **Users don't work on 10+ repos at once.** They focus on 3-5 solutions at a time. Everything else is a side project or an idea they haven't followed through on.
2. **Repos don't exist in isolation.** Multiple repos form a product/solution. Monorepos contain multiple components. Libraries are shared across solutions.
3. **Non-technical users need to make sense of the system too.** A deployment diagram or traceability view should be readable without understanding the code.
4. **The ACP needs the big picture.** When editing one repo, the AI should know how it fits into the solution, what other repos it connects to, and what docs/requirements exist.

## Core Model

```
Workspace
├── Active Solutions (3-5 the user is currently working on)
│   ├── Solution: Acme Platform
│   │   ├── acme-api (backend)
│   │   ├── acme-ui (frontend)
│   │   └── acme-shared (library) ← also used by other solutions
│   └── Solution: Internal Tools
│       ├── admin-dashboard (frontend)
│       └── admin-api (backend)
├── Libraries (standalone, referenced by multiple solutions)
│   └── design-system (used by Acme Platform + Internal Tools)
├── Side Projects (parked, visit occasionally)
│   └── blog-engine
└── Ideas (considered but not started)
    └── ml-pipeline-experiment
```

### What is a Solution?

A solution is a **group of repos that collectively deliver a product or system**. It can be:

- A **monorepo** — auto-detected from workspace config (package.json workspaces, Cargo.toml workspace, settings.gradle). Components become virtual repos within the solution.
- A **multi-repo product** — user groups repos together and assigns roles.
- A **single repo** — stands alone as its own solution.

### What is a Library?

A library that is referenced by **2+ solutions** is automatically promoted to a standalone entity. It appears in the sidebar as a first-class item, not buried inside a solution. This signals: "this is a product in its own right — changes here affect multiple solutions."

A library referenced by only one solution stays inside that solution.

### Repo Lifecycle

When repos are first imported (scan folder), they land in an **unassigned** state. The system then:

1. **Auto-detects solutions** — monorepos, name-prefix groups (`acme-*`), same GitHub org
2. **Suggests groupings** — "These 3 repos look related. Create a solution?"
3. **User confirms/adjusts** — tags roles (backend/frontend/docs/infra), names the solution, optionally assigns a client
4. **Remaining repos** become side projects or ideas based on activity (last commit age, commit frequency)

## Navigation

```
Sidebar
├── ★ Active                              ← section header
│   ├── Acme Platform                     ← solution (click to expand)
│   │   ├── Overview
│   │   ├── Repos
│   │   ├── Architecture
│   │   ├── Traceability
│   │   └── Sessions
│   └── Internal Tools
├── 📚 Libraries                          ← standalone libs shared across solutions
│   └── design-system
├── 💤 Side Projects                      ← parked, visited occasionally
│   └── blog-engine
├── 💡 Ideas                              ← not started yet
│   └── ml-pipeline
├── ──────────────
├── All Repos                             ← flat list, import, assign
├── ACP Registry
├── Settings
└── [Server status]
```

**The sidebar is the workspace.** Solutions are expandable — clicking one reveals its sub-pages. The active/side/idea categorization is based on:

| Category | Signal |
|----------|--------|
| Active | Opened in ACP in last 7 days, or user pinned |
| Side Project | Has commits but not opened recently |
| Idea | < 5 commits, no recent activity, or user-tagged |

Users can drag solutions between categories or right-click to recategorize.

## Views

### Overview (Solution Dashboard)

One screen that replaces Projects + Indexer + Graph + Sessions for a given solution.

```
┌─────────────────────────────────────────────────────────────┐
│  Acme Platform                                    [⚙ Edit] │
│  3 repos · 847 symbols · 12 docs · last indexed 2m ago     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Connection Diagram                                         │
│  ┌─────────┐   REST    ┌──────────┐                        │
│  │ acme-ui │─────────▶│ acme-api │──────▶ [PostgreSQL]     │
│  │ React   │           │ Express  │                         │
│  └────┬────┘           └────┬─────┘                         │
│       │                     │                               │
│       └─────────┬───────────┘                               │
│                 ▼                                            │
│           ┌──────────┐                                      │
│           │  design  │ ← also used by Internal Tools        │
│           │  system  │                                      │
│           └──────────┘                                      │
│                                                             │
│  ┌──────────────┬──────────────┬──────────────┐            │
│  │ Index Health │ Doc Coverage │ Recent Work  │            │
│  │ ████████░ 85%│ 8/12 traced │ Fix auth  ✓  │            │
│  │ 2 partial    │ 4 gaps      │ Add cache …  │            │
│  └──────────────┴──────────────┴──────────────┘            │
│                                                             │
│  Drift: 2 docs may be stale  ·  1 library has updates      │
└─────────────────────────────────────────────────────────────┘
```

**Developer Impact panel** (bottom of Overview):

```
┌─ Developer Impact ─────────────────────────────────────────┐
│                                                            │
│  This week              vs last week                       │
│  ┌──────────┬──────────┬──────────┬──────────┐            │
│  │Sessions  │ FTR      │ Tokens   │ Cost     │            │
│  │ 24       │ 87% ▲    │ 2.1M     │ $4.20    │            │
│  │ +6       │ was 72%  │ -400k ▼  │ -$1.80 ▼ │            │
│  └──────────┴──────────┴──────────┴──────────┘            │
│                                                            │
│  Sensei Efficiency                                         │
│  ┌────────────────────────────────────────────────────┐   │
│  │ Context packs served:  142  (avg 3.2k tokens each) │   │
│  │ Tokens saved by packs: ~680k  vs loading full files│   │
│  │ Pattern reuses:        18   (4 unique patterns)    │   │
│  │ Drift alerts caught:    3   before they hit PR     │   │
│  │ Skills triggered:      87   (top: session-mgmt 24) │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  FTR Trend (last 30 sessions)                              │
│  100%│         ·  · ·                                      │
│   75%│    · ·  ·    · · ·  · ·                             │
│   50%│  ·                     ·  · ·                       │
│   25%│·                             ·                      │
│     0└─────────────────────────────── sessions →           │
│                                                            │
│  Quality Signals                                           │
│  ┌────────────────────────────────────────────────────┐   │
│  │ ✓ Zero-errors policy: 22/24 sessions passed first  │   │
│  │ ✓ Test coverage: +3.2% this week (now 68%)         │   │
│  │ ⚠ Doc coverage: 67% (3 gaps since last week)       │   │
│  │ ✓ Pattern compliance: 92% of new code follows      │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  [View full analytics]  [Export report]                    │
└────────────────────────────────────────────────────────────┘
```

**What's tracked** (from hooks, MCP tools, and OTLP events):

| Metric | Source | What it shows |
|--------|--------|--------------|
| **FTR (First Try Right)** | `checkpoint` tool outcome | % of sessions completed without blocked/retry — quality of AI output |
| **Tokens used** | OTLP `api_requests` events | Total in/out tokens, cost — efficiency trend |
| **Tokens saved by context packs** | `context_pack` tool calls vs raw file sizes | How much sensei reduces token waste |
| **Pattern reuses** | `record_pattern_use` tool | Team convention adherence |
| **Drift alerts** | `detecting-doc-drift` skill triggers | Docs caught stale before they cause confusion |
| **Skills triggered** | Session-start hook + skill activation | Which skills are actually useful |
| **Zero-errors compliance** | `/commit` command results | Build discipline |
| **Test coverage delta** | Diff of test file count over time | Quality trajectory |

**Aggregation levels:**
- **Solution-level**: shown on Solution Overview (aggregates all repos)
- **Repo-level**: shown in expanded repo card (that repo only)
- **User-level**: shown in global Settings → Analytics (across all solutions)
- **Time ranges**: this week, last 7d, last 30d, all-time (selectable)

This is the proof that sensei improves development quality — visible to the developer, their lead, and anyone reviewing the tooling investment.

**Connection diagram derivation:**
- Monorepo: workspace packages + internal imports
- Multi-repo: cross-repo dependencies (shared lib imports, API client packages, proto/schema files)
- Infra files: Dockerfile → service, docker-compose.yml → topology, k8s → deployment targets
- User hints: role tags (backend/frontend) + manual connection overrides

**Shared libraries** are shown with a note: "also used by X" — making cross-solution impact visible.

### Repos

Single expandable list. Each repo card shows: avatar/logo, name, role badge, stack, mini-graph sparkline, symbol count, index status, doc count.

```
┌─ 🔷 acme-api ─────── backend ─── Express ────────────────┐
│  342 fns · 8 docs · indexed 2m ago     [mini-graph: ···⬡·]│
│  ████████████████░░░░ 80% coverage                        │
└───────────────────────────────────────────────────────────┘
```

**Mini-graph sparkline**: A tiny force-directed cluster visualization (~60x30px) showing the community structure at a glance. Dense clusters = tightly coupled code. Isolated dots = potentially dead code. Red dots = god nodes. This gives an instant visual fingerprint of the repo's shape without clicking in.

Expanding a card reveals inline tabs:

- **Symbols** — searchable function/type list, god nodes, communities
- **Docs** — classified doc list (requirement/design/api/changelog)
- **Index** — live activity log (see Indexer Transparency below)
- **Graph** — interactive community visualization (see Interactive Graphs below)

No separate pages for indexer, graph, or libraries. It's all here.

### Indexer Transparency

The indexer must not be a black box. Every stage of indexing is visible and debuggable.

**Live activity log** (shown in the Index tab of an expanded repo card):

```
┌─ Index: acme-api ──────────────────────────────────────────┐
│                                                            │
│  Status: Indexing (Pass 2 of 4)          [Re-index] [Stop] │
│  ████████████████░░░░░░░░ 342/847 files   41% · 12s       │
│                                                            │
│  Current: src/routes/auth/login.ts                         │
│                                                            │
│  Activity Log                              [Filter ▼]      │
│  ──────────────────────────────────────────────────────     │
│  20:15:03  Pass 1: Scanning files                          │
│  20:15:03  Found 847 files (312 unchanged, skipped)        │
│  20:15:04  Pass 2: Indexing code files                     │
│  20:15:04  ✓ src/index.ts — 8 functions, 3 types           │
│  20:15:04  ✓ src/db/schema.ts — 2 functions, 5 types       │
│  20:15:05  ⚠ src/legacy/old.js — 1 error (see below)      │
│  20:15:05  ✓ src/routes/auth/login.ts — 4 functions        │
│  ...                                                       │
│                                                            │
│  Errors (1)                                [Collapse ▲]    │
│  ┌────────────────────────────────────────────────────┐    │
│  │ src/legacy/old.js                                  │    │
│  │ SyntaxError: Unexpected token at line 42           │    │
│  │ This file was skipped. It will be retried on next  │    │
│  │ index or when the file changes.                    │    │
│  └────────────────────────────────────────────────────┘    │
│                                                            │
│  Previous Run: 2m ago · 847 files · 342 fns · 0 errors    │
│  Manifest: 535 files tracked · 312 unchanged this run      │
└────────────────────────────────────────────────────────────┘
```

**What the user sees at each stage:**

| Pass | What's happening | User sees |
|------|-----------------|-----------|
| Scan | fast-glob discovering files | "Scanning... found N files (M unchanged)" |
| Pass 1 | Code files: parse symbols, write to Kuzu | Per-file: filename, symbols found, or error |
| Pass 2 | IMPORTS edges: resolving cross-file imports | "Resolving imports... N edges" |
| Pass 3 | CALLS edges: linking function calls | "Linking call graph... N edges" |
| Pass 4 | Doc files: indexing .md/.mdx | Per-file: filename, edges created |
| Done | Write traceability, clear progress | Summary: files, functions, types, edges, errors, duration |

**Error drill-down:** Each error is expandable with the full error message, the file path (clickable to open in editor), and what happens next (skipped, will retry). Errors are persisted in `index-errors.json` and visible on next visit.

**Solution-level index health** (on the Overview page):

```
Index Health
├── acme-api:    ████████████████████ 100%  · 0 errors · 2m ago
├── acme-ui:     ██████████████████░░  90%  · 3 errors · 5m ago
└── acme-shared: ████████████████████ 100%  · 0 errors · 5m ago

Overall: 96% indexed · 3 errors across solution
[Re-index all]  [View errors]
```

### Interactive Graphs

Graphs are not static images. They are force-directed, zoomable, clickable visualizations.

**Repo-level graph** (in the Graph tab of an expanded repo card):

```
┌─ Graph: acme-api ─────────────────────────────────────────┐
│                                                           │
│  View: [Communities ▼]   Color: [By community ▼]          │
│  Show: [☑ Functions] [☑ Types] [☐ Files] [☑ God nodes]   │
│                                                           │
│         ┌───────────────────────────────────┐             │
│         │                                   │             │
│         │    ◉ AuthController               │             │
│         │   ╱│╲                              │             │
│         │  ○ ○ ○  auth community             │             │
│         │                                   │             │
│         │         ◉ PaymentService ← RED    │             │
│         │        ╱│╲╲                        │             │
│         │       ○ ○ ○ ○  payment community   │             │
│         │                                   │             │
│         │    ○ ○  utils (loose)             │             │
│         │                                   │             │
│         └───────────────────────────────────┘             │
│                                                           │
│  Selected: PaymentService                    [God Node ⚠] │
│  File: src/services/payment.ts:42                         │
│  Degree: 23 (callers: 15, callees: 8)                     │
│  Sig: class PaymentService { ... }                        │
│  Why it's a hotspot: High coupling — 15 callers across    │
│  4 communities. Changes here have wide blast radius.      │
│                                                           │
│  Callers: login.ts, checkout.ts, refund.ts, ...  [See all]│
│  Callees: stripe.ts, db.ts, logger.ts, ...       [See all]│
└───────────────────────────────────────────────────────────┘
```

**Interactions:**

| Action | Result |
|--------|--------|
| Hover node | Tooltip: name, kind, file:line, degree |
| Click node | Selection panel: signature, docstring, callers/callees list |
| Double-click node | Open file in default editor (via Tauri) |
| Scroll/pinch | Zoom in/out |
| Drag canvas | Pan |
| Drag node | Reposition (physics pauses for that node) |
| Click community cluster | Zoom into that community, dim others |
| Right-click node | Context menu: "Find callers", "Find callees", "Open in ACP", "Copy path" |

**Visual pain point indicators:**

| Indicator | What it means | Visual |
|-----------|--------------|--------|
| **God node** (red, large) | High degree centrality — many callers/callees | Red circle, size ∝ degree |
| **Orphan** (grey, small) | No callers, no callees — possibly dead code | Grey dot, smallest |
| **Bridge** (orange outline) | Connects two otherwise separate communities | Orange border |
| **Tight cluster** (dense blob) | High internal coupling — hard to change one without the other | Overlapping nodes |
| **Long chain** (linear path) | Deep call chain — fragile, hard to trace | Nodes in a line |

**Solution-level graph** (on the Architecture → Structural page):

Shows repos as large nodes, cross-repo edges as connections. Click a repo node to drill into its internal graph. This is the "zoom" metaphor: solution → repo → community → symbol.

### Solution Identity

Each solution gets a visual identity:

**Avatar/Logo**: Auto-generated or user-uploaded.
- Auto-generated: first letter of solution name in a colored circle (color derived from name hash for consistency)
- User can upload a logo/icon (stored in `~/.sensei/solutions/{id}/avatar.png`)
- For repos: use the GitHub avatar if remote URL is GitHub, otherwise first-letter circle

**Mini-graph**: The 60x30px sparkline shown on repo cards and in the sidebar next to solution names. Generated from the community structure:
- Each community = a cluster of dots
- Dot size = symbol count in that community
- Red dots = god nodes
- Updated on each index run, cached as SVG

```
Sidebar with identity:
├── ★ Active
│   ├── 🔷 Acme Platform    [···⬡··⬡·]     ← logo + mini-graph
│   └── 🟢 Internal Tools   [··⬡····]
├── 📚 Libraries
│   └── 🎨 design-system    [·⬡·]
```

### Configuration (per solution)

Each solution has a config panel accessible via the ⚙ icon on the Overview page.

```
┌─ Configuration: Acme Platform ─────────────────────────────┐
│                                                            │
│  General                                                   │
│  ┌────────────────────────────────────────────────────┐   │
│  │ Name:        [Acme Platform          ]              │   │
│  │ Client:      [Acme Corp              ] (optional)   │   │
│  │ Category:    [Active ▼]                             │   │
│  │ Avatar:      [🔷] [Upload]                          │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  Watch Folders                                             │
│  ┌────────────────────────────────────────────────────┐   │
│  │ /Users/dev/projects/acme-api        [Remove]       │   │
│  │ /Users/dev/projects/acme-ui         [Remove]       │   │
│  │ /Users/dev/projects/design-system   [Remove]       │   │
│  │ [+ Add folder]  [Scan parent folder]               │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  Index Settings                                            │
│  ┌────────────────────────────────────────────────────┐   │
│  │ Include patterns:  **/*.ts, **/*.tsx, **/*.py, ...  │   │
│  │ Exclude patterns:  **/node_modules/**, **/dist/**   │   │
│  │ Auto-reindex on file change:  [☑ enabled]          │   │
│  │ Watch debounce:     [300] ms                        │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  ACP Configuration                                         │
│  ┌────────────────────────────────────────────────────┐   │
│  │ Claude Code    [✓ MCP configured]  [Reconfigure]   │   │
│  │ Cursor         [✗ Not configured]  [Configure]     │   │
│  │                                                     │   │
│  │ Skills installed: 4 (orientation, workflow,         │   │
│  │                       context, patterns)            │   │
│  │ [Regenerate skills]  [Install to all ACPs]          │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  Repo Roles                                                │
│  ┌────────────────────────────────────────────────────┐   │
│  │ acme-api        [backend  ▼]   [label: API       ]│   │
│  │ acme-ui         [frontend ▼]   [label: Dashboard  ]│   │
│  │ design-system   [library  ▼]   [label:            ]│   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  Connections (manual overrides)                            │
│  ┌────────────────────────────────────────────────────┐   │
│  │ acme-ui → acme-api   via: REST API    [auto] [✕]  │   │
│  │ acme-api → postgres  via: database    [manual][✕] │   │
│  │ [+ Add connection]                                 │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  Danger Zone                                               │
│  ┌────────────────────────────────────────────────────┐   │
│  │ [Delete solution]  [Remove all repos from solution]│   │
│  └────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────┘
```

**Configuration scopes:**

| Setting | Scope | Stored in |
|---------|-------|-----------|
| Solution name, client, category, avatar | Solution | `~/.sensei/solutions/{id}/config.json` |
| Watch folders, include/exclude patterns | Per-repo within solution | `.sensei/config.yaml` in repo |
| ACP registration (MCP, hooks, skills) | Per-ACP, shared across solutions | ACP-specific config files |
| Repo roles, labels | Solution | `~/.sensei/solutions/{id}/config.json` |
| Manual connections | Solution | `~/.sensei/solutions/{id}/config.json` |
| Auto-reindex, debounce | Per-repo | `.sensei/config.yaml` in repo |

### Skills & Workflow

Skills are the behavioral layer — they tell the ACP *how* to work. Today they're one-size-fits-all: every skill is installed into every project regardless of stage, type, or team preference. This won't scale.

#### The Problem

- 19 plugin skills + 4 generated categories applied uniformly
- A greenfield prototype doesn't need `detecting-doc-drift` or `reformatting-docs`
- A mature production codebase doesn't need `building-app-mockups`
- The session-start hook injects JS package-manager rules into Rust/Python/Java projects
- Users can `/enable` and `/disable` individual skills, but there's no guidance on *which* to use *when*
- No way to customize skill behavior, add project-specific overrides, or compose workflows

#### Skill Tiers

Skills are organized into three tiers based on when they apply:

```
┌─────────────────────────────────────────────────────┐
│  Universal (always on)                              │
│  Session management, checkpoint, context efficiency │
│  These apply to every project, every stage.         │
├─────────────────────────────────────────────────────┤
│  Stage-specific (selected by project stage)         │
│  Greenfield: design, mockup, decomposing tasks      │
│  Growth: pattern extraction, doc creation, testing  │
│  Mature: drift detection, refactor, auditing        │
│  Maintenance: reverse-engineering, doc reformatting  │
├─────────────────────────────────────────────────────┤
│  Project-specific (generated per repo)              │
│  Orientation, workflow, context, patterns            │
│  Generated from codebase analysis. Unique per repo. │
└─────────────────────────────────────────────────────┘
```

#### Project Stages

Each solution has a **stage** that determines which skill tier is active:

| Stage | Description | Skills activated |
|-------|-------------|-----------------|
| **Greenfield** | New project, exploring ideas, rapid prototyping | design, mockup, decomposing-broad-tasks, building-app-mockups |
| **Growth** | Active development, building features, establishing patterns | pattern-based-development, identifying-patterns, guiding-doc-creation, test-gen, working-smarter |
| **Mature** | Stable product, focus on quality, documentation, optimization | detecting-doc-drift, refactor, extract-docs, context-efficiency, codebase-indexing |
| **Maintenance** | Legacy or handed-over code, understanding and updating | reverse-engineering, reformatting-docs, auditing, analyze |

Stage is either:
- **Auto-detected**: based on repo age, commit frequency, test coverage, doc density
- **User-set**: override in solution config

A solution can be in multiple stages if repos are at different maturity levels (e.g., new frontend + mature backend). In that case, the union of stage skills is available, and per-repo overrides are possible.

#### Skill Configuration UI

Accessible from the solution Configuration panel, under a "Skills & Workflow" section:

```
┌─ Skills & Workflow: Acme Platform ──────────────────────────┐
│                                                             │
│  Stage: [Growth ▼]          [Auto-detect]                   │
│                                                             │
│  Universal (always on)                                      │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ ☑ session-management    Session start protocol       │   │
│  │ ☑ context-efficiency    Smart context loading        │   │
│  │ ☑ working-smarter       Commit-first discipline      │   │
│  │                                         [cannot disable]│   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Growth stage                                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ ☑ pattern-based-development  Check PATTERNS.md first │   │
│  │ ☑ identifying-patterns       Extract new patterns    │   │
│  │ ☑ guiding-doc-creation       Doc creation guidelines │   │
│  │ ☑ test-gen                   Generate test coverage  │   │
│  │ ☐ decomposing-broad-tasks    Break down large tasks  │   │
│  │                                    [enable/disable ▼]│   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Other stages (not active — click to preview)               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ ▸ Greenfield (4 skills)                             │   │
│  │ ▸ Mature (5 skills)                                 │   │
│  │ ▸ Maintenance (4 skills)                            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Project-specific (generated)                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ acme-api:                                           │   │
│  │   ☑ orientation  ☑ workflow  ☑ context  ☑ patterns  │   │
│  │   Generated 2h ago   [Regenerate]  [Edit ✎]         │   │
│  │ acme-ui:                                            │   │
│  │   ☑ orientation  ☑ workflow  ☑ context  ☑ patterns  │   │
│  │   Generated 2h ago   [Regenerate]  [Edit ✎]         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Custom Skills                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ No custom skills yet.                               │   │
│  │ [+ Add custom skill]  [Import from file]            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Commands                                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ /session  /checkpoint  /commit  /backlog            │   │
│  │ /feature  /product  /audit  /mockup                 │   │
│  │ /pattern-extract  /pattern-use                      │   │
│  │ /enable  /disable  /help                            │   │
│  │                                                     │   │
│  │ [+ Add custom command]                              │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### Custom Skills & Overrides

Users can:

1. **Edit generated skills** — click "Edit" to open the skill markdown in a text editor. Edits are saved as overrides in `~/.sensei/solutions/{id}/skill-overrides/`. The original generated version is preserved; the override takes precedence.

2. **Add custom skills** — write a SKILL.md with frontmatter (trigger description, activation conditions). Custom skills are stored in `~/.sensei/solutions/{id}/custom-skills/` and installed alongside standard skills.

3. **Disable stage skills** — uncheck any non-universal skill. Disabled skills are tracked in `~/.sensei/solutions/{id}/config.json` under `disabledSkills: string[]`.

4. **Add custom commands** — write a command markdown file. Stored in `~/.sensei/solutions/{id}/custom-commands/`. Available via `/command-name` in the ACP.

#### Skill Delivery to ACPs

When skills are installed to an ACP, the resolution order is:

```
1. Universal skills (always included)
2. Active stage skills (minus disabled ones)
3. Generated per-repo skills (orientation, workflow, context, patterns)
4. Custom skills (solution-level)
5. Skill overrides (replace generated versions)
```

The session-start hook applies the same resolution:
- Detect the current repo's solution membership
- Look up the solution's stage
- Load only the applicable skill set
- Inject language-appropriate rules (not JS-only package-manager rules for all projects)

#### Skill Data Model

```typescript
interface SkillConfig {
  stage: ProjectStage | "auto";
  disabledSkills: string[];          // skill IDs disabled by user
  customSkills: CustomSkill[];
  customCommands: CustomCommand[];
  overrides: Record<string, string>; // skillId → override file path
}

type ProjectStage = "greenfield" | "growth" | "mature" | "maintenance";

interface SkillDefinition {
  id: string;
  name: string;
  description: string;
  tier: "universal" | "stage" | "generated" | "custom";
  stages?: ProjectStage[];           // which stages this skill applies to
  trigger: string;                   // when this skill activates
  languages?: string[];              // language filter (null = all)
}

interface CustomSkill {
  id: string;
  name: string;
  path: string;                      // path to SKILL.md
  createdAt: string;
}

interface CustomCommand {
  id: string;
  name: string;                      // becomes /name in the ACP
  path: string;
  createdAt: string;
}
```

#### Stage Auto-Detection Heuristics

| Signal | Greenfield | Growth | Mature | Maintenance |
|--------|-----------|--------|--------|-------------|
| Repo age | < 3 months | 3-12 months | > 12 months | > 24 months |
| Commit frequency | Bursts | Steady | Slowing | Sporadic |
| Test files | < 5% of files | 5-15% | > 15% | Any |
| Doc files | 0-2 | 3-10 | > 10 | Any |
| Contributors last 90d | 1-2 | 2+ | 2+ | 0-1 |
| PATTERNS.md exists | No | Possibly | Yes | Yes |
| CI/CD configured | No | Possibly | Yes | Yes |

When signals conflict (e.g., old repo with few docs), the system picks the more conservative stage and lets the user override.

### Global Settings (unchanged page, but expanded)

```
Settings
├── Workspace
│   ├── Scanned folder roots (where repos were imported from)
│   ├── Reset workspace
│   └── Export/Import workspace config
├── Daemon
│   ├── Port (default 7744)
│   ├── Auto-start on launch
│   ├── Concurrency (worker pool size)
│   └── Ollama model selection
├── ACP Registry
│   ├── Detected ACPs with status
│   ├── Configure/Reconfigure each
│   └── OTLP endpoint
└── About
    ├── Version
    └── Check for updates
```

### Architecture

Sub-views toggled by tabs within the page:

**Structural** — component boundaries, import arrows, community clusters. For developers.

**Deployment** — derived from infra files. Services, databases, queues, external APIs. For DevOps and non-technical stakeholders.

```
┌─────────────────────────────────────────────────────┐
│  Deployment: Acme Platform                          │
│                                                     │
│  ┌─────────────────────────────────────┐           │
│  │           Kubernetes Cluster         │           │
│  │  ┌──────────┐    ┌──────────┐       │           │
│  │  │ acme-ui  │    │ acme-api │       │           │
│  │  │ :3000    │───▶│ :8080    │       │           │
│  │  │ 2 pods   │    │ 3 pods   │       │           │
│  │  └──────────┘    └─────┬────┘       │           │
│  │                        │             │           │
│  │                   ┌────▼─────┐       │           │
│  │                   │PostgreSQL│       │           │
│  │                   │  :5432   │       │           │
│  │                   └──────────┘       │           │
│  └─────────────────────────────────────┘           │
│                                                     │
│  Derived from: k8s/deployment.yaml, k8s/service.yaml│
└─────────────────────────────────────────────────────┘
```

**Data Flow** — API endpoints, database tables, message topics. For architects.

### Traceability

Chain: **Requirements → Design → Code → Tests**

```
┌───────────────────────────────────────────────────────────┐
│  Traceability: Acme Platform                              │
│                                                           │
│  Filter: [All repos ▼]  [All types ▼]  [Gaps only ☐]    │
│                                                           │
│  Requirement        Design          Code         Tests    │
│  ──────────────────────────────────────────────────────── │
│  AUTH-001           auth-flow.md    login.ts     ✓ 3      │
│  User login         (acme-api)      (acme-api)   tests    │
│                                                           │
│  AUTH-002           auth-flow.md    oauth.ts     ✓ 2      │
│  OAuth support      (acme-api)      (acme-api)   tests    │
│                                                           │
│  PAY-001            payments.md     stripe.ts    ✗ 0      │
│  Stripe payments    (acme-api)      (acme-api)   ← gap    │
│                                                           │
│  UI-003             —               Button.tsx   ✓ 1      │
│  Button redesign    ← gap           (acme-ui)    test     │
│                                                           │
│  Coverage: 3/4 requirements → code  ·  2/4 fully tested  │
└───────────────────────────────────────────────────────────┘
```

Works across repos: requirement doc in `acme-docs`, implementation in `acme-api`, tests in the same repo. Connected via solution membership + COVERS/MENTIONS_FN graph edges.

### Sessions

Same as today but scoped: only shows sessions for repos in the selected solution.

### All Repos

The global flat list. Starting point for import. Shows:
- **Unassigned** — recently scanned, not yet in a solution
- **All** — every repo, with solution membership shown

From here users can:
- Scan a folder to import repos
- Drag repos into solutions
- Tag roles / assign clients
- See which repos aren't in any solution

## Data Model

### Solution

```typescript
interface Solution {
  id: string;
  name: string;
  description?: string;
  client?: string;               // optional client/org tag
  category: "active" | "side" | "idea";
  repos: SolutionRepo[];
  createdAt: string;
  updatedAt: string;
}

interface SolutionRepo {
  repoId: string;
  path: string;
  role: RepoRole;
  label?: string;                // display name override
}

type RepoRole =
  | "backend" | "frontend" | "mobile" | "middleware"
  | "infra" | "docs" | "library" | "shared" | "unknown";
```

### Standalone Library detection

After all solutions are formed, scan for libraries referenced by 2+ solutions:

```typescript
interface StandaloneLibrary {
  repoId: string;
  path: string;
  name: string;
  usedBy: string[];              // solution IDs
}
```

A repo is a standalone library if:
- It has role `"library"` in any solution AND
- It appears in 2+ solutions

These are displayed in the sidebar under "Libraries" and show "used by X, Y" on the overview.

### Solution auto-detection

On import, run these heuristics in order:

1. **Monorepo** — `workspaces` in package.json, `[workspace]` in Cargo.toml, `include` in settings.gradle → create solution with components
2. **Name-prefix** — `acme-api`, `acme-ui`, `acme-shared` share prefix `acme` → suggest solution "Acme"
3. **GitHub org** — repos under same org with similar names → suggest grouping
4. **Cross-imports** — if repo A's package.json references repo B → suggest grouping
5. **Infra co-location** — docker-compose.yml references services matching other repo names → suggest grouping

User always confirms. System never auto-groups without consent (except monorepos, which are inherently one solution).

### Infra detection

| File pattern | Detected as | Extracted info |
|-------------|-------------|----------------|
| `Dockerfile` | Deployable service | Base image, exposed ports |
| `docker-compose.yml` | Service topology | Services, networks, volumes, dependencies |
| `k8s/**/*.yaml` | K8s deployment | Deployments, services, ingress, config maps |
| `Tiltfile` | Dev orchestration | Service names, port forwards |
| `terraform/**/*.tf` | Cloud infra | Resources (RDS, S3, SQS, etc.) |
| `serverless.yml` | Serverless | Functions, triggers, endpoints |
| `.github/workflows/*.yml` | CI/CD | Build/deploy targets |
| `Procfile` | Heroku/process | Process types |

### Doc classification

| Signal | Type | Used in traceability |
|--------|------|---------------------|
| `docs/requirements/`, `specs/`, `PRD` in title | requirement | Left column |
| `docs/design/`, `docs/architecture/`, `ADR-` prefix | design | Second column |
| `CHANGELOG`, `RELEASE` | changelog | — |
| `README` | overview | — |
| `docs/api/`, `openapi.yaml`, `swagger.json` | api-spec | — |
| `docs/runbook/`, `docs/ops/` | operations | — |
| User-assigned via frontmatter `type: requirement` | any | Explicit override |

## Routes

```
/(app)/                                → redirect to last active solution
/(app)/s/[id]/                         → Overview dashboard
/(app)/s/[id]/repos                    → Repos list (expandable)
/(app)/s/[id]/arch                     → Architecture (structural/deployment/data)
/(app)/s/[id]/trace                    → Traceability
/(app)/s/[id]/sessions                 → Sessions scoped to solution
/(app)/all                             → All repos (import, assign, manage)
/(app)/acp                             → ACP registry
/(app)/settings                        → Settings
```

## Pages Removed

| Current | Replaced by |
|---------|------------|
| `/projects` | Solution Overview + Repos |
| `/indexer` | Inline in Repos (expand card → Index tab) |
| `/graph` | Architecture → Structural |
| `/libraries` | Sidebar "Libraries" section (standalone) + inline in Repos |
| `/ideas` | Sidebar "Ideas" category (solutions with category=idea) |
| `/sessions` | Scoped to solution |

## ACP Context

The solution model enriches `get_session_context` / `get_bearings`:

```json
{
  "solution": {
    "name": "Acme Platform",
    "repos": [
      { "name": "acme-api", "role": "backend", "stack": "express+typescript" },
      { "name": "acme-ui", "role": "frontend", "stack": "react+typescript" },
      { "name": "design-system", "role": "library", "stack": "typescript", "standalone": true }
    ],
    "connections": [
      { "from": "acme-ui", "to": "acme-api", "via": "REST API" },
      { "from": "acme-api", "to": "design-system", "via": "npm import" },
      { "from": "acme-ui", "to": "design-system", "via": "npm import" }
    ],
    "infra": {
      "services": ["acme-ui:3000", "acme-api:8080"],
      "databases": ["postgresql:5432"],
      "deployedVia": "kubernetes"
    }
  },
  "currentRepo": {
    "name": "acme-api",
    "role": "backend",
    "relatedDocs": ["docs/design/auth.md", "specs/AUTH-001.md"],
    "impactNote": "design-system is also used by Internal Tools — changes propagate"
  }
}
```

The ACP now knows:
- This is the backend of a 3-repo solution
- It connects to the frontend via REST and shares a library
- That library affects another solution too
- There are specific requirement/design docs for the area being modified

## Implementation Phases

### Phase 1: Solution model + workspace navigation
- Solution entity in localStorage + server
- Solution CRUD: create, name, add/remove repos, assign roles, set category
- Auto-detect monorepos as solutions
- Sidebar with Active/Libraries/Side/Ideas sections
- Solution selector with sub-navigation
- Overview page (stats, connection diagram placeholder)
- Repos page (merged view with expandable cards)
- Remove old pages (projects, indexer, graph, libraries, ideas)
- Migrate existing localStorage projects_raw → solutions

### Phase 2: Auto-detection + architecture views
- Name-prefix and org-based solution suggestions on import
- Infra file detection (Docker, k8s, compose, terraform)
- Connection diagram on Overview (auto-derived)
- Deployment view in Architecture
- Standalone library detection (2+ solution references)
- Structural view from graph communities

### Phase 3: Traceability + doc classification
- Auto-classify docs by folder convention and content
- Trace chain: requirement → design → code → test
- Cross-repo traceability within solutions
- Coverage metrics and gap detection
- Drift alerts: "doc X covers code Y which changed since doc was last updated"

### Phase 4: ACP integration
- Feed solution context into get_session_context / get_bearings
- Cross-repo awareness in search and context_pack
- Impact analysis: "this library is used by solutions X, Y — here are the callers"
- Solution-scoped recommendations
