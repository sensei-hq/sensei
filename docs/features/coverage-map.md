---
name: Feature Coverage Map
description: Built-vs-pending map across mockups, journeys, docs, and code — the authoritative status snapshot
date: 2026-07-25
---

# Feature Coverage Map

Cross-references four sources — **mockups** (`docs/mockups/Sensei/lib/**`), **journeys** (`docs/journeys/*`), **docs** (`docs/features/*`, `docs/design/*`), and **code** (crates + `dojo/` + `app/` + `database/ddl/`) — into one built-vs-pending snapshot. This is the current-truth status; the per-feature docs' own status tables trail it (see §F).

**Legend** — ✅ Built (shipped + code evidence) · 🟡 Partial (backend or UI present, the other half or wiring missing) · 🖼️ Mock-only (mockup exists, no built surface) · 📄 Designed (doc/design only, no code) · ⛔ Gap (intended but neither designed nor built) · 💤 Dormant (built but behind an off-by-default flag / not merged to main).

Scale of the built surface: **~130 daemon routes · 37 MCP tools · 29 Dōjō `/v1` routes · 45 app screens · 7 dojo2 pages (~30 sections) · 123 DB tables across 8 schemas.**

---

## A · Sensei desktop app (`app/`)

The wired IA comes from `observatory.jsx`'s section router; app screens are `app/src/routes/(observatory|config|project|health)`.

| Surface | Mockup (wired) | App route | Status | Notes / gap |
|---|---|---|---|---|
| Health/bootstrap gate | `bootstrap-splash.jsx` | `(health)/health` | ✅ | Six-gate probe→auto-fix→green. |
| Setup (first-run) | `setup-wizard.jsx` (10-stage) | `(config)/setup/{welcome,roots,scan,assistants,done}` | ✅ | Deliberately **thinned to 5 stages** (value-before-setup); the wizard's inference/assignments/libraries/registry stages moved to Settings. |
| Today | `observatory-today.jsx` | `(observatory)/` | ✅ | Shared with the marketing site — keep in sync. |
| Intake (front door) | `intake.jsx` | `/intake` | ✅ | 3-axis classify → 6-playbook recommend → confirm; Start/Playbooks/History tabs. |
| Projects index | `navigation.jsx#ProjectsIndexA` | `/projects` | ✅ | Palette (B) + tree (C) variants unbuilt (by design). |
| Project window | `project-pages.jsx#SidebarD` | `(project)/project/[id]/{overview,about,sessions,impact,instruments,libraries,memories,patterns,traceability}` | ✅ | 11-section sidebar; Atlas is a top-level route. |
| Atlas (code graph) | `project-atlas.jsx` | `/atlas` | ✅ | SVG graph, 4 granularities, doc-drift squares. |
| Sessions | `sessions-zen.jsx` | `/sessions` | ✅ | Retro digest, 6 chart modes. |
| Logs | `project-logs.jsx` | `/activity-logs` | ✅ | Trace explorer + anonymized issue-report modal. |
| Insights | `learnings-v2.jsx#LearningsTriage` | `/insights` | ✅ | Now/Soon/Settled triage. |
| Memories | `learnings-anatomy-v2.jsx` | `/learnings` | ✅ | What/Why/How/Where anatomy. |
| Traceability | `traceability.jsx` | `/traceability` | ✅ | Doc↔code drift + fix. |
| Impact | `impact.jsx#ObsImpact` | `/impact` | ✅ | Before/after metric grid + MOE reasoning. |
| Instruments | `instruments-simple.jsx` | `/instruments` | ✅ | Playground/Replay/Health (MCP tools). |
| Libraries | `libraries.jsx#VariantA` | `/libraries` | ✅ | Doc-health + attached rules + MCP examples. |
| Upgrades (downstream) | `upgrades.jsx` | `/upgrades` | ✅ | Incoming gifts inbox. |
| Share/review | `share-hub.jsx` | `/share-review` | ✅ | Unified per-trust-boundary sharing. |
| Dōjō connections/sharing | `share-hub.jsx`, `collective-settings.jsx` | `/dojo/connections`, `/dojo/sharing` | ✅ | Membership + sharing governance. |
| Knowledge sources | (federation) | `/knowledge-sources` | ✅ | External doc/library sources. |
| Settings (Preferences) | `setup-wizard.jsx mode=preferences`, `inference-settings.jsx` | `/settings/general` + `/assistants,/inference,/extensions,/instruments,/libraries,/projects,/providers,/roots` | ✅ | The wizard-stage config, made a persistent editable surface. |
| **Impact-alert** (regression) | `impact.jsx#ObsNegativeAlert` | — | 🖼️ | Full-screen red-flag alert mocked, no route. |
| **Consolidation** | `consolidation.jsx` | — | 🟡 | Backend shipped (`consolidated_rulesets`, `/rules/consolidate`); **no app screen**. |
| **Benchmark** (A/B) | `benchmark.jsx` | — | 🖼️ | `benchmark_reports/runs` tables exist; no runner UI, no run wiring. |
| **Solution-track** (engagement rollup) | `solution-track.jsx` | — | 🖼️ | Multi-project client rollup on desktop; dōjō Clients zone partly overlaps. |
| **Agent/Persona editors** | `agent-persona-editors.jsx` | — (`/assistants` is thinner) | 🟡 | Autonomy-ceiling + tool-envelope + **replay-test-against-past-sessions** not built. |

