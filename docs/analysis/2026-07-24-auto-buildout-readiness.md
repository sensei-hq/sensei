---
title: Auto-Buildout Readiness — dojo / governance / coordinator
date: 2026-07-24
status: assessment (input to an unattended build-out decision)
scope: what must be TRUE for an unattended (no "continue/yes" per turn) build-out of the remaining dojo + governance + coordinator work; the phased build sequence; per-phase verification gates; and the decisions that must be locked before the run starts
inputs: six read-only as-built maps (governance-instruction, skills-agents-stickiness, local-models-gateway, orchestration-execution, docs-framework-state, built-vs-premise)
branch-state: develop is 117 commits ahead of main; main at v0.6.0 (126cd9d6). Cutover to main is Jerry-gated and NOT done.
---

# Auto-Buildout Readiness Assessment

## 0. What "unattended" means here

The goal: I drive the remaining dojo / governance / coordinator work to completion without a human "continue / yes please" every turn. Unattended does **not** mean unsupervised of *everything* — it means **the loop runs itself through the safe majority of the work and stops only at a small, pre-declared set of human gates** (cloud DB writes, prod deploy, secrets, irreversible cutover, and net-new product decisions). The distinction this doc makes throughout is:

- **Safe-unattended** — deterministic, local, reversible, verifiable per chunk (DDL edits, Rust/TS on `develop`, tests, local browser-verify, staging import into a local/e2e DB, commits/pushes to `develop`).
- **Human-gated** — irreversible or trust-boundary-crossing (cloud/prod Supabase writes, Worker prod deploy, secret/VAPID/service-role handling, merge `develop`→`main`, `/console`→dojo2 cutover, activating the blocking hook-gate, any payment-provider choice).

Autonomy is earned per chunk by clarity + verifiability, not granted globally.

---

## 1. What must be TRUE before an unattended run (readiness preconditions)

An unattended run is only safe when **every remaining chunk has full clarity, a locked decision, and a per-chunk verification gate that I can run without a human**. Concretely:

### 1.1 Docs must be unambiguous on the traps that silently mislead a builder
The as-built maps surfaced five premise-vs-code traps that would derail an unattended builder. **These must be corrected in-doc (or explicitly noted in the run brief) before the run**, because an autonomous loop trusts docs:

1. **"D1" is Decision-1, not Cloudflare D1.** The dojo `/v1` backend is **Supabase Postgres** (service-role, `dojo` schema, `dojo-supabase.ts`), not Cloudflare D1. Commit messages and the retire-dojo-mind plan say "D1/D2/D3" meaning Decision-1/2/3. An unattended builder will otherwise author D1 bindings/SQL. **Tier-3 tables = `database/ddl/table/dojo/*.ddl` via `dbd deploy` + staging import — never D1 migrations.**
2. **`daemon/` path in CLAUDE.md is stale.** The daemon is `crates/senseid`; DDL is `database/ddl/` at repo root (per-object files, no combined `.sql`). A builder that `cd daemon/` fails.
3. **Fixtures render as "working."** Every Tier-3 surface (`billing`, `scopes`, `projects`, `stance`, `ladder`, `constitution`, `rulePacks`) renders full, real-looking data from `fixtures.ts` through the *same return shape* as real loaders. A browser-verify sees green and concludes "done." **The only truth signal is the inline `// Tier 3, needs DDL` comments and the absence of a `dojo.<table>` + a `/v1` route — trust those, not the screen.**
4. **Governance is resolve-only; there is no author path.** Rules only enter via daemon federation-publish. The dojo has **no rule-authoring write-route**, and the daemon **does not inject** resolved rules into a relay-driven agent (no `context_pack` in `relay_drivers/claude.rs`). "Governance authoring" is build-all-three (table + route + injection target), and the stance store is **user-scoped, not tenant-scoped** (easy to get wrong).
5. **Two IAs coexist, no cutover.** `(console)` and `(dojo2)` both live on `develop`; nothing redirects old→new. Editing the legacy `(console)` group is a real failure mode.

### 1.2 Decisions must be locked (see §4) — no "figure it out mid-run"
The project rule is "do not start implementing if you don't have clarity, ask for clarifications." Under unattended execution I cannot ask. So **every decision in §4 must be locked before the run**, or the corresponding chunk is out of scope for the unattended run and stays behind a gate.

