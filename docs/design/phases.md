---
type: design
date: 2026-08-18
status: draft
supersedes: reframes the phase sequencing from `plan/2026-07-20-phases-1-3-plan.md`
---

# Phases — incremental delivery toward the operating model

> The operating model (v2 vision) describes the full system: intake playbooks,
> the Planner, governance enforcement, brownfield onboarding, non-code projects,
> design subsystems. Most of it is unbuilt. The *proven* parts — the code graph,
> the learning loop, the relay engine — are independently valuable today. This doc
> defines **four phases**, each shipping value on its own, each making the next
> phase possible. Phase N does not require Phase N+1 to be useful.
>
> The existing `plan/2026-07-20-phases-1-3-plan.md` covers mock gaps, verify-only
> stories, and bug lists — the tactical detail of *what's broken*. This doc is the
> strategic view of *what ships when and why*. The two complement each other; this
> one should be read first.

---

## 1. The core insight driving the phases

The operating model honestly diagnoses the adoption failure:

> *"Claude Code, wired to the full Sensei MCP surface, still reaches for
> `grep`/`read` instead of the graph, memories, and mindsets. The intelligence
> is offered, not routed."* — `operating-model.md:47-48`

**Phase 1 fixes this.** Everything else — Dōjō, Relay, the Planner — is
downstream of "does the AI actually use the context?"

---

## 2. Phase overview

| Phase | Name | Ships | Exit criterion | Refs |
|-------|------|-------|----------------|------|
| **1** | **Context pushes itself** | AI uses the graph without being asked | The assistant resolves a question using a sensei memory or pattern *without the user invoking an MCP tool* | §3 |
| **2** | **Dōjō as the join surface** | Team shares rules, patterns, memories | A second person joins the Dōjō and sees the team's shared knowledge | §4 |
| **3** | **Relay for one** | Long runs supervised from anywhere | You leave for lunch and a run completes + gates you from your phone | §5 |
| **4** | **Dōjō as the governance plane** | Knowledge compounds across teams | A pattern from project A improves FTR in project B | §6 |

Each phase is independently useful. Phase 1 makes the existing build reach
the AI. Phases 2-4 build the Dōjō flywheel, but each layer adds value
without requiring the next.

---

## 2.1 Release cuts — sequencing to a quick v1

**v1 ships locally, without Dōjō or Relay.** The releasable local product is three
work-streams; two are mostly built, one is the big foundational rebuild. What's *done*
vs the *gap* sets the priority:

| Stream | Spec / phase | Done vs gap | v1? |
|---|---|---|---|
| **Metrics engine** — repo-grain, watermark, user-attributed quality | [`spec/2026-08-18-repo-grain-metrics-watermark-engine.md`](../spec/2026-08-18-repo-grain-metrics-watermark-engine.md) **P-A/P-B (local)** | **BIG** — foundational rebuild + scanner hardening (D15). Largest v1 item. | ✅ |
| **Library auto-discovery** | [`library-auto-discovery.md`](library-auto-discovery.md); Phase 1 P1.7/P1.8 | **SMALL** — infra exists; wire the trigger + session-start inject | ✅ |
| **Context pushes itself** | Phase 1 P1.1–P1.6 | **MEDIUM** — hooks exist; wire injection | ✅ (recommended) |
| Metrics **Dōjō sync / me-vs-team** | same spec **P-C** (enrollment/auth D16) | Dōjō-dependent | ⛔ post-v1 |
| Dōjō join surface | Phase 2 | join flow broken | ⛔ post-v1 |
| Relay for one | Phase 3 | P0–P4 done, P5 gap | ⛔ post-v1 |
| Governance flywheel + team metrics | Phase 4 | needs 2 + 3 | ⛔ v2 |

**v1 critical path:** Metrics P-A (scanner + schema + engine) → P-B (quality commit-walk),
in parallel with Libs P1.7/P1.8 and Context P1.1–P1.6 — the three are independent (no shared
tables). Metrics is the pacing item.

**Post-v1 order:** Dōjō join (Phase 2) **+** Metrics P-C (Dōjō sync — shares the enrollment/
auth seam, D16, and `sensei.dojo_memberships`) → Relay (Phase 3) → governance flywheel + team
metrics (Phase 4). The metrics spec's local half (P-A/P-B) is v1; its Dōjō half (P-C) rides the
Phase-2 Dōjō layer.

