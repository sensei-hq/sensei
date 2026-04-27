# Design Requirements — From Claude Design Session

> Extracted from `setup/transcript.md` and design artifacts (JSX + screenshots).
> Source: Claude Design session 2026-04-22.

---

## Design System — "Zen/Sumi"

### Tokens

| Token | Light | Dark | Usage |
|-------|-------|------|-------|
| `--paper` | warm off-white | dark charcoal | Page background |
| `--paper-2` | slightly deeper | slightly lighter | Cards, sidebar |
| `--paper-3` | inset areas | inset areas | Muted backgrounds |
| `--paper-edge` | hairlines | hairlines | Borders |
| `--sumi` | near-black ink | near-white | Primary text |
| `--sumi-2` | dark grey | light grey | Secondary text |
| `--sumi-3` | mid grey | mid grey | Tertiary/metadata |
| `--sumi-4` | light grey | dark grey | Faint text |
| `--shu` | vermillion | bright vermillion | The one accent color |
| `--jade` | green | green | Positive/success |
| `--amber` | amber | amber | Warning |

### Typography

| Class | Font | Usage |
|-------|------|-------|
| `.display` | Fraunces (serif) | Headlines, hero text |
| Default | Inter | UI text, labels, body |
| `.mono` | JetBrains Mono | Code, numbers, metrics |
| `.kanji` | Yu Mincho / Hiragino Mincho | Section markers, decorative kanji |

### Kanji Usage

| Kanji | Meaning | Where used |
|-------|---------|------------|
| 先 | "first/ahead" | Logo — Sensei |
| 観 | "observe" | Observatory ready / watching |
| 師 | "teacher" | Teaching concept |
| 静 | "quiet/calm" | Local / privacy |
| 聴 | "deep listening" | Hero teaching |
| 繰 | "repeat" | Pattern recurring |
| 昇 | "rise/promote" | Teaching adopted |
| 探 | "search/detect" | Drift detected |
| 場 | "place" | Projects navigation |
| 工 | "craft" | Project icon (Lumen Studio) |
| 雲 | "cloud" | Project icon (Lumen Cloud) |
| 紋 | "crest/pattern" | Project icon (Brand Kit) |

**Setup step markers:** 一二三四五六七八九 (1-9 in kanji)

**Role kanji:** 後 (backend), 前 (frontend), 書 (library), 記 (docs), 基 (infra)

---

## Setup Wizard — 9 Steps

### Step 1: Welcome
- Full-bleed hero: "A teacher does not write the code."
- Three pillars: Observe (FTR · turns · corrections), Teach (patterns · rules · skills), Local (on your machine)
- "~4 minutes · nothing leaves your machine"
- ACP compatibility list: claude-code, cursor, codex, aider

### Step 2: Components
- Check/install: sensei-cli, MCP bridge, sensei-daemon
- Three sub-states: Fresh install / Partial / All present
- Shows version + status per component (missing/installed/stopped/ready)
- "Nothing leaves localhost:PORT"

### Step 3: Assistants (ACPs)
- Detect installed AI coding platforms
- Card per ACP: name, version, path, found/not-found, checkbox
- "Registers plugins, skills, commands, agents, logging and metrics"

### Step 4: Folders
- Text input + Browse button (native folder picker)
- List of added folders with "recursive" toggle and remove
- "You can manage folders and exclusions later from Settings"

### Step 5: Scan
- Live SSE event log with timestamps
- Stats bar: roots, repos discovered, projects detected, files processed
- Per-project cards showing repo list with progress bars
- Right-side real-time event ticker

### Step 6: Projects
- "A project has one or more repos. Edit, split, or confirm."
- Auto-grouped cards: name, path, repo count, MULTI-REPO badge
- Per-repo: name, file count, role badge (editable dropdown), role kanji
- Actions: merge, edit, split, confirm checkbox
- "More options — external integrations, clients, custom rules — per project later from its Settings"

### Step 7: Libraries
- "Libraries without their own MCP — sensei indexes docs & code and wraps them with its own tools"
- Filter chips: detected count / will-be-wrapped count
- Per-library: name, version, language, usage count, doc status (indexed/partial/schema only/no docs)
- Each row shows "why" it's being wrapped
- "+ Add a library" button (name + URL)

### Step 8: MCP Registry
- "Sensei recommends these based on what it detected in your stack"
- Detected stack shown as chips (languages, frameworks, services)
- Split: "Recommended for your stack" / "Also available"
- Per-MCP: name, publisher, summary, tool count, verified badge, install toggle
- Each shows which stack item triggered the recommendation

### Step 9: Enter (Done)
- 観 kanji hero
- "The observatory is ready."
- "Start a session with your assistant. Sensei will watch in silence for a few days, then begin to teach."
- Stats: projects, repos, libraries, MCPs, assistants
- "— · the first session is always the teacher"
- Button: "Enter observatory →"

---

## Observatory (Home Screen)

### Layout
- Left sidebar: Projects (active + recent) + Sections
- Main area: Hero teaching + insights

