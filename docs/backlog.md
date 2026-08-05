---
name: Implementation Backlog
description: Prioritized index of OPEN work — tracked as GitHub issues (sensei-hq/sensei)
date: 2026-04-28
---

# Implementation Backlog

Work is tracked as **GitHub issues** in [`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues). This file is the prioritized index of **open** work. When an item ships, **close its issue and remove it here** — shipped history lives in git (`git log -- docs/backlog.md`) and in [`plan/decisions.md`](plan/decisions.md).

> The **capability roadmap** — implementation vs vision, ranked gaps G1–G10, phased —
> is [`plan/README.md`](plan/README.md). This backlog is the *issue tracker*; the plan
> is the *why/what-next*. New capability work should land as a G-gap in the plan and,
> when scoped, a filed issue here.

---

## Open GitHub issues (12)

### Epics
| Issue | Summary |
|-------|---------|
| [#91](https://github.com/sensei-hq/sensei/issues/91) | **Dōjō governance track** — admin site + policies + preferences + skills/agents |
| [#85](https://github.com/sensei-hq/sensei/issues/85) | **Track 3 — Project window** (per-screen, separate Tauri window) |

### Observatory · Instruments
| Issue | Summary |
|-------|---------|
| [#96](https://github.com/sensei-hq/sensei/issues/96) | Instruments: background-task visibility (scheduler state + logs UI) — frontend/backend shipped; e2e verify remaining |
| [#90](https://github.com/sensei-hq/sensei/issues/90) | Instruments Replay: usage verdict classifier (used / partial / ignored) — DDL + classifier + endpoints shipped; blocked on the Replay screen to close |
| [#43](https://github.com/sensei-hq/sensei/issues/43) | Observatory Configure section (design + build) |
| [#44](https://github.com/sensei-hq/sensei/issues/44) | J7 Extend & Customize screens (design + build) |
| [#45](https://github.com/sensei-hq/sensei/issues/45) | J5 Pattern knowledge catalog |
| [#46](https://github.com/sensei-hq/sensei/issues/46) | J9 Context-pack tool |

### UI / quality / site
| Issue | Summary |
|-------|---------|
| [#47](https://github.com/sensei-hq/sensei/issues/47) | Mockup component migration to Rokkit (steps 4–14) |
| [#81](https://github.com/sensei-hq/sensei/issues/81) | Update website `/sensei` route to the new mockup (focused sections, covers Dōjō) |

### Foundation / deferred
| Issue | Summary |
|-------|---------|
| [#39](https://github.com/sensei-hq/sensei/issues/39) | Bootstrap diagnostic logging + debug mode (`diagnostic_sessions`+`diagnostic_traces` model + log viewer + anonymized issue export) |
| [#50](https://github.com/sensei-hq/sensei/issues/50) | Extract `bootstrap` into a reusable library (deferred) |

---

## Open — not yet filed as issues

- **Operating-model reframe — Phase 1 continuation** *(active 2026-07-18 — relay landed, resumed)* — sensei+dōjō vision reframe as the **"OS for AI-assisted work"**; spec [`plan/operating-model.md`](plan/operating-model.md) (all 7 open decisions resolved). Phase 1 (Foundations) **SHIPPED on `develop`**: (1) `sensei scaffold` doc-structure command (plan [`plan/2026-07-17-doc-scaffold-command.md`](plan/2026-07-17-doc-scaffold-command.md); commits `b546db74`·`0b8c91f3`·`5bf594a1`); (2) `sensei scaffold feature <name>` per-feature dossier scaffolder (§3.2; plan [`plan/2026-07-18-feature-dossier-scaffolder.md`](plan/2026-07-18-feature-dossier-scaffolder.md); `0f15480e`·`8723520a`); (3) `sensei scaffold baseline --kind <code|content>` capability-contract scaffold (§3.6 → `docs/baseline.md`; plan [`plan/2026-07-18-baseline-scaffold.md`](plan/2026-07-18-baseline-scaffold.md); `6c05e6ce`·`910e1177`); (4) **memory-anchoring** — `spine_slot`/`feature` on `sensei.memories` + heuristic auto-anchor + slot-scoped retrieval (`list_memories_for_slot`, slot-lead `assemble_context`) + MCP `save/propose`/`get_layered_context` slot plumbing (design+plan `plan/2026-07-18-memory-anchoring-{design,plan}.md`; `ab215a87`→`633a40d8`; subagent-driven TDD + whole-feature review). All 4 TDD + reviewer-clean 2026-07-18. NOT merged to `main`. **Phase 1 (Foundations) COMPLETE.** **NEXT = Phase 2 (front-door intake conversation + playbook catalog).** Open decisions parked in the plans' post-impl notes (esp. `promote_memory` anchor-carry; memory auto-inject-by-cwd hook; LLM slot-classification).
- **Codebase-wide silent-error audit** — find + fix every discarded error (`.ok()`, `let _ =`, empty catch, masking `unwrap_or_default`); log so it's inspectable. (Directed after the `node_kind` drop; that was one instance.) → plan WS D.
- **Stale / orphaned project cleanup** — scan reconcile already *tags* dead-but-ambiguous folders `stale` + empty projects `orphaned` (never auto-deletes). Needs list endpoints + a gated purge action + a housekeeping UI.
- **Activity-data GC** — periodic prune of `assistant_events`/`turns`/`sessions`/`transcript_turns` past a TTL, *only after* analysis has derived insights (structured-log TTL already shipped). GC counterpart to the analyzer + transcript backfill.
- **Analyzer/#65 follow-ups** — consolidation TOCTOU race (partial unique index on `(project_id, trigger_detail->>'signature')` + ON CONFLICT); subagent sidechains; embeddings on transcript turns.
- **Per-language calls-edges adapters** — #57 shipped Rust + a language-agnostic call-site contract; Python/Svelte/TS adapters still need to adopt it for `get_callers`/`get_callees`/`call_flow`.
- **Wizard → Preferences arch change** — split the wizard into a thin 5-stage first-run + a persistent editable Preferences surface (operationalises "value before setup"). No new backend. Verify current stage count first.
- **DORA delivery-performance module** — surface the DORA Four Keys (change lead time · deploy frequency · change fail rate · recovery time) + the generative-culture signal *alongside* FTR, derived from the code+activity graph + git history; correlate AI-pairing patterns with DORA movement (the "AI amplifies fundamentals" story). **Prereq: a deploy/release-signal detector** — sensei captures coding, not CI/CD today. Source: [`spec/governance/default-constitution.md`](spec/governance/default-constitution.md) + the Kasperowski *DORA-2025* thesis. → plan Phase 3.
- **Default governance bundle** — ship a curated starter constitution / guardrails / guidelines (DORA + XP/CD + Core Protocols, framed for AI-assisted work) as the **default** personal-Dōjō rules, so a fresh project inherits real rules instead of an empty file. Drafted: [`spec/governance/default-constitution.md`](spec/governance/default-constitution.md); seed `~/.sensei/rules.md` + the marketplace defaults.
- **Dōjō console backend — port `/v1` routes into the Worker** *(decision 2026-07-16: Path A)* — **DONE (2026-07-23): the `dojo-mind` Rust service (`sensei-dojo` binary) has been removed; the dojo Worker's `/v1` is the only dōjō backend, for the console and for senseid's federation (rules + artifacts over the `dojo_protocol` wire).** Original scope: port `dojo-mind`'s `/v1/t/{tenant}/…` logic route-by-route into the CF Worker (cloud Supabase) rather than deploying `sensei-dojo` + proxying; the TS clients + UIs already existed (`engagements/+server.ts` was the reference). Remaining follow-ups tracked elsewhere: the **join/membership flow** (dead "Join" button) and keeping the ported triage scoring / k-anonymity / seq-lock logic correct.
- **Heterogeneous execution router** *(design note 2026-07-19)* — sensei's **own** subagent-style execution that routes typed tasks to the cheapest *capable* driver: gateway inference (local model) for narrow types (classify/extract/format/doc-gen) · local-model agent via ACP for cheap mechanical edits (verify → escalate on fail) · cloud agent (Claude) for integration/architecture/review. The execution-plane of the capability-contract + depth-proportional-to-risk + §9 learning; composes **Relay** (ACP drivers) + the **gateway** (local-first routing). Rationale: Claude Code subagents are **Claude-only**, so local models are unused in subagent runs today. Design note [`plan/2026-07-19-heterogeneous-execution-router.md`](plan/2026-07-19-heterogeneous-execution-router.md). **Prereq: a gateway inference-usage ledger** (no per-inference model/latency/success table today — `gateway/` is config-only; only `insight_copy` persists `resp.model`). First datapoint already shipping via the intake feature's `playbook_run.classified_by`/`model_fallback`. NOT planned.
- **Automated runs — phase 1 (Claude-CLI-driven)** *(design 2026-07-26)* — skills + commands that let Claude Code plan, register, and orchestrate a multi-session automated run relayed via Dōjō, with the daemon as the durable spine (the bootstrap of D-EXEC-TEAM; the daemon-owned team is phase 2+). Three command→skill pairs (`/sensei:analyze`→`analyzer`, `/sensei:plan`→`planner`, `/sensei:execute`→`executor`), an authored plan **graph** with per-task `agent`/`model`/`spec_ref` + typed deps under `docs/plan/<plan-id>/`, plan **registered to Dōjō** by authoring `dojo.relay_segments` (+3 cols) via a new `register_plan` MCP verb, and a minimal 4-verb coordinator contract (`register_plan`·`update_task_status`·`report_run_outcome`·`get_pending_nudges`). Design [`design/automated-run.md`](design/automated-run.md); decisions AR-1..AR-4 in [`decisions.md`](decisions.md). Drive stays **OFF**; Dōjō prod-apply of the DDL is gated (D-TIER3-DDL). Build order: DDL → daemon handlers → MCP wrappers → skills/commands → live smoke.

---

## Relay engine (P0–P6) — see [`plan/relay-engine.md`](plan/relay-engine.md)

The consolidated design + phased build-up. **Supersedes the old R1–R8 pairing
framing** — the "encrypted pairing transport" is dropped; Relay rides the daemon's
existing Dōjō line (poll-first → realtime). Traces:
[`architecture/relay.md`](architecture/relay.md) · [`journeys/relay.md`](journeys/relay.md).
Building autonomously via `/loop` on `develop`.

- **P0 — contract + schema** ✅ *(done on `develop` 2026-07-16; approach A — batched, not merged to `main` yet)* — `dojo.*` relay tables+enums + daemon-local `activity.runs`/`run_events`+enums (**apply to a real Postgres verified**); `dojo-protocol::relay` wire contract (30 tests); daemon `dojo/client.rs` relay methods (9 tests); reviewer gate + semgrep clean; the `relay_inbox.seq` cursor + payload-default fixes from review applied. Commits `e4bf9ca9`·`a27519ff`·`32770c7a`·`db20d41a`·`f5c0aadd`. **Seed + RLS moved to P1** (only exercised by the round-trip; local-Supabase-gated on Docker).
- **P1** ✅ *(done on `develop` 2026-07-16)* — vertical slice **proven end-to-end** (round-trip: daemon device-token → Worker `/v1/relay` → phone JWT reply → daemon poll via `seq` trigger; `scripts/relay-roundtrip.sh`). `dojo-protocol::relay` contract, daemon `dojo/client.rs` methods, `resolveApiKeyAccess` (device-token plane) + `resolveTenantAccess` (JWT plane), Worker `session`/`inbox`/`reply` routes + `relay_inbox_seq_bump` trigger, `memberships.device_token_hash`. Reviewer + semgrep clean. Commits `8708ccb8`·`1ce49015`·`8632752f`·`69c4b5bc`. **RLS + prod expose/grant + plane-B → a hardening pass** ([decisions.md](plan/decisions.md)).
- **P2** ✅ *(done on `develop` 2026-07-16; approach A — batched, not merged to `main` yet)* — real hook triggers + segment feed + PR-review send + running/paused/stuck badge. **Phone UI (C):** Worker `/v1/relay` `segments`/`review`/`gates`/`nudge` routes; `relay-data.ts` client (8 tests) + `relay-view.ts` (13 tests); console run-list + `Relay` nav (`18e4a871`), run-detail segment outline + PR-review Send (`86ff5eea`), "needs you" gate card + band (`0d3b4774`/`9fd600ff`), nudge composer + shared running-pulse status badge (`896de788`/`5dee8a10`); 151 dojo tests + `@testing-library` component tests. **Daemon (Rust):** A2 segment-publish — `TodoWrite` → `relay_project` projection → `upsert_segments`, uuid-guarded, enrolled-membership-gated, fire-and-forget off `ingest_hook_event` (`d0220b08`); B hook-gate — `POST /hook/gate` blocking PreToolUse gate (raise → `await_reply` poll → allow/deny), **fail-open everywhere**, **off by default** (`SENSEI_RELAY_GATE_TOOLS`), zero-knowledge payload (tool name only), hook script **not registered** (`f47fc403`). Each chunk: `bun run check`/`build`/`test` or `cargo build`/`test` + `feature-dev:code-reviewer` (clean). End-of-P2 security gate: `semgrep` 0 findings + `sensei-security-reviewer`. `relay-roundtrip.sh` extended (segments/review/gates/nudge) GREEN. **B activation is a deliberate later step** (per-tool daemon round-trip on live sessions). Follow-ups: multi-membership gating, TodoWrite-enqueue debounce, dojo Playwright harness, RLS, plane-B signing.
- **P3** ✅ *(done on `develop` 2026-07-17; approach A — batched, not merged to `main` yet)* — daemon-owned run engine, built + reviewer-gated + security-gated chunk by chunk (`f001aab9..04618bc5`). **P3.1** run-state model + CRUD (`activity.runs`/`run_events`); **P3.2** `AdvanceRun` tick + scheduler + run API; **P3.3a/b** agent-spawn primitive (`run_agent`, argv-exec, kill+reap) + OFF-by-default drive (`SENSEI_RUN_DRIVE`); **P3.4** limit→pause→auto-resume (the 5-day-run fix); **P3.5** hard-block classifier + progress-over-asking gating; **P3.6** watchdog + crashed recovery (stall→bounded-recover→crash); **P3.7** `plan-depth-reviewer` agent + skill (the depth bar, dogfooded on a real TDD plan); **P3.8** MCP run-control (`start_run`/`run_status` + `POST /api/runs`). **Drive smoke PASSED** on a scratch repo (tick→drive→FeatureDone, correct cwd, no false hard-blocks, watchdog healthy), then drive restored OFF. **End-of-P3 security gate:** `sensei-security-reviewer` PASS (2 High + 1 Medium classifier gaps fixed — `..`-traversal, obfuscated/indirect-run, WebFetch/NotebookEdit coverage) + `semgrep` 0 findings. **DEFERRED (Jerry-gated):** `main`-merge + `make bump`; `SENSEI_RUN_DRIVE` activation on a real run; B hook-gate activation on live sessions. Follow-ups tracked in [decisions.md](plan/decisions.md): `respond_gate` local reply channel, gemma4 classifier backstop, console button migration, fail-open→`public.logs`, RLS, plane-B, limit-parse timezone, multi-membership scoping, daemon `0.0.0.0`-bind, watchdog push-notify.
- **P4** ✅ *(done on `develop` 2026-07-18; approach A — batched, not merged to `main` yet)* — away-from-keyboard, built + reviewer-gated + security-gated + **browser-verified** chunk by chunk (`2e5bd323..b27774c1`). Plan: [`plan/2026-07-17-relay-p4-away-from-keyboard.md`](plan/2026-07-17-relay-p4-away-from-keyboard.md). **P4.3** Web Push client (SW + PWA + subscribe + `push_subscriptions` store); **P4.4** Worker Web Push send (VAPID/aesgcm via `@block65/webcrypto-web-push`, fire-and-forget/fail-open on gate·stall·crash, prefs-gated + dedup + 410-disable); **P4.6** "what's blocked on me" home (urgency-ordered cross-run gates + empty state); **P4.5** offline/reconnect (draft store + partial-failure-safe action queue + reconnect flush); **P4.1** relay RLS (own-rows SELECT-only, `user_id = auth.uid()`; + `supabase_realtime` publication; JWT harness 14/14); **P4.2** realtime swap (client-direct, JWT-authed, RLS-scoped Supabase Realtime → coalesced refresh). **Browser-verified live** (Playwright, screenshots in `relay-p4-*.png`): blocked-home renders with real data + a seeded gate appeared **live 1→2 with no reload** (proves RLS+realtime end-to-end), notify toggle renders, 0 console errors. **End-of-P4 security gate:** `sensei-security-reviewer` PASS (no Critical/High; 2 low fixed — https-only push endpoint + anon/select-only RLS assertions; rest tracked) + `semgrep` 0. **DEFERRED (Jerry-gated):** prod VAPID key (`wrangler secret put VAPID_PRIVATE_KEY`) + prod `PUBLIC_SUPABASE_ANON_KEY`; deploy dojo schema (ships relay RLS+grants); run the `supabase_realtime` publication migration in prod; validate the 4 RLS checks. Realtime transport = **client-direct + RLS** (Jerry-confirmed).
- **P5** — multi-assistant adapters (ACP + fallback ladder).
- **P6** — team relay (folds into the Dōjō `/v1` port above).

---

## Mockup gaps — ✅ RESOLVED 2026-07-14

The 2026-07-14 mockup-gap pass is closed. Details in [`spec/MOCKUP-INDEX.md`](spec/MOCKUP-INDEX.md).

| Gap | Resolution |
|---|---|
| Stale spec component refs | fixed (`Splash`, `ProjAboutPane`) |
| Duplicate splash | `splash-healthcheck.jsx` retired to `discarded/` |
| 3 solution-track screens | drawn (`solution-track.jsx`) + specs wired |
| Relay specs | 13 written (`relay-*.md`) |
| Dōjō console per-role | split into admin/maintainer/lead + **developer**; all specced |
| Extensions/skill editors | `extensions-browser`/`skill-editor` retired; `agent-editor`/`persona-editor` specced |
| Benchmark runner | specced (`benchmark-runner.md`) |
| Prune orphans + `lib/` reorg | done (folders: shared/setup/observatory/project/dojo/relay/data/discarded) |
| Empty/loading/error states | `ScreenState` helper added; screens take a `state` prop |
| _also_ | in-app Dōjō flows → Observatory ⑦; ecosystem architecture board (⑧); `project-atlas` specced |

---

## Cleanup / tech-debt

| Item | Summary |
|------|---------|
| _(file issue)_ | **Rename `setup` remnants after the entry-gate simplification.** The entry gate was simplified to health-gate → folder-scan → dōjō-auto-discover; everything else moved to a separate Configuration surface (see [`features/changes.md`](features/changes.md)). The config routes still live under `app/src/routes/(config)/setup/` and share `setup`-named modules (`app/src/lib/setup/*`, `stages.ts`). Rename/reorganize so the code reflects Setup (the gate) vs Configuration, and drop the leftover wizard-only stages. |

---

## Website

| Item | Summary |
|------|---------|
| _(file issue)_ | **On-page SEO** — canonical, OpenGraph, Twitter Card tags + a generated `sitemap.xml` (root `<svelte:head>`); submit to Search Console. |
| _(file issue)_ | **Website redesign** — screenshots→flows + a "For teams · 結 Dōjō" section + a Teams nav; **reconcile the "0 external requests / local-first" promise with the opt-in networked Dōjō**; trim Dōjō copy to shipped reality. |

---

## Front door — deploy + tooling findings (2026-07-20, from intake e2e verification)

Surfaced while verifying the app intake form end-to-end. The feature itself is fine (4/4 e2e green); these are deploy/tooling gaps that broke the *live* daemon + the e2e harness.

| Item | Detail |
|---|---|
| **Deployed `sensei` DB behind `develop`** | Live DB (from released v0.6.0 bundle) was missing `sensei.playbook_run` (whole table), `playbook_rules.base_priority`, the learned index, and the catalog/guide/rules seeds → `recommend` 500'd (`column base_priority does not exist`). **Interim fix applied** to the local `sensei` DB (surgical additive DDL + seed of playbooks/intake_guide/playbook_rules) so intake works for manual test. **The durable fix is a proper deploy** — a `dbd reconcile` also wants to apply a ⚠ **`gateway.models.capabilities` type change** (`sensei.model_capability[] → model_capability[]`, flagged for a two-snapshot migration) + `sensei.memories` anchoring cols. Fold into the next `make bump`, or reconcile deliberately with the gateway.models change reviewed. |
| **`dbd import` jsonb path broken (dbd-rs 0.8.10)** — [`sensei-hq/dbd#6`](https://github.com/sensei-hq/dbd/issues/6) | **Root-caused → external dbd-rs bug** (`~/Developer/dbd-rs`), not a sensei schema issue. dbd 0.8.10 manages `import_jsonb_to_table` internally ("now managed internally by dbd") and does `CREATE TABLE IF NOT EXISTS _temp(data jsonb)` → `COPY` → `CALL <schema>.import_jsonb_to_table('_temp', <target>)` → `DROP _temp`. The `_temp` table + proc + CALL aren't consistently schema-qualified vs the target table's schema → `relation "_temp" does not exist` (full deploy) / `activity.import_jsonb_to_table does not exist` (explicit `-f`). Hits **every jsonb-column staging table** (assistant_events, **models, routers, models_in_router, libraries, benchmark_reports** — incl. essential gateway config), so it can't be worked around by excluding demo data. Impact: fresh installs + e2e can't self-provision seeds until dbd is fixed (workaround = pre-provision via `dbd apply` + hand-seed, as done for the intake e2e). **Fix in dbd-rs** (qualify `_temp`/proc against the target schema), then repin. The live `sensei` DB is unaffected (already seeded + reconciled). |
| **e2e standard `globalSetup` lacked `SENSEI_DDL_DIR`** | Unlike `globalSetup-cold`, it applied the *released* bundle → stale schema for new columns. **Fixed** (`SENSEI_DDL_DIR` + boot-wait 120→240s). |
| **~~Daemon binds `:7744` only after warmup~~ — CORRECTED: non-issue** | Measured a fresh boot: HEALTH 200 in **3.6s**, GUIDE 200 in 3.7s, **0 model-load lines at boot**. The release build's `embedded-llama-cpp` adapter loads models **lazily on first inference** (not at boot); `fastembed`/`ort` (which do block) aren't in the release build + are env-gated. So the service already comes up fast — the earlier "binds after warmup" note was a misread of a stale log. Optional future nicety: a background pre-warm at boot so the *first* inference call isn't slow. |
| **Classifier under-reads `ux`** | Dogfood: "produce UI mockups/screens…" classified as `intent=feature` (→ `gsd`), not `ux` (→ `mockup_first`). The classifier (LLM + heuristic) doesn't treat design/mockup work as UX. Tighten the `classify_chunk` prompt + `heuristic_axes` (design/mockup/screen/UI/wireframe → `ux`). Good candidate for the §9 learning loop once real runs accrue. |

---

## Code graph — indexer idempotency follow-ups (2026-08-05)

Spec [`spec/pipeline/code-graph.md`](spec/pipeline/code-graph.md) + plan
[`plan/2026-08-05-code-graph-idempotency-plan.md`](plan/2026-08-05-code-graph-idempotency-plan.md)
cover D1–D6 (idempotent indexing). Deferred out of that plan:

| Item | Detail |
|---|---|
| **D5c — `package` + sub-symbol nodes** | Emit `package`/`module` containers above `file` (Cargo/npm/Python packages) and the sub-symbol kinds `property`/`field`/`parameter`/`enum_variant`. P2 — **cut from the code-graph plan** (not required by any P1 Done-gate; Atlas nests on the folder tree + `nodes.parent_id` until then). Pick up after the idempotency plan lands. |
| **Capability roadmap (patterns / search / traceability)** | Pattern-intelligence, LLM-search (FTS), and requirement-level traceability are audited + sequenced in [`analysis/2026-08-05-indexer-capability-coverage.md`](analysis/2026-08-05-indexer-capability-coverage.md) — all gated on the code-graph fixes (embedding survival, deterministic ids, `section` nodes). Roadmap item 0 there: reconcile the three `roadmap`-marked specs (patterns/semantic-search/traceability) to reality. |
| **`branch_switch` single-writer + branch handling** | The D6e/W5 single-writer guard (`enqueue_unique`, keyed on `(kind, folder_path, path)`) deliberately **excludes** `branch_switch` (scan.rs) because the key omits `branch` — guarding it could silently drop a branch switch that races an in-flight plain scan, leaving `props.branch` stale. `branch_switch` keeps plain `enqueue` for now (always applies). Proper fix: make branch_switch single-writer too, but on dedup either supersede the in-flight scan or apply the new branch to it (so a branch switch is never a silent no-op). Design + implement in a later worker increment. |
| **⚠ D1 DEPLOY GATE — clear/dedup the graph BEFORE the edges.ddl unique indexes reach a live DB** | D1 (edge identity) added two partial UNIQUE indexes to `edges.ddl` (`edges_unique_resolved`, `edges_unique_unresolved`) and made `insert_edge`'s `ON CONFLICT` depend on them. `CREATE UNIQUE INDEX IF NOT EXISTS` skips only by index NAME, never by data — so on a live DB that still holds duplicate edges (the dev `sensei` DB had ~2.1M edges, dup-laden), index creation FAILS, and the daemon's boot-time `dbd apply` returns a hard `Err` → the daemon can't materialize the schema. **Before deploying the D1 binary + DDL to any live DB, run the spec Migration (docs/spec/pipeline/code-graph.md §Migration): `TRUNCATE sensei.edges, sensei.nodes CASCADE; TRUNCATE inference.communities;` then re-derive traceability + re-scan** (or the drift-preserving dedup fallback). sensei_test was truncated + indexed for CI. Nothing in code enforces this ordering — it's an operator gate. |
| **W3 — tolerated parse errors should advance `scan_state`** | inc5 (D6c-trigger) implemented the fatal side (a fatal DB write → `Err` + folder `failed` + `scan_state` NOT advanced). The spec's W3 also says a **tolerated** parse/read error should keep `Ok` **and advance `scan_state`** so a persistently-unparseable file isn't re-parsed on every scan. Currently the tolerated path (binary skip / parse error / parser panic) returns `Ok(0)` early **without** writing `scan_state` (pre-existing behaviour, unchanged by inc5) — so such files are re-parsed each scan (wasteful, not incorrect). Land the tolerated-advance (write the fingerprint on the tolerated-skip paths) with a test; low-risk, deferred from inc5 to keep that increment scoped to the load-bearing fatal/fail-closed behaviour. |
| **Fault-injection seam for `build_connections` status-read error** | inc5's shared fail-closed writer (`helpers::mark_folder_indexed_fail_closed`) declines to mark `indexed` when `get_folder_status` itself errors, but that branch has no test (no seam to force a read error). The recovery re-drive (a `failed`/`indexing` folder is re-driven next scan) self-heals a transient read blip, so it's low-risk; add a seam + test when convenient. Also: an end-to-end `fatal_file_failure_is_recorded_and_retried` driving the `process_file` fault seam through the real worker (asserting `task_executions.retry_number` increments for `ProcessFile` specifically) — currently the retry wiring is proven generically via `ProcessGitFolder`. |