> Note: the metrics spec **supersedes** the assumed-metrics behind the Phase-1 FTR verify
> stories (P1.6) — those now measure against the repo-grain engine.

---

## 3. Phase 1 — "Context pushes itself"

**Theme:** the highest-leverage fix. The core loop (capture → graph → learn →
deliver → measure) is proven — but the *delivery* step is pull, not push.
Phase 1 makes context arrive in the AI's path without the user asking.

### 3.1 What exists today

The delivery surfaces are documented in
[`design/instruction-delivery-model.md`](instruction-delivery-model.md) §2:

| Surface | Push/pull | Content | Compaction survival |
|---------|-----------|---------|-------------------|
| S1 `~/.sensei/rules.md` | push (hook-read) | user + general scope only | no |
| S2 `get_rules` MCP tool | **pull** | full scope resolution | n/a |
| S3 `CLAUDE.md` pointer | push once | one-line pointer | yes |
| S4 SessionStart hook | push at start | rules head + lean mindset | no |
| S5 PreCompact hook | push on compaction | thinner refocus block | yes |
| S6 Skill/agent load | **pull** | skill body / subagent procedure | no |

**The gap:** S2 and S6 — the two surfaces that carry the most value (scoped
rules, patterns, context_pack) — are pull. The model must elect to call them.
The operating model's fix: **push, not pull** — context arrives in the model's
path before it reaches for grep.

**Library intelligence — what exists today:**

The library system is well-built but has one missing step in the pipeline:

| Capability | Status | Detail |
|------------|--------|--------|
| Dependency detection | ✅ auto | `resolve_libs` + `extract_deps` walk manifests → `project_libraries` |
| Library URL discovery | ✅ auto | `discover_lib_url` probes 7 llms.txt patterns per library name |
| llms.txt fetch + index | ✅ manual | `index_library` fetches, parses, stores per-component pages — but **requires user to call `add_library`** |
| Library capabilities (skills/agents) | ✅ manual | `sensei.library.json` manifest → `library_skills` / `library_agents` — but **only loaded for local sources** |
| Library docs serving | ✅ auto | `get_lib_docs`, `search_lib_docs`, `list_library_skills`, `list_library_agents` MCP tools |
| Library update scheduler | ✅ auto | Periodic re-index + version check + security advisory scan |
| **Auto-index after detection** | ❌ missing | After `extract_deps` detects a library, nothing discovers or indexes its docs automatically |

**The gap:** dependency detection is automatic, but doc indexing requires a
manual `add_library` call. The AI assistant must already know about a library
and ask for it — the opposite of "push, not pull." Phase 1 story P1.7 closes
this: detected dependency → auto-discover llms.txt + skills/agents → index → 
available to the AI without anyone calling `add_library`.

### 3.2 Stories