---

## B · Dōjō web console (`dojo/`)

dojo2 (`(dojo2)/`) is the current rebuild; `(console)/` is legacy (cutover human-gated). The `[section]` loader wires live `/v1` data for some sections and renders fixtures for the rest.

| Zone · Section | dojo2 screen | `/v1` route | Wired? | Status | Gap |
|---|---|---|---|---|---|
| **Personal · Your work** | `ScrYourWork` | (aggregates) | fixture | 🟡 | Landing + needs-band; not live. |
| Personal · Live runs / Approve / Decide / Chat | `ScrRelayWatch/Approve/Decide/Chat` | `relay/{session,gates,inbox,reply}` | partial | 💤 | Relay backend shipped + P4 browser-verified; drive/gate flags OFF, not merged to main. |
| Personal · Constitution (stance) | `ScrConstitution` | `/api/stance` (daemon) | fixture | 🟡 | Stance resolves in the daemon; console reads the fixture. |
| Personal · Rule packs | `ScrRulePacks` | — | fixture | 🟡 | `dojo.rule_packs` DDL shipped; no `/v1` catalog route + no console wiring. |
| Personal · Contributions / My dōjōs / Project preview | `ScrContributions/ScrMyDojos/ScrProjectPreview` | `/api/dojo/memberships` (daemon) | fixture | 🟡 | Preview = the resolution ladder; not wired. |
| **Org · Home / Projects** | `ScrOrgHome/ScrProjects` | — | fixture | 🟡 | Org project jurisdiction; no route. |
| Org · Constitution/ladder (authoring) | `ScrOrgLadder` + `RuleEditor` | `/rules` (federation) | fixture | 🟡 | Rule authoring UI not wired to `/rules`. |
| **Govern · Triage / Approvals** | `ScrTriage/ScrApprovals` | `triage`, `triage/[sig]/decide` | ✅ | ✅ | Live (Tier-2). |
| Govern · Knowledge (catalog + prune) | `ScrKnowledge` | `artifacts` | fixture | 🟡 | Catalog lists agent/command/skill; not wired to `artifacts`; personas/hooks/plugins missing. |
| **Clients · Engagements / Incidents / Client-audit** | `ScrEngagements/ScrIncidents/ScrClientAudit` | `engagements`, `incidents`, `audit` | ✅ | ✅ | Live (Tier-2). |
| Clients · Confidentiality model | (panel in Engagements) | — | fixture | 🟡 | Kept/dropped + anonymization example illustrative. |
| **Admin · Members / Roles / Audit** | `ScrRoleSurfaces` | `members`, `policies`, `audit` | ✅ | ✅ | Live. Audit fixture shape is a placeholder. |
| Admin · Identity & SSO | `ScrIdentity` | `identities` | ✅ | ✅ | Live. |
| Admin · Health | `ScrHealth` | `health` | ✅ | ✅ | Live. |
| Admin · Plan & billing | `ScrBilling` | `billing` | ✅ (seats live) | 🟡 | Live seat count + plan/renewal; **tiers/invoices fixture until a payment provider (D-BILLING)**. |
| Admin · Scopes & ownership | `ScrScopes` | — | fixture | 🟡 | Scope-owner assignment; no route. |