### 1.3 Every chunk must be verifiable by me, locally, without a human
The per-chunk gate must be runnable by me: `cargo test` / `cargo clippy` (zero-errors-policy), `dbd deploy`/`apply` against a **local or e2e** DB, `bun`/vitest for dojo, and local `wrangler`+Playwright browser-verify. If a chunk's only real verification is against cloud/prod data, it is **not** safe-unattended — it stops at a gate.

### 1.4 The tooling must be confirmed live (not assumed blocked)
Per prior lessons: the e2e harness works (`make test-app-e2e`), Docker runs, Supabase is installed. Confirm these at run start; do not park work assuming a tool is unavailable.

### 1.5 A clone/e2e DB must exist so DDL + import are exercised off prod
Tier-3 DDL and staging-import must be validated against `sensei_e2e` (or a local dojo Supabase), never prod. The seeding procedures already have timestamp-based guards, but the unattended rule is stronger: **the loop never writes cloud/prod DB.**

### 1.6 The decisions log must have a home before the run generates more decisions
Doc-system defect: `docs/decisions.md` (canonical slot) is empty; the real ADR log is misfiled at `docs/plan/decisions.md`. An unattended run *produces* decisions. Decide the canonical home (recommend: reconcile onto `docs/decisions.md`, per-feature into `features/<name>/decisions.md`) so run-time decisions land instead of evaporating.

---

## 2. Phased build sequence

Phases are ordered so each unlocks the next and so **all cloud/prod/irreversible steps cluster at named gates**. Everything inside a phase that isn't marked **[HUMAN GATE]** is safe-unattended.

### Phase 0 — Run brief + doc-trap correction (safe-unattended)
- Correct the five §1.1 traps in the relevant docs (or capture them in a run brief the loop reads first): D1≠Cloudflare-D1, `daemon/`→`crates/senseid`+`database/ddl/`, fixtures-are-not-done, governance-is-resolve-only, two-IAs-no-cutover.
- Reconcile the decisions-log home (§1.6).
- Confirm tooling live (§1.4) and clone the e2e DB (§1.5).
- **Gate:** docs build/lint clean; e2e DB reachable.

### Phase 1 — Rule-pack shape + injection hook (governance stickiness, mostly safe-unattended)
The core weakness across the maps: **scoped/mandatory rules are pull-only and never pushed into a relay-driven agent's context.** This phase makes governance *reach the work.*
1. **Rule-pack shape (data + resolve side).** Define what a rule-pack is: a curated bundle of `shared_rules` (registry rows keyed on `namespace_id, content_hash`). Decide whether packs are a new `dojo.rule_packs` table (Tier-3, Phase 2 DDL) plus a pack↔rule join, or a tag/collection on existing rows. Resolve-side (`governance.rs` `structure_ruleset` / `resolve_rules_raw`) already dedups + mandatory-locks; extend to carry pack membership.
2. **Injection hook (the push channel).** Two sub-targets, both real gaps:
   - **SessionStart hook** currently injects only `user`+`general` from `~/.sensei/rules.md` + a *reminder* to call `get_rules`. Upgrade it to resolve the CWD repo and inject the per-repo `get_rules` result (`resolve_rules_raw(folder_id)` output) into `additionalContext` — not just a nudge. (`marketplace/plugins/sensei/hooks/session-start`, `knowledge.rs:155`.)
   - **PreCompact re-inject** currently drops global rules/personas — upgrade `hooks/pre-compact` to re-inject the **full** mandatory tier, not a summary.
   - **Relay-driver context-pack.** If "push not pull" is to be literally true for daemon-driven runs, add a driver-side context-pack inject path (`relay_drivers/claude.rs` has none today). This is the "coordinator injects governance" seam. **[Depends on decision D-INJECT in §4 — the *policy* of push-vs-nudge and whether the blocking gate is in scope.]**