| ID | Title | Status | Acceptance criteria | Refs |
|----|-------|--------|-------------------|------|
| P1.1 | Auto-inject layered context at session start | build | SessionStart hook injects top-3 memories + top-3 patterns + top-3 rules into `additionalContext`; the model sees them without calling any MCP tool | `instruction-delivery-model.md` S4; `get_layered_context` MCP; `design/governance.md` §Rules |
| P1.2 | Context-pack as a hook (PreToolUse injection) | build | When the model calls `read_file`, `write_file`, or `grep` on a known codebase path, PreToolUse injects relevant context_pack hits as inline hints; the model uses the hints instead of re-grep-ing the same files | `design/remote.md` control channel; `mcp/src/lib.rs` `context_pack` |
| P1.3 | Library auto-pull on reference | build | When a session references a library name (e.g. "rokkit", "drizzle"), `search_lib_docs` is called and the top hit is injected; the model gets the component API without fetching docs manually | `mcp/src/lib.rs` `search_lib_docs` / `get_lib_docs`; `design/assistants.md` |
| P1.4 | Context-use logging (feedback signal) | build | When injected context is referenced in a subsequent assistant turn (the model quotes or uses a pattern/memory that was injected), log a `context_used` event; when it isn't, log `context_ignored`; this is the beginning of §9 playbook→outcome attribution | `playbook.rs`; `analysis/` analyzer pipeline; `plan/2026-07-19-learning-loop-*.md` |
| P1.5 | Mindset auto-invocation from blast radius | build | When the code graph shows a change touches auth, security-sensitive paths, or UI, the hook auto-loads the relevant mindset (security-reviewer, performance, ux); the model gets guidance without manual `/sensei:agent` invocation | `design/instruction-delivery-model.md` §push model; `marketplace/plugins/sensei/skills/` |
| P1.6 | Verify: FTR measurably improves | verify-only | After P1.1-P1.5 ship, measure FTR on a project with injected context vs without; the injected-context sessions should show higher FTR (or honest null if too few sessions) | `verdicts.rs`; `plan/README.md` G1 (FTR loop closed); `design/playbook.md` §learning loop |
| P1.7 | Auto-index detected dependencies (llms.txt + skills + agents) | build | When `extract_deps` detects a library from a manifest, a background task auto-discovers its llms.txt URL (via `discover_lib_url`), fetches and indexes the docs, and checks for a `sensei.library.json` manifest to register any declared skills/agents. No user action required — library docs are available to `get_lib_docs`/`search_lib_docs` within minutes of project scan. Libraries that have no llms.txt (or whose probe fails) are silently skipped — never an error. | §3.3 below; `indexer/lib_indexer.rs`; `tasks/handlers/libraries.rs`; `discover_lib_url` in `api/handlers/mcp.rs`; `libraries/manifest.rs` |
| P1.8 | Auto-inject library context at session start | build | SessionStart hook checks the project's `project_libraries` and injects summaries of the top-5 most-used libraries (by import frequency or relevance) into `additionalContext`; the model sees "this project uses rokkit (component library), drizzle (ORM), etc." and their key patterns without calling any MCP tool | P1.7 (docs must be indexed first); `instruction-delivery-model.md` S4; `get_lib_docs` MCP |

### 3.3 Design — auto-index detected dependencies (P1.7)

This is the pipeline that makes library intelligence automatic. Today:
`extract_deps` detects → nothing happens → user must call `add_library`. After
P1.7: `extract_deps` detects → background task discovers → fetches → indexes → 
skills/agents registered → available to the AI.

**Trigger:** the `resolve_libs` barrier task (runs after every scan reconcile)
already calls `extract_deps` which upserts into `project_libraries`. P1.7 adds
one step at the end of `extract_deps`: for each newly-detected or
version-changed library that has no `indexed_at` (i.e. docs were never fetched),
enqueue an `IndexLibrary` task.

**The auto-index task flow:**

```
extract_deps detects "rokkit" v2.1.0 from package.json
    │
    ▼
upsert_library("rokkit", "npm", "2.1.0", kind=detected)
    │
    ▼  (new or version changed → indexed_at IS NULL)
    │
enqueue IndexLibrary(library_id, source=auto-detect)
    │
    ▼  (async, background)
    │
discover_lib_url("rokkit", "")
    │  probes: rokkit.com/llms.txt, rokkit.dev/llms.txt, ...
    │  first hit > 50 bytes wins → returns URL
    ▼
index_library(library_id, url)
    │  fetch → parse → component derivation → upsert_library_page (×N)
    │  stamp_docs_applied_if_indexed
    ▼
load_manifest_from_root(url)  ← if local path, or if llms.txt references manifest
    │  look for sensei.library.json
    │  if present → replace_library_capabilities (skills + agents)
    ▼
library is now available:
    get_lib_docs("rokkit")         → component pages
    search_lib_docs("styling")     → hits rokkit styling docs
    list_library_skills("rokkit")  → declared skills
    list_library_agents("rokkit")  → declared agents
```

