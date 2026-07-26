---
name: Decisions log — sensei
updated: 2026-07-20
---

# Decisions

> Append-only. One entry per decision: date · decision · why · alternatives.
> The anti-rework memory — never re-derive a settled choice.
> The **design journey** (approaches tried, what worked, what didn't) lives in
> [`analysis/2026-07-24-design-journey.md`](analysis/2026-07-24-design-journey.md).

---

## 2026-07-24 — Auto-buildout decisions locked (P0–P5)

Locked with Jerry ahead of the phased unattended build-out. Full context:
[`analysis/2026-07-24-auto-buildout-readiness.md`](analysis/2026-07-24-auto-buildout-readiness.md) §4.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **Relay-first** | Build **relay supervision first** (P1): publish the plan as a relay run (phases→segments), daemon federates status → Dōjō, Jerry watches as `jerrythomas` + nudges; **no update in 5 min = stall**. Relay **STATUS** (safe), distinct from relay **DRIVE** (off). | Makes the whole unattended run watchable/nudgeable — solves "am I stuck?" without pings. Rejected: task-list-only visibility (too coarse for a long run). |
| **D-INJECT** | Governance rules are **pushed**, not pulled: resolve the repo's rules → inject into `additionalContext` at **SessionStart**, re-inject the **mandatory tier at PreCompact**, and inject into the **relay driver** for daemon runs. Mandatory/required always; advisory on-demand. | Root cause of "not sticky" = pull-only (`get_rules` reminder). Rejected: nudge-only (status quo, doesn't stick); SessionStart-only (drops after compaction). |
| **D-RULEPACK** | A rule pack is a **new `dojo.rule_packs` table + pack↔rule join** (area/scope/source/enforcement first-class). | Clean provenance + adopt-at-scope + maps to the designer's original LIB_PACKS model. Rejected: tag/collection on `shared_rules` (weaker metadata). |
| **D-COORD** | Coordinator **v1 = task-typed router at `driver_for` + gateway single-shot leg for cheap types + attribution ledger, drive OFF.** | Smallest useful superpowers-mirror. Rejected (v1): local ACP coding-agent + Planner→Builder→Judge loop (bigger, riskier → follow-up). |
| **D-GATE** | Blocking PreToolUse hook-gate stays **OFF** (activation = Phase-5 human gate). | Enforcement on live sessions is irreversible-ish; Jerry-gated. |
| **D-DRIVE** | `SENSEI_RUN_DRIVE` stays **OFF** the whole unattended run. | Creating/publishing runs is safe; driving a live agent is a human gate. |
| **D-SEED** | Invoke `dojo.seed_default_governance()` on the **Worker/Supabase** side + auto-register the global-dōjō pull `knowledge_source` on install. Prod-apply **gated**. | The seed has zero callers today ("reaches nobody"). |
| **D-ORIGIN** | Canonical provenance vocab = `authored | promoted | federated`; retire `remote`. | Doc/code mismatch (`remote` vs `federated`). |
| **D-STANCE-SCOPE** | Personal **stance** store is **user-scoped, not tenant-scoped**. | Personal governance follows the user with or without a dōjō. |
| **D-TIER3-DDL** | Tier-3 tables full-DDL-no-ALTER under `database/ddl/table/dojo/*.ddl`, `dbd deploy` against **e2e/local**; shapes proposed + confirmed as authored. Prod apply **gated**. | dbd is the schema tool; never Cloudflare-D1 (that's Decision-1). |
| **D-BILLING** | Billing = **schema + route only**; payment provider out of the unattended run until chosen. | No provider decided. |
| **D-CUTOVER** | `/console`→dojo2 redirect, `develop`→`main` merge, prod deploy, prod seed — **all human-gated (Phase 5)**. | Irreversible / trust-boundary. |
| **D-DECISIONS-HOME** | This file is canonical; per-feature → `features/<name>/decisions.md`. | The empty canonical slot; ADRs were misfiled. |

**Unattended boundary (standing):** safe-unattended = deterministic, local, reversible,
verifiable per chunk (DDL edits, Rust/TS on `develop`, tests, local browser-verify, dbd on
e2e/local, commits/pushes to `develop`). Human-gated = cloud/prod DB writes, Worker prod
deploy, secrets/VAPID/service-role, merge→main, cutover, gate activation, flipping
`SENSEI_RUN_DRIVE`, any payment-provider choice. The loop prepares + stages gated work,
writes the evidence, and waits.

---

## 2026-07-25 — Coverage-audit scope calls

Resolved during the mockup/journey/doc/code coverage audit (see
[`features/coverage-map.md`](features/coverage-map.md)). These set what gets built next.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **D-PLANNER** | **Build a real planner** — takes a goal / spec / issue and generates a structured plan (phases → features → acceptance criteria) that the plan-as-run engine, the project-window **Tasks** tab, and the `sensei-plan-depth-reviewer` gate consume. | The one missing spine: "project needs the planner" + plan-as-run wants phases→features→gates, but plans are hand-authored `docs/plan/*.md` today. Biggest single unlock. Rejected: keep plans human-authored only (no autonomous decomposition). |
| **D-SCREENS** | Build the **Consolidation** screen (backend already shipped — just the app route), the **Agent/Persona editors** (autonomy ceiling + tool envelope + **replay-test against past sessions**), and **Solution-track** (desktop multi-project engagement rollup — a distinct solo-consultant surface, kept alongside the dōjō Clients zone). **Benchmark runner CUT** for now. | Designed-but-unbuilt with real user value. Benchmark (A/B sensei-vs-no-sensei) is a research/marketing artifact, not a near-term product surface — deferred. |
| **D-CHECKER** | **Build checker execution** — a rule with a `checker_ref` runs its checker (eslint / ruff / clippy / test) and yields a **pass/fail verdict**. Rules become enforceable, not advisory-only; ties to a future CI/deploy (DORA) signal. | `rule_pack_rules.checker_ref`/`verification` exist but nothing runs them; the old `dojo-library.jsx` wired stack reviewers, dropped in dojo2. Rejected: advisory-only. |
| **D-SEED (SHIPPED)** | **Default governance bundle shipped as BUNDLED LOCAL PACKS** (via D-LOCAL-PACKS, not `dojo.shared_rules`). Retired the zero-caller `dojo.seed_default_governance`; replaced by `sensei.seed_default_constitution()` — four global-library packs by `rule_pack_area`: `default-principles` (4, mandatory), `default-architecture` (5), `default-process` (12), all **auto-adopted at the general `global-dojo` namespace** so a fresh install resolves them **offline** through `get_rules`; `stack-templates` (9, Rust/TS/Svelte/Python) **seeded but not adopted** (opt-in per stack). Wired into `bootstrap::database::deploy` (`seed_bundled_packs` CALLs constitution + ponytail after every deploy, idempotent, **fail-open**), so install/upgrade seeds it on both planes (local `sensei` + Supabase). Content = `spec/governance/default-constitution.md`, verbatim. Verified: DB-backed test (adopts 3 packs, 21 rules resolve, stack excluded, idempotent) + live `get_rules` offline. Prod Worker apply stays gated (D-CUTOVER). | A new project starting empty undercuts "grounding that sticks". Packs (not shared_rules) keep ONE source, deploy to both planes, and resolve with zero dōjō. |
| **D-PACK-KIND** | The **ponytail** coding discipline (and coding-convention libraries generally) = a **rule pack** (`dojo.rule_packs`, `area=principles`), **not** a new "skill pack" type. Rules are conventions — injected (D-INJECT), resolved on the ladder, verifiable (D-CHECKER). A "skill pack" (bundle of *callable* skills) is the marketplace `extensions`; a rule references a skill via `rule_pack_rules.skill_ref`. Shipped: `dojo.seed_ponytail_pack()` — a global-library pack (6 rules), seeded local Supabase (idempotent), resolution + never-weaken override proven; **prod apply + boot-call gated**. Spec: [`spec/governance/ponytail-pack.md`](spec/governance/ponytail-pack.md). | Don't overload rule_packs into skill_packs — the two compose via `skill_ref`. Ponytail's *satellite* skills (`-audit`/`-review`) would be marketplace extensions, separate from the discipline-as-rules. |
| **D-LOCAL-PACKS** | **Rule packs live in BOTH planes, in tandem.** The daemon keeps a **local replica** of the pack tables (`dojo.rule_pack*` deployed to the local `sensei` DB via the `default` scope AND to Supabase via the `dojo` scope — one definition, two planes; the **same seed procedures seed both**, so no content copy). Curated packs (default constitution, ponytail, user-authored) resolve **offline alongside memories** — never poured INTO `sensei.memories` (memories = *learned* knowledge; packs = *curated*). Works with **zero dōjō** ("just another shared mechanism"); **when a user runs a dōjō the two sync**. Resolution = memories + local adopted packs + the live dōjō fold-in, in tandem. | Keeps curated rules out of the learned-memory store; offline-first; no vendor lock-in. **dbd resolution (Jerry):** NOT a dbd fix — `includes` is authoritative-by-design (`scope.rs::resolve`: non-empty includes = base, then excludes subtract → `includes:[dojo.rule_packs]`+`excludes:[dojo]` = ∅; making includes additive would break `includes:[app]`="only app"). Idiom = **put shared objects in a shared schema.** So **move the pack objects `dojo.* → sensei.*`** (enums `rule_pack_area`/`rule_check`, tables `rule_packs`/`rule_pack_rules`/`rule_pack_adoptions`, proc `seed_ponytail_pack`): `default` scope gets them free (local), `dojo` scope includes them explicitly (Supabase). REFACTOR (atomic): move DDL + `design.yaml` dojo-scope includes + daemon `resolve_rules_raw` reads LOCAL `sensei.rule_packs` (the DojoClient fold-in becomes the sync) + Worker `rules-data.ts` `sensei.rule_packs` + re-reconcile both DBs + re-seed ponytail + drop orphan `dojo.rule_pack*` on supabase. Verify get_rules offline. |
| **D-EXEC-TEAM** | **Design now, build later:** a sensei-owned **agentic execution team** — planner → coordinator → model-chain workers (coder · reviewers · tools · persona-tester) — that plans + executes + quality-gates feature work **as monitored relay runs**, so it's model-agnostic and controllable (not Claude-only). Full design: [`design/agentic-execution-team.md`](design/agentic-execution-team.md). Resolved sub-calls: spec-fidelity = rendered-DOM check + **React-mock reuse-not-rewrite**; **worktree per feature**; the **quality gate is authored into the spec by the skills**; **build the inference-usage ledger first**; the **planner self-reviews** for completeness/coverage; runs **drive-OFF**. Build v0(ledger)→v1(skinny slice)→v2(team+routing)→v3(parallel+consensus). | Controllers own their loop → can't gate/route/enforce mocks (torii+seiki: mocks ignored, tests missed it). Owning the orchestration = control + no vendor lock-in. Deferred build (Jerry): want a robust base first. |

**Still open (mock hygiene / scope):** prune the superseded-but-not-in-`discarded/` mock
variants (sharing-review, mcp-replay-insights, the multi-option learnings/project-pages/
libraries/nav files); decide whether dojo2 ports the richer **extension governance**
(personas/hooks/plugins + org→team→project scoping + adoption tracking) from
`dojo-extensions.jsx`.