3. **Seed + subscribe the default constitution (the "reaches nobody today" fix).** `dojo.seed_default_governance()` has **zero callers** and the `dojo` schema is `excludes: [dojo]` from the daemon deploy. Invoke the seed **on the Worker/Supabase side** (dojo owns the schema) and **auto-register the global-dōjō as a `knowledge_source` pull subscription on install** so `run_pull_loop` lands it as `origin=federated` general-scope memories. **[The seed-apply itself is a cloud DB write → HUMAN GATE for prod; safe-unattended against e2e.]**

- **Verification:** unit tests on resolve/inject (pure core is already unit-tested — extend it); local browser/session check that a scoped rule appears in `additionalContext`; against e2e DB, confirm seed→pull→`resolve_*`→`rules.md` renders real rules (not the "_No global rules yet_" placeholder).
- **Safe-unattended:** code, hooks, resolve-side, e2e seed test. **Gated:** prod seed-apply; activating the blocking gate (see D-GATE).

### Phase 2 — Tier-3 DDL via `dbd deploy` (mostly safe-unattended; prod apply gated)
Seven tables are missing for Tier-3 (confirmed: `dojo/` has 24 tables, none of these): **`projects`, `scopes`, `billing`, `stance`, `ladder`, `constitution`/`constitution_sections`, `rule_packs`.**
- Author full-DDL (no ALTERs) per-object files under `database/ddl/table/dojo/*.ddl`, plus enums/views as needed. Watch: `shared_rules` deliberately has **no `tenant_id`** (tenant scoping is the auth boundary); `stance` is **user-scoped, not tenant-scoped**; projects likely FK→engagements.
- Add a `seq`-bump trigger for federation republish/retract (the `nextval()`-on-UPDATE divergence — republished/retracted rules currently don't re-surface in a puller's delta). Precedent: `dojo.relay_inbox_seq_bump`.
- Apply via `dbd deploy`/`apply` (never `combine`); seed via **staging tables + timestamp-guarded import procedures**.
- **Verification:** `dbd deploy` + `dbd graph`/`apply` against **e2e/local** DB clean; staging import round-trips; enum alpha-order guarded via CASE rank (known dbd behavior).
- **Safe-unattended:** DDL authoring + apply against local/e2e + import validation. **[HUMAN GATE]:** applying Tier-3 DDL to **cloud/prod Supabase**.

### Phase 3 — Governance authoring + remaining /v1 wiring (safe-unattended code; prod deploy gated)
Now that Tier-3 tables exist, wire the author-side and swap fixtures for real data.
- **/v1 routes** for `projects`, `scopes`, `billing`, `stance`, `ladder`, `constitution`, `rule_packs` following the existing pattern (`resolveTenantAccess` JWT for human/admin, `resolveApiKeyAccess` device-token for machine). Add `toKit*` mappers; flip each `(dojo2)` loader off `fixtures.ts` (degrade-to-empty on 403/404 is the established pattern).
- **Governance authoring write-routes**: org admin composes/edits rules → `dojo.shared_rules` (+ pack membership). This is the missing author ingress (today the only ingress is daemon federation-publish).
- **Billing**: table + route only; the **payment-provider integration is a product decision (D-BILLING) and is out of scope for the unattended run** until locked.
- **Verification:** vitest on route handlers; local `wrangler` + Playwright browser-verify each swapped surface shows **real DB data** (Members-shows-real-member is the reference); confirm the `// Tier 3` comments are gone and the loader reads a route.
- **Safe-unattended:** routes, mappers, loader swaps, local wrangler+Playwright. **[HUMAN GATE]:** Worker **prod deploy**; billing beyond schema.