**Key design decisions:**

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Scope: detected deps only, or all deps?** | Detected only (from manifests) | Imported-only libs were manually added — the user already chose to index them. Auto-index applies to what the manifest says the project uses. |
| **Rate limiting** | Max 5 concurrent auto-index tasks; per-library 5s fetch timeout | A project with 200 deps shouldn't hammer 200 URLs simultaneously. The existing `discover_lib_url` already has 5s timeouts. |
| **Failure handling** | Silent skip, never error | Most libraries don't have llms.txt. A probe failure means "no docs available" — not a problem to surface. Log the attempt + failure reason for debugging, but don't block the scan pipeline. |
| **Skills/agents from non-local sources** | Probe for `sensei.library.json` at the library's documented URL | Today the manifest is only loaded from local paths. For website-sourced libraries, check if the llms.txt root also hosts a `sensei.library.json` (a natural extension of the manifest convention). |
| **Re-index cadence** | Per the existing library update scheduler | The scheduler already checks for new versions and re-indexes. Auto-index just seeds the first index; the scheduler keeps it current. |
| **User control** | `enabled` flag on `project_libraries` + a "skip auto-index" tag | Users can disable auto-index for specific libraries. A `tags` array entry like `["auto-index-skipped"]` prevents re-enqueuing. |

**What the user sees (nothing, by design):**

The whole point is zero UI. After a project scan, within a few minutes:
- `search_lib_docs("drizzle schema")` returns real Drizzle ORM docs
- `list_library_skills("rokkit")` returns styling/component skills
- The session start hook (P1.8) includes "This project uses: rokkit (v2.1), drizzle (v3.2), …"

The user never called `add_library`. The docs just appeared because the
dependency was detected.

**What the AI sees at session start (P1.8):**

The SessionStart hook already injects rules + patterns + memories (P1.1). P1.8
adds a library block:

```
<sensei-libraries>
Project dependencies with docs available:
- rokkit (v2.1.0): component library for Svelte 5. Use `get_lib_docs("rokkit", "<component>")` for API details.
  Skills: rokkit-components, rokkit-styling, semantic-styles-rokkit
- drizzle-orm (v3.2.0): TypeScript ORM. Use `search_lib_docs("drizzle")` for schema/query patterns.
- supabase (v2.45): client library. Use `get_lib_docs("supabase")` for API details.
</sensei-libraries>
```

The model now knows what libraries are available and how to get their docs —
without having to discover them itself.

### 3.4 Dependencies

P1.1 is the foundation — it proves push works. P1.2-P1.5 can be parallelized
after P1.1 ships. P1.6 is the exit gate — it validates the entire phase.

P1.7 depends on P1.1 (the session-start injection mechanism) and is a
prerequisite for P1.8 (auto-inject library summaries). P1.2-P1.5 and P1.7
can be built in parallel.

P1.8 depends on P1.7 (docs must be indexed before they can be injected).

No Dōjō or Relay dependency. No new tables — rides `project_libraries`,
`library_pages`, `library_skills`, `library_agents`. No new API surfaces —
rides the existing `add_library` → `IndexLibrary` pipeline and MCP tools.

---

## 4. Phase 2 — "Dōjō as the join surface"

**Theme:** the minimal team layer. Dōjō is architecturally sound (pull-never-push,
membership-scoped, k-anonymized) but the join flow is broken and the console
surfaces 403 for every member. Phase 2 makes Dōjō *usable* by fixing the join
and wiring the minimal knowledge-sharing surface.

### 4.1 What exists today

- [`design/governance.md`](governance.md) — scopes, rules, promotion, identity
- [`design/dojo-web.md`](dojo-web.md) — Dōjō Worker architecture
- [`docs/architecture/dojo.md`](../architecture/dojo.md) — principles, membership types
- [`docs/architecture/dojo-deployment.md`](../architecture/dojo-deployment.md) — CF Worker + Supabase
- Dōjō console: 113 Svelte components in `dojo/src/`
- Federation backend: `/v1` routes ported from the removed `dojo-mind` Rust service
- Dead "Join" button: magic-link creates `auth.users` but no membership

### 4.2 Stories