**lib/dojo features NOT ported to dojo2** (feature regressions to decide on): stack reviewers / mechanical checkers (qlty·eslint·ruff·clippy) from `dojo-library.jsx`; extensions personas/hooks/plugins + org→team→project scoping + adoption tracking from `dojo-extensions.jsx`.

---

## C · Relay (away-from-keyboard)

| Piece | Status | Evidence |
|---|---|---|
| Contract + schema (relay tables/enums, `dojo_protocol::relay`) | ✅ | P0 |
| Vertical round-trip (daemon→Worker→phone→poll) | ✅ | P1 proven |
| Segment feed · PR-review · gate card · nudge | ✅ 💤 | P2; hook-gate OFF by default (`SENSEI_RELAY_GATE_TOOLS`) |
| Daemon run engine (plan-as-run, tick, limit→pause→resume, watchdog, MCP run-control) | ✅ 💤 | P3; agent drive OFF (`SENSEI_RUN_DRIVE`) |
| Web Push (PWA + VAPID) · "what's blocked on me" · offline queue · RLS · realtime | ✅ | P4, browser-verified live |
| Seat attribution (open/refresh seats from federation) | ✅ | This session (`41276793`+`6ca14319`) |
| Multi-assistant adapters (Codex/OpenCode/Aider/Zed via ACP) | 🟡 | P5 — Claude driver + fallback shipped; other backends pending |
| **Activation** (flip drive/gate on, merge relay→main) | 💤 | Human-gated |

---

## D · Backend engine + planes (the always-on loop)

| Capability | Status | Evidence |
|---|---|---|
| Capture (hooks, sessions, transcript turns, multi-assistant) | ✅ | `activity.*` (11 tables), `/hook/event`, adapters |
| Graph (code+activity, call edges, incremental watcher) | ✅ | `nodes/edges`, `/api/graph/*`, watcher |
| Analyze (enrich, signals, patterns, FTR) | ✅ | `inference.*`, analyzer schedulers |
| Learn (memories, patterns, guards, recommendations) | ✅ | knowledge plane (~26 routes), `sensei.memories` |
| Deliver context (MCP, first-try) | ✅ | 37 MCP tools, `context_pack`/`get_layered_context`/`get_rules` |
| Measure verdict (FTR before/after, outcomes) | ✅ | `impact_verdicts`, `/knowledge/outcomes` |
| Gateway (embedded Gemma, routing, MoE/consensus, image) | ✅ | `gateway.*` (6 tables), 17 gateway routes |
| Governance resolution (scopes/namespaces/ladder, Tier-1/2, injection) | ✅ | `/knowledge/{rules,constitution}`, `render_rules_tiers`, hooks push (D-INJECT) |
| Rule packs (adopt-at-scope, fold into resolution) | ✅ (local) | `dojo.rule_packs*`, daemon fold-in; prod apply gated |
| Stance (autonomy·sharing·review) + autonomy drive gate | ✅ | `sensei.stances`, `/api/stance`, `autonomy_permits` |
| Federation (pull `/v1/rules` → inbox → memories) | ✅ | `dojo_inbox/outbox`, `knowledge_sources` |
| Billing (per-tenant seats, private-project count) | ✅ (local) | `dojo.seats`, `billing_accounts`, `tenant_seat_usage`, `/v1/billing`; prod + provider gated |
| **Planner / tasks-plan** (decompose a plan into features+criteria) | ✅ (v0) | `crates/senseid/src/planner.rs` + `POST /api/planner/generate` + MCP `plan`: goal → gateway `reasoning` decomposition → structured phases→features→acceptance-criteria + rendered `docs/plan` md. Stateless (run engine is file-based); DB persistence + Tasks-tab wiring deferred to the project window. |
| **Local-agent coordinator** (heterogeneous execution router) | 📄 | `design/local-agent-coordinator.md` draft; prereq = gateway inference-usage ledger (none). |
| **Default governance bundle** (seed the starter constitution) | 📄 | `spec/governance/default-constitution.md` drafted; `seed_default_governance()` has 0 callers. |
| **DORA delivery module** | 📄 | Framed in spec/backlog; prereq = a deploy/release-signal detector (undocumented). |