### Phase 4 — Local-agent coordinator v1 (safe-unattended build; drive stays OFF)
The coordinator is where sensei mirrors superpowers (plan→execute, fan-out, discipline gates) but **daemon-owned**. Today: run engine exists to P3.6 but **drive is OFF by default (`SENSEI_RUN_DRIVE`), single-shot** (`goal` as whole prompt, one `claude -p`), and `driver_for` is **hardcoded to `ClaudeDriver`**.
- **Insertion point:** `relay_drivers/mod.rs::driver_for` — replace the hardcoded return with a **task-typed router** (`task_type × risk × capability-contract → Box<dyn RunDriver>`). The `RunDriver` seam already exists (`trait_def.rs`).
- **v1 scope (honest capability limit):** route only *cheap mechanical* task types to a local/gateway driver (classify/extract/format/doc-gen via single-shot `gateway.execute`); keep reviewer/gate + multi-file integration pinned to Claude (Opus for reviews, exempt from local-first). A **local-model agent-loop driver (ACP)** is a bigger, separate piece — v1 can ship the *router + gateway single-shot leg* and leave the local ACP coding-agent as a follow-up.
- **Attribution ledger:** add a per-inference/`task → driver → outcome` usage row (today only `playbook_run.classified_by`/`model_fallback` exists; gateway tables are config-only). The router can't be tuned/trusted without it. This is DDL (`inference/` table) → Phase 2-style, plus write path.
- **Coordinator loop (Planner→Builder→Judge) is DESIGN-ONLY (P6)** — not in the unattended scope unless D-COORD locks it; v1 stays single-shot-per-step behind the router.
- **Verification:** pure-core unit tests on the router + `map_drive_outcome`; run engine with **drive still OFF** by default (creating runs is always safe); attribution rows appear for routed inferences.
- **Safe-unattended:** router, gateway single-shot leg, attribution DDL/write, tests — all with **drive OFF**. **[HUMAN GATE]:** flipping `SENSEI_RUN_DRIVE` on for any live pilot; adding a local ACP coding-agent driver that autonomously edits.

### Phase 5 — Cutover + release (HUMAN-GATED)
- **[HUMAN GATE]** `/console`→dojo2 redirect.
- **[HUMAN GATE]** merge `develop`→`main` (117 commits) + push.
- **[HUMAN GATE]** prod Worker deploy + prod DB apply/seed.
- **[HUMAN GATE]** activate blocking hook-gate (if D-GATE says so).
- Safe-unattended within this phase: only the *preparation* (redirect code behind a flag, release notes, a dry-run of the merge on a scratch branch) — never the irreversible act.

---

## 3. Per-phase verification gates (what proves a chunk done)

| Phase | Tests | Browser-verify | dbd | Cloud/prod steps that STILL need Jerry |
|---|---|---|---|---|
| 0 Brief/traps | docs lint/build clean | — | e2e DB reachable | none (all local) |
| 1 Rule-pack + injection | `cargo test` resolve/inject pure core (zero-errors); session-context injection unit test | local session shows scoped rule in `additionalContext`; not the "_No global rules yet_" placeholder | seed→pull→resolve verified on **e2e** | **prod** seed-apply; **activating blocking gate** |
| 2 Tier-3 DDL | `dbd` graph consistency; import round-trip | — | `dbd deploy`/`apply`/`graph` clean on **local/e2e**; enum CASE-rank guard | **apply Tier-3 DDL to cloud/prod Supabase** |
| 3 Authoring + /v1 | vitest on route handlers (JWT + device-token planes) | local `wrangler` + Playwright: each swapped surface shows **real DB data**, `// Tier 3` comments gone | routes read real tables (Phase-2 DDL) | **Worker prod deploy**; **billing beyond schema (payment provider)** |
| 4 Coordinator v1 | pure-core router + `map_drive_outcome` unit tests; attribution rows written | run created (drive OFF) via `start_run`/`run_status` | attribution `inference` table deployed on **e2e** | **flip `SENSEI_RUN_DRIVE` on**; **local ACP coding-agent driver**; **P6 loop if in scope** |
| 5 Cutover/release | full workspace `cargo test` + zero-errors-policy | dojo2 smoke via prod-shaped local | prod DB apply dry-run on e2e | **redirect, merge→main, prod deploy, prod seed, gate activation — ALL gated** |

Cross-cutting gate (every phase, every commit): **zero-errors-policy** (zero lint, zero test errors) before any commit; commit/push to **`develop`** only; browser-verify via computed mask (not a11y) for icons; rebuild before any wrangler smoke.

**Safe-unattended vs human-gated — the rule of thumb:** if a step is deterministic, local, reversible, and I can produce evidence it works without a human, it runs unattended. If it crosses a trust boundary (cloud/prod DB, prod deploy, secrets/VAPID/service-role, irreversible cutover/merge, activating enforcement, or a net-new product choice), it stops at a gate — the loop prepares it, stages it, writes the evidence, and waits.

---

## 4. Decisions that MUST be locked before the unattended run