| ID | Title | Status | Acceptance criteria | Refs |
|----|-------|--------|-------------------|------|
| P2.1 | Fix join/membership flow | build | A new user signs up via magic-link → membership row created → user sees their Dōjō console (not 403) | `governance.md` §identity/membership; `dojo/src/lib/server/dojo-auth.ts` (`resolveTenantAccess`); `docs/design/2026-07-27-dojo-relay-rls-membership-function.md` |
| P2.2 | Shared rules visible to members | build | After join, the member sees the Dōjō's shared rules (org-scoped mandatory + recommended rules) in their console; rules resolve correctly per the specificity ladder | `governance.md` §scopes/precedence; `design/remote.md` §governance |
| P2.3 | Shared memories visible to members | build | After join, the member sees k-anonymized memories contributed by other members; triage status (proposed / approved / declined) is visible | `governance.md` §promotion/triage; `dojo/src/routes/v1/` triage routes |
| P2.4 | Admin approve/decline shared knowledge | build | Admin console shows a triage queue; admin can approve or decline; approved items appear for all members; declined items are hidden | `governance.md` §promotion lifecycle; `dojo/src/routes/v1/triage/` |
| P2.5 | Dōjō connect from desktop app | build | The desktop app's Settings surface has a working Dōjō connect flow; user enters a Dōjō URL → validates membership → the app shows the Dōjō's shared knowledge | `design/projects.md`; `app/src/routes/(config)/` settings surface |
| P2.6 | Verify: second person joins | verify-only | A real second user signs up, joins the Dōjō, and sees shared rules + memories; no cross-org data leakage | Playwright on `dojo/`; DB check on `dojo.memberships` |

### 4.3 Dependencies

P2.1 is the blocker — everything 403s without it. P2.2-P2.5 can be parallelized
after P2.1. P2.6 is the exit gate.

No Relay dependency. No new API surfaces beyond what's ported from `dojo-mind`.
The Dōjō Worker and Supabase stack must be running (local dev via
`supabase/` or deployed to CF).

---

## 5. Phase 3 — "Relay for one"

**Theme:** personal supervision. P0-P4 are complete on `develop`. P5
(multi-assistant adapters) is the gap. Phase 3 ships the *personal* relay —
single-user, no team coordination — so long runs can be supervised from
anywhere.

### 5.1 What exists today

- Relay P0-P4 shipped on `develop` (not merged to `main`):
  - P0: contract + schema (`dojo-protocol/src/relay.rs`)
  - P1: vertical slice proven E2E (`scripts/relay-roundtrip.sh`)
  - P2: hook triggers + segment feed + PR-review send + phone UI
  - P3: daemon run engine (tick, agent-spawn, limit-pause, watchdog, crash recovery)
  - P4: away-from-keyboard (Web Push, RLS, realtime, "what's blocked on me" home)
- `SENSEI_RUN_DRIVE` is OFF by default (deliberate activation gate)
- Single driver: `relay_drivers/claude.rs` (`ClaudeDriver`); P5 stubs exist for ACP/fallback

### 5.2 Stories

| ID | Title | Status | Acceptance criteria | Refs |
|----|-------|--------|-------------------|------|
| P3.1 | Multi-assistant adapters (P5) | build | Relay works with Claude Code + at least one other assistant (Codex / OpenCode / Aider) via ACP or a fallback adapter; the run engine drives whichever adapter the user selects | `design/remote.md` §run engine; `relay_drivers/trait_def.rs` (`RunDriver`); `plan/2026-07-18-relay-p5-multi-assistant.md` |
| P3.2 | Activate drive for a supervised pilot | build | `SENSEI_RUN_DRIVE` is turned on for a real run; the daemon drives an agent through a plan; gates are raised and answered from the phone; the run survives a laptop sleep | `relay-engine.md` §5; `design/remote.md` §gates+nudges; `run_limits.rs`; `run_watchdog.rs` |
| P3.3 | `respond_gate` local reply channel | build | A raised gate can be answered from the desktop app (not just the phone); the `await_reply` loop checks both the Dōjō inbox and a local in-daemon reply store | `relay-engine.md` §control channel; `design/remote.md` §hooks; `decisions.md` deferred items |
| P3.4 | Prod deploy of P0-P4 | deploy | VAPID prod keys, prod `PUBLIC_SUPABASE_ANON_KEY`, dojo schema deployed, `supabase_realtime` publication migration, RLS validated; push notifications work for real users | `backlog.md` P4 deferred items; `decisions.md` relay deploy notes |
| P3.5 | Verify: long run from phone | verify-only | Start a run, close the laptop, the run completes (or gates); phone shows progress via realtime; gate reply resumes the run | Playwright on `dojo/src/` relay components; `scripts/relay-roundtrip.sh` extended |

### 5.3 Dependencies

P3.1 builds on the existing P0-P4 codebase. P3.2 requires P3.1 (the adapter
layer). P3.3 is independent (can be done in parallel with P3.1). P3.4 is
independent (infra, not code). P3.5 is the exit gate.