### Sidebar Sections
| Section | Kanji | Label |
|---------|-------|-------|
| Projects | (per-project kanji) | Active projects list |
| Libraries | 書 | Indexed libraries |
| MCP Registry | (new) | Registered MCPs |
| MCP Playground | (new) | Tool interaction |
| Teachings | 教 | Learned patterns/rules |
| Sessions | 録 | Session history |
| Settings | 設 | Configuration |

### Hero Teaching
The most important insight right now. Always exactly one.

**Mature state** (enough data):
- Large kanji (聴)
- Koan: "The AI does not know your auth."
- Body: explains what happened across sessions
- Impact: "Projected FTR +14% in Lumen Cloud"
- Action button: "Draft a persona"
- Source: links to sessions

**Early state** (not enough data):
- 観 kanji
- "Still listening."
- "Sensei has watched 4 sessions so far."
- "~2-3 more sessions until first lesson"

### Insights (below hero)
Never more than 3. Each has:
- Kanji marker
- Label (e.g., "Pattern recurring", "Teaching adopted", "Drift detected")
- Description
- Tag (e.g., "3rd time", "+7% FTR")
- Tone: warn / good / mute

---

## Project Pages — Top Tabs Layout

Tabs: Overview · Sessions · Libraries · Patterns · Settings

### Overview
- Stats cards: sessions 7d, FTR, turns/session, tokens
- Connection diagram (repo topology)
- FTR trend sparkline
- Recent sessions list

### Patterns (new)
Two sides:
- **Patterns to follow**: Adapter, Factory, Observer, CRDT commutativity, etc.
  - Status: rule / suggested / gap
  - Instance count, example locations
- **Anti-patterns to avoid**: Duplication, god nodes, monolithic code, dead code
  - Severity: critical / warning / info
  - Occurrence count, locations
  - Suggested fix: "This duplication could become an Adapter"

### Settings (v2 — document + summary rail)
- Left rail (280px sticky): 80px icon, name, client, goal, quick facts, anchor nav
- Right side (scrollable): Identity · Stack · Repos · Links · Guidelines · Backlog
- Identity: compact — icon upload, name, client (just a name), goal (one line)
- Stack: detected languages, frameworks, runtimes, services as chips
- Links: external URLs (Jira, Confluence, Figma, docs, dashboards)
- Guidelines: enforceable rules for this project
- Backlog: tracked issues/tasks

---

## Libraries Page
- Unified list (no detected vs imported split)
- Filter chips by kind: All / Code / Services
- Language filter
- Search
- Detail panel on right: sections, doc preview, MCP tool examples

---

## MCP Registry (new top-level page)
- Global catalog of available MCPs
- Per-project: which are installed
- Recommendations driven by project Stack
- Split: Recommended / Also Available
- Per-MCP: name, publisher, tool count, verified badge, install toggle

---

## MCP Playground (new top-level page)
- Lists sensei MCP tools + any installed third-party MCP tools
- MCP selector at top (default: Sensei, switch to any installed)
- Per tool: name, description, input parameters
- Interactive: fill params, invoke, see result
- For third-party MCPs: tools listed from their manifest

---

## Projects Navigation
- Cards grid with search + status filter
- Status chips: All / Active / Dormant / Archived (with kanji: 全 動 眠 蔵)
- Dense cards: project name, kanji icon, client, FTR, sessions, repo count
- Dormant/archived cards show without stats
- "+ new project" button

---

## Daemon Gaps (features needed to support this design)

### Must build

| Feature | Impact | Current state |
|---------|--------|---------------|
| **Stack detection** | Drives MCP recommendations, project identity | Not implemented — need to detect from manifests/Dockerfiles |
| **MCP registry** | New concept: global catalog + per-project installs | Not implemented |
| **Component health** | Setup step 2 checks CLI/MCP/daemon status | Partially exists (health endpoint) |
| **Hero teaching system** | The core observatory insight | Not implemented — needs session analysis + FTR correlation |
| **Pattern detection (follow + anti)** | Patterns page needs both types | Partially exists (detected_patterns table, no anti-patterns) |
| **Project icons** | Display in sidebar/cards | Implemented (icon scanner) but not exposed via API |
| **External links per project** | Settings Links section | Implemented (link scanner) but not exposed via API |

### Needs daemon API

| Endpoint | Purpose |
|----------|---------|
| `GET /api/repos/{id}/metadata` | Return icon, links, summary from metadata column |
| `GET /api/stack/{repoId}` | Detected technology stack |
| `GET /api/mcps` | Global MCP registry |
| `POST /api/mcps/install` | Install MCP for a project |
| `GET /api/teaching` | Current hero teaching + insights |
| `GET /api/patterns/{repo}/antipatterns` | Duplication, god nodes, dead code |
| `GET /api/health/components` | CLI + MCP bridge + daemon versions/status |

### i18n considerations
- Kanji are decorative, not functional — they're section markers, not translated text
- All user-facing text should be in English (or localizable)
- Kanji meanings should be verified for accuracy (see table above)