---

## E · Designed-but-unbuilt & doc gaps

**Mock screens with no built surface** (§A/§B): impact-alert, benchmark runner, solution-track, agent/persona editors, and every fixture-only dōjō section (rule-packs catalog, knowledge, scopes, org ladder authoring, constitution/stance console read).

**Documented-not-built:** dōjō auto-discover (setup, 0% built, designed 3×); planner; default governance bundle; DORA module; billing payment provider; HF model support `[ ]`; brownfield onboarding `[ ]`; chat guidance (Phase 4, roadmap only).

**Doc stubs to fill:** `features/front-door/plan.md` (header only), `features/front-door/tests/README.md` (empty), `features/front-door/mockup-ref.md` (pending — no designer screens); `docs/personas/` (stub — personas synthesized, never authored). Governance/grounding screens (`features/governance/feature.md`) have no mockup-ref.

---

## F · Doc/code lag (accuracy debt)

The feature docs are evidence-dated **2026-07-14/20**; a large build wave landed **2026-07-24/25**. So several docs say "not built" for things now shipped. Docs to refresh against this map:

- `features/05-governance.md` — rule packs, injection (D-INJECT), stance, constitution endpoint now shipped.
- `features/06-relay.md` — P0–P4 + seat attribution shipped (flags off).
- `features/README.md` capability map — several `[~]`/`[ ]` are now `[x]` (rules resolution, injection, stance, billing).
- `design/instruction-delivery-model.md` — the rule-pack author/catalog it calls "proposed" now has DDL + daemon fold-in.
- `spec/` (~90 draft docs) — wholesale transitional; folding into `features/*` + `design/*`. Drift risk with the newer canon (e.g. front-door dossier vs `design/playbook.md` describe the same thing twice).

---

## G · Open questions

**Resolved 2026-07-25** (see [`../decisions.md`](../decisions.md) → Coverage-audit scope calls):
- **Planner** → build a real one (D-PLANNER). · **Desktop screens** → build Consolidation + Agent/Persona editors + Solution-track; **benchmark cut** (D-SCREENS). · **Rule verification** → build checker execution (D-CHECKER). · **Default governance** → ship the seed (D-SEED confirmed).

**Still open:**
1. **Mock hygiene** — prune the superseded-but-not-`discarded/` variants (sharing-review, mcp-replay-insights, multi-option learnings/project-pages/libraries/nav)?
2. **Extension governance** — port the richer `dojo-extensions.jsx` model (personas/hooks/plugins + org→team→project scoping + adoption tracking) into dojo2, or keep the thin catalog?
3. **Fixture-only dōjō sections** — rule-packs catalog, knowledge, scopes, org-ladder authoring: wiring order (post-cutover)?
4. **Gated activations** — relay drive/gate, prod DDL, billing provider, dojo2 cutover, main-merge: sequencing.
5. **Personas** — author the real persona records (`docs/personas/` is a stub)?
6. **Impact-alert** screen — build the regression full-screen, or fold into the Impact screen?