No Dōjō team-layer dependency beyond what P2 already provides. The personal
relay works with a single membership.

---

## 6. Phase 4 — "Dōjō as the governance plane"

**Theme:** the team flywheel. This is where the network effect lives:
knowledge from project A improves project B. Requires Phases 2 (members can
join) and 3 (runs can be supervised) to have users first.

### 6.1 What exists today

- Memory promotion ladder: `active → battle_tested` (DB trigger in `memory_outcome_apply`)
- Recommendation→verdict→reinforcement path (G1/G2 closed)
- Rule promotion: `battle_tested` memory elevated to higher scope via `promote_memory`
- Consolidation: Tier-2 LLM merge of rules (`analysis/rule_consolidation.rs`)
- Collective intelligence: `contribute_scheduler` is a no-op until opt-in; `dojo_inbox`/`dojo_outbox` at 0 rows

### 6.2 Stories

| ID | Title | Status | Acceptance criteria | Refs |
|----|-------|--------|-------------------|------|
| P4.1 | Rule promotion via FTR evidence | build | A memory promoted to `battle_tested` in project A is auto-suggested for promotion to the Dōjō scope; admin approves → it becomes an org rule; the rule resolves for other projects | `governance.md` §promotion; `playbook.rs` §learn; `design/instruction-delivery-model.md` §enforcement tier |
| P4.2 | Cross-project memory sharing | build | Proven memories from project A surface (with attribution) in project B's context when the projects share a Dōjō membership; k-anonymity enforced | `governance.md` §collective intelligence; `dojo/src/routes/v1/` federation routes |
| P4.2b | Shared library capabilities across Dōjō | build | Library-provided skills/agents (auto-indexed via P1.7) are visible across the Dōjō membership; a team can see "project A uses rokkit-components skill, project B should too"; the Dōjō becomes the shared library-capability catalog | `library-auto-discovery.md`; `governance.md` §collective intelligence; `library_skills` / `library_agents` tables |
| P4.3 | Team relay (P6) | build | Org gates fan into one shared queue; each decision carries attribution; on-call routing works | `relay-engine.md` §team relay; `objectives.md` R8; `design/remote.md` |
| P4.4 | DORA metrics (when deploy signals exist) | build | Deploy/release signals are captured (from git tags or CI webhooks); DORA Four Keys are computed and surfaced alongside FTR; AI-pairing patterns are correlated with DORA movement | `spec/governance/default-constitution.md`; `backlog.md` DORA module |
| P4.5 | Default governance bundle | build | A fresh project inherits a curated starter constitution (DORA + XP/CD + Core Protocols) as default rules; the user can dismiss/modify, but the baseline is not empty | `spec/governance/default-constitution.md`; `governance.md` §default gate line |
| P4.6 | Verify: cross-project improvement | verify-only | A pattern from project A is shared via Dōjō → project B picks it up → project B's FTR improves (or honest null if too few sessions to measure) | `verdicts.rs`; `design/playbook.md` §learning loop; `plan/README.md` G1-G2 |

### 6.3 Dependencies

P4.1-P4.2 depend on Phase 2 (members + shared knowledge). P4.3 depends on
Phase 3 (relay is working). P4.4 depends on a deploy/release-signal detector
(a prerequisite, not built yet). P4.5 is independent. P4.6 is the exit gate
— the moment the flywheel spins.

---

## 7. What this phases intentionally excludes

The operating model describes capabilities that are **not** in these four
phases. They are real, they are valuable, but they are *downstream* of the
flywheel spinning — they should ship after the core loop proves daily value.