The run cannot ask "continue?"; therefore each of these must be a locked answer, or its chunk is excluded and stays behind a gate.

- **D-INJECT — injection policy.** For daemon-driven/relay runs and SessionStart: is governance **pushed** (resolved rules injected into `additionalContext` / driver context-pack) or does it stay a **nudge to call `get_rules`**? Push is the fix for the core stickiness gap; confirm it's wanted and where (SessionStart + PreCompact + relay driver).
- **D-GATE — blocking hook-gate.** The PreToolUse gate is built but **OFF, fail-open, unregistered**. Does the unattended run **activate** it (and if so, fail-open or fail-closed, for which mandatory tools), or leave it OFF? Default: leave OFF; activation is a Phase-5 human gate.
- **D-SEED — default constitution seeding.** Confirm invoking `dojo.seed_default_governance()` on the **Worker/Supabase** side + auto-registering the global-dōjō pull `knowledge_source` on install. Lock the constitution content (the ~30 mandatory/guideline rows) as the shipped baseline. Prod-apply stays gated.
- **D-ORIGIN — origin vocabulary.** Doc says `authored|promoted|remote`; code writes free-text `learned`/`promoted`/`federated`. Lock the canonical set (recommend: adopt `federated`, retire `remote` from docs) before writing more provenance logic.
- **D-RULEPACK — rule-pack model.** Is a pack a new `dojo.rule_packs` table + join, or a tag/collection on `shared_rules`? Determines Phase-2 DDL.
- **D-STANCE-SCOPE — stance store scope.** Confirm `stance` is **user-scoped** (personal governance is "yours whether or not you belong to a dōjō"), so it needs a **user-scoped, non-tenant** store — not a `tenant_id` table.
- **D-TIER3-DDL — Tier-3 table shapes.** Lock the shapes/keys for `projects` (FK→engagements?), `scopes` (+owner→memberships), `ladder`, `constitution`/`constitution_sections`. Full-DDL-no-ALTER, dbd deploy.
- **D-BILLING — payment provider.** Unspecified anywhere. Until locked, billing = **schema + route only**; no provider integration in the unattended run.
- **D-COORD — coordinator v1 scope.** Confirm v1 = **task-typed router at `driver_for` + gateway single-shot leg for cheap types + attribution ledger, drive OFF**. Confirm Planner→Builder→Judge (P6) and a local ACP coding-agent driver are **out** of the unattended run (design-only / follow-up).
- **D-DRIVE — run drive switch.** `SENSEI_RUN_DRIVE` stays **OFF** for the whole unattended run (creating runs is safe; driving a live agent is a human gate).
- **D-CUTOVER — release gates.** Confirm `/console`→dojo2 redirect, `develop`→`main` merge (117 commits), prod deploy, and prod seed are **all human-gated** (Phase 5), not autonomous.
- **D-DECISIONS-HOME — decisions log location.** Lock the canonical decisions home (recommend reconcile onto `docs/decisions.md` + per-feature `features/<name>/decisions.md`) so run-time decisions land.

---

## 5. Bottom line

An unattended run is safe **today** for: Phase 0 (traps/brief), the code+e2e portions of Phase 1 (injection hook, resolve-side, rule-pack shape), Phase 2 DDL against e2e, Phase 3 routes/mappers/loader-swaps against local wrangler, and Phase 4 router + gateway single-shot leg + attribution with **drive OFF** — all committed to `develop`, each behind a runnable per-chunk gate (`cargo test`/zero-errors, `dbd` on e2e, local Playwright).

It is **not** safe-unattended for anything that writes cloud/prod DB, deploys the Worker to prod, touches secrets/VAPID/service-role, activates the blocking gate, adds a local agent that autonomously edits, or performs the cutover/merge to `main` — those are the human gates, clustered in Phase 5 and flagged inline in Phases 1–4.

The single biggest blocker to *starting* is not code — it is **locking the §4 decisions** (especially D-INJECT, D-RULEPACK, D-TIER3-DDL shapes, D-STANCE-SCOPE, D-COORD scope, D-BILLING out-of-scope). With those locked and the §1.1 doc-traps corrected, the loop has full clarity and can run the safe majority to done, stopping only at the named gates.
