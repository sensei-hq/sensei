# Implementation Plan

---

## What Already Exists (Keep)

These are production-quality and carry forward without rewrite:

| Package | Capability | Notes |
|---|---|---|
| `packages/engine` | Symbol extraction, L0–L3 resolution, ranking, context packs, library indexing | Adapts to write SQLite instead of Supabase |
| `packages/server` | MCP server — all tools used by Claude Code | No changes needed |
| `packages/collector` | Session event collection, FTR scoring | Adapts to write SQLite |
| `packages/cli` | `sensei init`, `sensei index`, `sensei doctor` | Keep; used in CI and headless contexts |
| `apps/dashboard` | SvelteKit frontend — analytics, FTR, sessions, library views | Embedded in Tauri; new routes added |

Session protocol, skills, hooks, and library intelligence all carry forward unchanged.

---

## Phase 0 — Simplify the Foundation

**Goal:** Remove Supabase as a dependency. Replace with SQLite. Zero Docker.
Establish the coordinator adapter boundary so Claude Code assumptions are not baked deeper.

**Why first:** Everything else builds on the data layer. Getting this right unlocks
all future work. Also: this is the single biggest friction point for new users today.

**Deliverables:**

- [ ] `packages/shared` — replace Supabase client with SQLite client (Bun's built-in `bun:sqlite`)
- [ ] `packages/engine` — rewrite Supabase writes to SQLite writes (symbols, call_edges, imports, embeddings)
- [ ] `packages/collector` — rewrite session/FTR writes to SQLite
- [ ] `apps/dashboard` — rewrite Supabase reads to SQLite reads
- [ ] Schema: define `~/.sensei/sensei.db` (global) and `<repo>/.sensei/index.db` (per-repo)
- [ ] SQLite FTS5 for keyword search (replaces BM25 in Supabase)
- [ ] SQLite-vec (or manual cosine in JS) for semantic search (replaces pgvector)
- [ ] `sensei init` no longer requires Supabase URL — detects stack, creates SQLite, done
- [ ] All existing tests pass against SQLite
- [ ] `CoordinatorAdapter` interface defined in `packages/shared` (see `05-coordinator-adapters.md`)
- [ ] `ClaudeAdapter` refactored to implement the full `CoordinatorAdapter` interface
- [ ] `CoordinatorRegistry` with auto-detection; Claude Code registered as default
- [ ] `sensei init` selects coordinator via registry, not hard-wired Claude Code paths
- [ ] Confidence tagging on all graph edges: `EXTRACTED` / `INFERRED` / `AMBIGUOUS` (see `06-graph-intelligence.md`)
- [ ] "Why" extraction: parse `// NOTE:`, `// WHY:`, `# IMPORTANT:` comments into `Rationale` nodes

**Acceptance:** `sensei init` on a fresh machine with no Docker, no cloud accounts,
no environment variables. `sensei index` runs. `sensei doctor` reports clean.
Dashboard loads FTR and session data.

---

## Phase 1 — Tauri Shell

**Goal:** The SvelteKit dashboard runs as a native desktop app. Two-binary model established.

**Why second:** Users need a proper app, not a localhost tab. Also establishes the IPC
boundary before we build features on top of it.

**Deliverables:**

- [ ] `apps/desktop` — new Tauri app (Rust + Tauri v2)
- [ ] Embed existing SvelteKit app as the webview
- [ ] Rust commands: open-folder dialog, get-git-status, start/stop daemon, check-ollama
- [ ] Daemon starts automatically when the desktop app launches
- [ ] System tray: daemon status, quick-open workspace
- [ ] macOS and Windows installers via Tauri bundler
- [ ] Auto-update via Tauri updater against GitHub Releases
- [ ] `sensei` daemon registered as a login item (optional, user-controlled)

**Acceptance:** Download installer, open app, point at a repo folder. Daemon starts.
Dashboard shows indexed data. Closing the app offers to stop or background the daemon.

---

## Phase 2 — Workspace and Project Registry

**Goal:** Sensei knows about all the developer's projects and ideas, with maturity signals.

**Why third:** Before building the card system, the container model needs to exist.

**Deliverables:**

- [ ] SQLite tables: `projects`, `ideas`, `maturity_signals`
- [ ] Workspace view: Recent, Active, Ideas, All, Archived tabs
- [ ] Add project: open folder, detect stack, index, register
- [ ] Add idea: title + optional description, no repo required
- [ ] Maturity level: computed from card count, code presence, last activity
- [ ] Maturity indicators in the list view
- [ ] Archive / unarchive
- [ ] Quick search across project and idea names

- [ ] God node detection: top-N symbols by degree surfaced in project view and `get_session_context`

**Acceptance:** Register 10 repos and 5 ideas. Each shows correct maturity level.
Recent view shows last 5 touched. Archive one project — it disappears from main views.

---

## Phase 3 — Cards and Phase Pipeline

**Goal:** Projects and ideas can have phases and cards. Links between cards and code symbols.

**This is the core of the new model.**

**Deliverables:**

- [ ] SQLite tables: `phases`, `cards`, `card_links`
- [ ] Default phase set created on project init (optional — user can decline)
- [ ] Phase pipeline view: horizontal lane per phase, cards as tiles
- [ ] Card CRUD: create, edit, link, change status (open / accepted / deferred / superseded)
- [ ] Card links: card→card, card→symbol, card→file, card→decision
- [ ] Bidirectional edges written to graph on link creation
- [ ] Catalog view: cards by type across phases
- [ ] Phase completion indicator (% of cards resolved)
- [ ] Exploration phase: no required fields, scratchpad behaviour
- [ ] Leiden community detection on the symbol graph (`graphology-communities-leiden`)
- [ ] Hyperedge table in SQLite: group relationships connecting 3+ nodes
- [ ] Graph narrative report: output of `/analyze-repo` — god nodes, community clusters, surprising connections, suggested questions (Graphify-style `GRAPH_REPORT.md`)
- [ ] Card system as compounding wiki: re-running `/analyze-repo` updates existing cards, not duplicates

**Acceptance:** Create a project. Add Requirements and Design phases. Create 5 requirement
cards. Link 2 of them to code symbols. Run the catalog view — see all cards of each type.
Mark 3 cards as accepted — phase completion updates.

---

## Phase 4 — Prompt Workspace

**Goal:** The prompt bar is context-aware and cites sources. Built-in commands work.

**This is the primary interaction model.**

**Deliverables:**

- [ ] Persistent prompt bar at the bottom of every view
- [ ] Context detection: what is open determines what the prompt can reference
- [ ] Claude integration: prompt bar sends to Claude via MCP, responses stream in
- [ ] Citation rendering: responses include links to source cards and symbols
- [ ] Built-in commands: `/gap-analysis`, `/analyze-repo`, `/trace`, `/find-orphans`,
      `/phase-summary`, `/design-review`, `/token-estimate`, `/decision-log`,
      `/library-status`, `/session-recap`
- [ ] `/analyze-repo` auto-generates Analysis phase cards from the symbol graph
- [ ] Command autocomplete in the prompt bar
- [ ] Response cards: prompt outputs can be saved as cards with one click

**Acceptance:** Open a project. Run `/gap-analysis`. Get a list of requirements with
no implementation, each linking to the relevant card. Ask "what does the auth module do" —
get an answer citing specific files and symbols.

---

## Phase 5 — Local Inference Integration

**Goal:** Ollama/Gemma handles description generation during indexing. Fully offline indexing.

**Why after Phase 4:** Local inference is an enhancement to indexing quality,
not a prerequisite for the core workflow.

**Deliverables:**

- [ ] Ollama detection at daemon startup (check localhost:11434)
- [ ] Gemma 3 2B as default model for indexing tasks
- [ ] L1 description generation: one-sentence summaries during index run
- [ ] Card suggestion: after `/analyze-repo`, suggest Analysis cards based on symbol patterns
- [ ] Fallback: if Ollama not available, skip L1 generation (use L0 signatures only)
- [ ] Settings view: configure Ollama endpoint, model choice, enable/disable
- [ ] `sensei import-graph <path>` — ingest a graphify `graph.json` into the SQLite graph (adds cross-type edges: code ↔ paper ↔ image ↔ doc)

**Acceptance:** With Ollama running, `sensei index` produces L1 descriptions for every symbol.
Without Ollama, indexing completes normally with L0 only. No error, no prompt to install.

---

## Phase 6 — Distribution and Sustainability

**Goal:** Anyone can install sensei in under 2 minutes. Donation link present but unobtrusive.

**Deliverables:**

- [ ] GitHub Releases with signed installers for macOS (arm64 + x86_64), Windows, Linux
- [ ] `brew install sensei` via a Homebrew tap
- [ ] Onboarding flow on first launch: open a folder, index, done
- [ ] "Support sensei" link in settings (Buy Me Coffee / GitHub Sponsors)
- [ ] Optional anonymous usage ping: count of sessions per week, no code or prompts
  (explicit opt-in, described plainly)

---

## Deferred

These are valid ideas but not on the critical path:

| Item | Why deferred |
|---|---|
| Telemetry for enterprise detection | Need trust established first; donation model first |
| Team/multi-user mode | Single-user model ships first; team is additive |
| Custom Rust editor component | High effort, separate product track; validate workflow in SvelteKit first |
| SaaS/cloud sync | Deliberately removed; revisit only if there is clear user demand |
| Mobile companion | Interesting but not the core use case |

---

## Principles for This Build

**Keep the engine.** Every phase builds on `packages/engine`. Do not rewrite it in Rust.
The TypeScript is well-tested and correct.

**SQLite before Tauri.** Phase 0 must be complete before Phase 1. Building on Supabase
inside a Tauri shell is not a stepping stone — it is a trap.

**Ship each phase independently.** Each phase produces something usable. Phase 0 is a
better CLI tool. Phase 1 is a desktop app. Phase 2 is a project registry. No phase
requires the next phase to be useful.

**Prompts are additive, not replacement.** Built-in commands do not replace the prompt bar.
They are shortcuts for common structured queries. Freeform prompts always work alongside them.

**No forced structure.** Every phase of the implementation plan must respect the workspace
model principle: phases and cards are available, never required.