| Capability | Why deferred | When to revisit |
|------------|-------------|----------------|
| **Planner** (FR/NFR → features → releases) | The playbook recommender exists but the full Planner is unbuilt; it needs daily-use data to know *what* to plan | After Phase 4 proves FTR improvement; the Planner needs real playbook→outcome data to be useful |
| **Brownfield onboarding** (reverse-engineer the spine) | Mostly an orchestration of existing capabilities (index, graph, drift, reverse-engineering); needs the core loop to be trusted first | After Phase 1 proves context pushes work; brownfield is "reconstruct the spine from code" — the spine must be trusted to exist |
| **Design/mockup subsystem** (brief → variations → review → handoff) | Needs the design system catalog to be well-populated and the component library to be mature | After the app's UI stabilizes; the generator needs a reliable component catalog |
| **Non-code projects** | Validating but not the primary market; the spine and playbooks are domain-agnostic but adapters need real non-code projects to test against | After the code-assistant use case proves the model works |
| **Chat-based user guidance** | The MCP surface already provides the data; a chat surface is UX polish, not a capability gap | After Phase 1 proves the context is valuable; chat is the human interface to what already exists |
| **Planner→Builder→Judge orchestrator** (P6 in the old phases) | This is the full autonomous loop; it needs the relay, the planner, and the governance plane all working first | After Phase 4; this is the endgame, not the wedge |

---

## 8. How this relates to existing docs

| Existing doc | Relationship |
|-------------|-------------|
| `plan/README.md` | The ranked-gap analysis (G1-G10). Phase 1 closes the adoption gap (the operating model's core finding). Phases 2-4 close the Dōjō/Relay gaps (G10 and beyond). |
| `plan/2026-07-20-phases-1-3-plan.md` | The tactical plan with mock gaps, verify stories, and bug lists. This doc is the strategic layer above it; that doc is the execution layer within each phase. |
| `design/instruction-delivery-model.md` | The delivery surfaces Phase 1 works with. P1.1-P1.5 implement the "push + re-assert" model described there. |
| `design/playbook.md` | The playbook recommender and learning loop. Phase 1 validates the playbook→outcome attribution (P1.4, P1.6). Phase 4 extends it across teams (P4.1). |
| `design/remote.md` | The relay run engine and control channel. Phase 3 ships the personal relay; Phase 4 adds team relay (P4.3). |
| `design/governance.md` | The governance plane. Phase 2 makes it joinable; Phase 4 makes it the team flywheel. |
| `plan/relay-engine.md` | The full relay spec (P0-P6). Phase 3 covers P5 (multi-assistant) and prod deploy; P6 (team relay) is Phase 4. |
| `spec/2026-08-18-repo-grain-metrics-watermark-engine.md` | The metrics rebuild. P-A/P-B (local, repo-grain, user-attributed quality) are v1 (§2.1); P-C (Dōjō sync, me-vs-team, enrollment D16) rides Phase 2. Supersedes the assumed-metrics behind P1.6. |
| `library-auto-discovery.md` | The Phase-1 P1.7/P1.8 module — detected dep → auto-index docs/skills → session-start inject. Nearly built; a v1 stream. |
| `operating-model.md` | The v2 vision. This doc is the incremental path *toward* that vision, not a replacement. The operating model describes the end state; these phases describe the sequence to get there. |
| `objectives.md` | Each phase's exit criterion maps to one or more objectives: Phase 1 → O5 (module loops offer action); Phase 2 → DJ1-DJ5 (Dōjō layer); Phase 3 → R1-R7 (Relay); Phase 4 → O3 (trust through proof) + DJ2-DJ5 (boundary/attribution). |

---

## 9. Open decisions

These are the decisions this phasing depends on, to be resolved before each
phase starts:

| Decision | Phase | Status | Detail |
|----------|-------|--------|--------|
| Auto-inject scope: how many memories/patterns/rules to push at session start? | 1 | open | Too many = context pollution; too few = missed value. Needs experimentation with real sessions. Start with top-3 each, measure FTR delta. |
| PreToolUse context injection: which file operations trigger it? | 1 | open | `read_file` + `write_file` + `grep` are the obvious ones; `list_files` may be noise. Define the allow-list. |
| Dōjō join flow: magic-link only, or also OAuth (GitHub/Google)? | 2 | open | Magic-link is simplest; OAuth is expected by teams. Decision affects the auth surface complexity. |
| Multi-assistant adapter priority: which assistant to support after Claude? | 3 | open | Codex, OpenCode, and Aider all have different control surfaces. Pick one to prove the adapter layer works. |
| Deploy/release signal source: git tags vs CI webhooks vs both? | 4 | open | DORA metrics need a deploy signal. Git tags are simplest but miss CI/CD; webhooks are complete but need integration. |
| Default governance bundle content: what ships as the starter rules? | 4 | open | `spec/governance/default-constitution.md` has a draft. Needs review before shipping as the default. |
