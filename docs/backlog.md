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

## dbd: `policies` ignores `--scope`

**Found 2026-08-25.** dbd 0.10.12. `dbd policies --scope default` applies every
file under `policies/`, including `policies/dojo/*.sql`, on a plane whose scope
EXCLUDES `dojo`. Each one then fails with `schema "dojo" does not exist` and the
run reports `Policies: 0 applied, 4 failed`.

Not fatal — the daemon's install path calls dbd's `apply` and `import_data`
directly and never runs the policies phase — but a manual `dbd deploy --scope
default` printed four FAILED lines that were an expected condition, which is how
real failures stop being read.

**Worked around** by wrapping each policy file's statements in a `do $$` block
that returns early when `to_regclass('dojo.<table>')` is null. Verified both
directions: the dōjō plane applies 4/4 with every policy, grant and RLS flag
present; the daemon plane reports 4 applied, 0 failed and creates nothing.

**Upstream:** `policies` should honour `--scope` the way `apply` and `import` do.


## Seeding: two mechanisms, one broken — migrated to staging + import_ (2026-08-25)

Found by asking why some procedures were named `seed_*` when every other seed
goes through `staging.<table>` + `import_<table>`. Three defects, each invisible.

**1. `seed_ponytail_pack()` had been failing since a column rename.** It writes
`rule_packs.source`; the column is `attribution`. `import_rule_packs` maps
`stg.source → attribution` correctly — the staging path was updated for the
rename and the hardcoded procedure was not. A plpgsql body is not validated until
it is CALLED, so nothing flagged it.

**2. That took the default constitution down with it.** The daemon ran
`psql -c "CALL seed_default_constitution(); CALL seed_ponytail_pack();"` — one
implicit transaction, `ON_ERROR_STOP=1` — so ponytail's error rolled BOTH back.
The caller was fail-open (`tracing::warn!(… "non-fatal")`). Net effect: every
fresh install since the rename shipped with ZERO bundled governance packs,
including the constitution, and said nothing. Existing installs were fine, which
is why it stayed hidden.

**3. dbd never called the `seed_*` procedures at all.** It creates them; only
`import_*` procedures run during a deploy. So the dōjō plane never had the 5
seeded packs — 36 of the 76 shipped library rules.

**4. Rule-pack RULES never landed on a fresh install either.** dbd does not order
imports, and `import_rule_pack_rules` sorts before `import_rule_packs` (`_` <
`s`). It joined an empty `rule_packs`, inserted nothing, and reported success.
`import_rule_packs` now drives its dependents in SQL, so order does not matter.
An explicit list under `import.staging:` in design.yaml does NOT affect order —
verified.

**Fixed by migrating all seeds to the staging model**, so there is one mechanism:
5 packs / 36 rules / 3 adoptions / 1 tenant moved to jsonl datafiles, and
`seed_default_constitution`, `seed_ponytail_pack` and `seed_global_dojo` deleted
along with the `seed_bundled_packs` shell-out.

Seeded ids are FIXED, not generated: `dojo.tenants.id` and the `general/global-dojo`
namespace id default to `gen_random_uuid()`, so an imported row got a different
uuid on every plane and every reset — divergence for a row that is global by
definition. Both now carry a uuid5 derived from their natural key.

**Still open — upstream in dbd:** imports are not dependency-ordered, and a
staging table whose import inserts nothing reports success. Both are silent.


## dbd drops a DDL file it cannot parse — silently, and the deploy still reports success

**Found 2026-08-25** while fixing `dbd deploy --scope dojo`. dbd 0.10.12.

Two files failed dbd's SQL parser and were removed from the entity set with no
effect on the deploy's exit status:

* `comment on function dojo.can_read_repository_metric(uuid, text, uuid, uuid) is …`
  → *Expected: comment object_type, found: function*. dbd parses
  `comment on function <name> is …` but not the form WITH an argument list.
* `revoke all on function … from public, authenticated;`
  → *Expected: end of statement, found: authenticated*. A comma-separated grantee
  list is accepted on GRANT but not on REVOKE.

The consequence is worse than a failed deploy: `dojo.can_read_repository_metric`
and `dojo.set_pack_adoption` were never created on any database, and nothing said
so. `set_pack_adoption` is called by the dōjō `/v1` rule-pack route, so that call
has been failing against a function that does not exist. `dbd inspect` DOES
report both, and `dbd deploy` prints the error count — but continues and finishes
with "Fresh install at v0 — 91 entities applied", which reads as success.

**Worked around in our DDL** (both changes are semantically identical): the
COMMENT drops its argument list, and the REVOKE is split one grantee per
statement. Both carry a comment saying why, so neither gets "tidied" back.

**Upstream:** dbd should fail a deploy when a file in scope does not parse, or at
minimum exit non-zero. A schema tool that silently omits an entity gives an
answer indistinguishable from success.


## kavach calls `resolve(event)` twice — every POST body under a public rule arrives empty

**Status:** RESOLVED 2026-08-25 in kavach 1.1.0; dōjō bumped to it.

The fix was already in kavach's source — 1.0.2 resolved once — but 1.0.2 was
published with no `dist/`, so it had no type declarations and dōjō could not move
to it (7 svelte-check errors). Cause: the publish workflow builds the tarball
with `bun pm pack`, which does not run `prepublishOnly`, and then runs `npm
publish <tarball>`, which does not either — npm runs lifecycle scripts when
publishing a DIRECTORY, not a prebuilt tarball. So `dist/` was never built in CI.

kavach now builds before packing and FAILS the publish if a tarball lacks the
types its manifest promises, plus two regression tests pinning the single-resolve
invariant. Released as 1.1.0.

Original diagnosis, kept for the record:

In `node_modules/kavach/src/kavach.js`, `handleUnauthorizedAccess` returns
`resolve(event)` when access is ALLOWED — a `Promise`, not a `Response`. The
caller, `handleRouteProtection`, then tests `protection instanceof Response`,
which is false for a Promise, so it falls through to its own `return
resolve(event)` at the end. `resolve` therefore runs twice for every permitted
route. The first run drains the request body stream; the second sees an empty
body.

GET is unaffected (no body). Every POST through a permitted route loses its body.

Reproduced 2026-08-25 against dōjō dev: `POST /v1/auth/cli/refresh` with
`Content-Length: 25` reached the handler with `request.text() === ''`. Replacing
`hooks.server.ts` with a bare `resolve(event)` made the same request arrive
intact, and restoring kavach reproduced the empty body immediately.

This is NOT specific to the new CLI-auth endpoints. It affects every existing
`/v1` POST that reads `request.json()` once the caller is authenticated —
`/v1/you/dojos`, `/v1/you/contributions/adopt`, `/v1/you/invites/accept`,
`/v1/you/rule-packs/[slug]/adopt`, `/v1/you/github/sync`. Those all call
`resolveCaller` first, which reads only headers, so an unauthenticated probe 401s
before touching the body and hides the fault.

**Fix (upstream):** `handleUnauthorizedAccess` should return the guard Response or
`null`/`undefined`, and let `handleRouteProtection` own the single `resolve` call.

**Not worked around here.** Buffering the body in `hooks.server.ts` and passing it
via `locals` would mean every handler reads its body from a non-standard place —
a workaround for a library bug spread across the whole API surface. Per the
project rule on silent workarounds, this is recorded rather than hidden.


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

- **Metrics engine rebuild + Dōjō sync (v1 PRIMARY)** — spec [`spec/2026-08-18-repo-grain-metrics-watermark-engine.md`](spec/2026-08-18-repo-grain-metrics-watermark-engine.md) (DRAFT, **D1–D16 locked, no code yet**). Move metrics to **repo-grain** (global `sensei.repositories` keyed on the normalized remote; `folders`→repository N:1, only the root folder is a checkout; metrics key on `repository_id`; project = aggregation view). A **project-orchestrated watermark engine** (`ComputeProjectMetrics` freezes one `as_of` → cadence-aware group children; per-`(repo×group)` watermark **retires** `covered_days`/`effective_from`/global `metrics.last_run` and the phantom-`covered_days` fix `81c49c2d`). **User-attributed quality/churn** (commit-walk over the local user's own commits ∩ touched files; config-pinned qlty + shared cache; `scope∈{user,repo}`, one scan → two derivations). **Scanner hardening (D15)** — `.git` dir OR file, nested + incremental-safe, so a nested checkout isn't masked as a subdir update. **Views/analytics** (compare · weekly/monthly snapshot · box/violin where D13 applies). **Dōjō me-vs-team** — `member_metrics` (own-RLS) + `repo_metrics` (dedupe by `repository+sha`), private + aggregate default (D9), two-way sync + enrollment reusing `sensei.dojo_memberships` + OS Keychain credential, server-side tenant isolation (D16). **v1 = P-A/P-B (local); P-C (Dōjō sync) rides Phase 2** ([phases.md §2.1](design/phases.md)). Post-metrics follow-up: **branch-awareness program** (indexer no-blind-delete on branch removal, branch-scoped code graph / MCP / UI / lib-docs — currently untracked).
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
- **`duplication_ratio` "new-symbol" (write-time) version** *(deferred 2026-08-09, from metrics Phase 5.3)* — v1 ships `duplication_ratio` as a **point-in-time snapshot** (eligible symbols in a duplicate cluster ÷ eligible symbols, via `PgStore::duplication_stats_scoped`). The catalog's original intent was the temporal measure — *new* symbols whose name/embedding collides with an *existing* symbol ÷ new symbols (a write-time DRY-erosion signal). That needs a way to tell a freshly-created symbol from an old one: `sensei.nodes` has **no reliable creation timestamp** (`modified_at` is bumped on every rescan, so it silently mismeasures under the version-rescan pass). Follow-up: add an **immutable `nodes.created_at`** (never touched by the `upsert_node_ex` / FQN `ON CONFLICT` branches) — or compute duplication **at write time** when "new" is unambiguous — then add the new-vs-existing partition (likely extending `find_duplicates_scoped` to carry node ids / `created_at`). Until then the snapshot is the shipped metric.
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
| ✅ **RESOLVED 2026-08-22** | **Duplication / maintainability charts — resolved by fixing the axis, not by plotting the rating.** The plan was to plot the 0–5 rating instead of the raw value (module_quality's range is 0–0.00527, so every `toFixed(2)` tick read "0.00") and to retune `module_quality`'s `rating_scale`. Neither was needed. The complaint ("scale is too small, the chart is not helpful") was caused by the tick formatter, fixed in fbb6f0d0 — `metricTickFormatter` keys off the metric, not just its type, and renders these two per-1,000-lines. Live data: module_quality raw 0–0.00527 (avg 0.00154) → per-kloc 0.0–5.3 (avg 1.5); duplication_ratio raw 0–0.897 (avg 0.0439) → per-kloc 0–897 (avg 43.9). Both readable. **Plotting the rating would regress it**: across 115 periods module_quality's rating takes only 3 distinct values (3:10, 4:49, 5:56) and duplication_ratio 4 (2:8, 3:36, 4:4, 5:67), so a continuous series becomes a 3-step staircase — and it would need a `rating` column threaded from `metric_rating_facts` through `get_project_metric_series` to do it. The rating is already legible as the A–F grade chip: `PER_KLOC_GRADES` [3, 6, 12, 25] is the same ladder as `rating_scale` [0.025, 0.012, 0.006, 0.003, 0.0015] (= per-kloc 25, 12, 6, 3, 1.5) minus its top step, so the two systems agree by construction. The scale was kept as an absolute ladder by decision — it does discriminate (10 periods sit at rating 3), and rebasing it on the observed average would make the bar relative to current quality rather than to a standard. Guard added in `metric-view.spec.ts`: spot values across each metric's real observed range must produce distinct tick labels. |
| **dojo transport — 2026-08-25** | **Do NOT mass-delete `crates/senseid/src/dojo/`.** An earlier analysis claimed ~7,600 LOC targets the non-existent dōjō service; measured per file, only `client.rs` (1,101) and `crates/dojo-protocol` (1,561) are transport-bound. The other **4,979 LOC is local logic every transport needs** — `attribution.rs` (the confidentiality dereference: client identifiers must never leave the machine), `gate.rs` (the hook-gate control leg, unrelated to dōjō and reached by `/hook/gate`), plus routing/membership/contribution assembly. The transport gets REPLACED in Phase 7, not removed; it costs nothing meanwhile (`dojo_outbox` has never held a row). Build nothing new on `client.rs` until the transport is decided. |
| **RLS note — 2026-08-25** | **A Supabase row policy runs as the CALLING role, so every table it reads needs its own grant.** Written inline, the repository-metrics policy traverses principals → team_members → teams → team_projects → repositories_in_projects → memberships — which would have meant granting every signed-in user SELECT on the entire organisation graph just to filter their own metrics: the check leaking more than it protects. Correct shape is a `SECURITY DEFINER` function (`dojo.can_read_repository_metric`) with `set search_path = dojo, pg_temp` — the traversal runs as the owner, the caller needs no grant and learns only a boolean. Also: RLS alone is not enough, the role still needs `GRANT SELECT ON <table>` or every read fails "permission denied" rather than returning an authorised subset. |
| **dbd note — 2026-08-25** | **Two more dbd gaps, found deploying the dōjō scope.** (1) It emits a PLACEHOLDER constraint name for a drop — `ALTER TABLE … DROP CONSTRAINT uq:tenant_id,provider,subject` — which is a syntax error; look the real name up in `pg_constraint` and do it by hand. (2) An enum used as a COLUMN TYPE is not FK-reachable, so a cross-scope table that gains one fails with `type "entity_origin" does not exist` until the enum is listed explicitly in that scope's `includes` (design.yaml documents this for other enums; it applies to every new one). Also re-confirmed: a renamed column needs `ALTER … RENAME` before reconcile, or dbd plans ADD+DROP and destroys the data — it would have dropped 14 rule-pack citations from the Supabase copy exactly as it nearly did locally. |
| **dbd note — 2026-08-24** | **A view column ADD/REORDER needs a DROP+CREATE, not `dbd reconcile`.** dbd emits `CREATE OR REPLACE VIEW`, which Postgres rejects with `cannot change name of view column "x" to "y"` whenever the new definition inserts a column anywhere but the end — reconcile then fails mid-run having already applied the table changes. Hit twice adding `persona_id` to `sensei.project_metrics`. Workaround: `DROP VIEW … CASCADE`, then re-apply the view DDLs iteratively (a dependent view can only be created after its source, and the chain here is 5 levels deep — metric_facts / project_metric_daily → weekly/monthly/quarterly/trend → metric_ratings / project_health_*). Two related dbd gaps found the same day, both destructive if trusted: a `text → enum` column change is emitted WITHOUT the required `USING` clause (fails), and a table rename is planned as `create` + `drop` (would have destroyed 15,424 rows). **Dry-run any dbd plan touching an existing column or table in a rolled-back transaction before applying it.** |
| _(file issue)_ | **`app/e2e/**` is outside every static gate** — `tsconfig.json` extends `.svelte-kit/tsconfig.json`, whose `include` covers `src/` only, so `bun run check` (1114 files, 0 errors) and `tsc --noEmit` never look at the Playwright suite: `tsc --listFiles` matches **0** files under `e2e/tests`. Typechecking it via a probe config (`{extends: "./tsconfig.json", include: ["e2e/**/*.ts"]}`) surfaces **9 pre-existing errors** — 3 in the playwright configs (`TS2769` no matching overload), and 6 in specs: `boot-flow.spec.ts:73` and `dojo-binding.spec.ts:136` pass a `RegExp` where a `string` is expected (`TS2345`), and `cold-start.spec.ts:49`, `instruments-observatory.spec.ts:63`, `instruments-t2-slices.spec.ts:146` (×2) pass 2 args to a 1-arg call (`TS2554`). Fix those, then add an `e2e` project to the check script so the gate covers them. Until then an e2e spec can only be validated with `bunx playwright test --list` (proves it transpiles and registers, not that it type-checks). |

### Quality pass (qlty.sh) — 2026-08-10 status + follow-ups

A qlty pass ran on 2026-08-10 (commits `2bd4dc2b`..`992c7256`). Landed: excluded vendored tree-sitter grammars (qlty smells 427→394); deduped the metrics test read-back helpers into `test_support` (→386); fixed two pipeline blockers (the bootstrap `brew`-shelling hang, and the app suite failing 98/98 on a missing generated tsconfig — app coverage was silently 0, now runs at ~45%); excluded generated `paraglide` from dojo coverage; added rulepacks-data DB-wrapper tests; and brought **senseid's pure (DB-free) modules** into CI coverage (`scripts/senseid-pure-coverage.sh` + a `senseid` job) — lifting the Rust+dojo aggregate from 75.8% to a locally-projected ~80.7%. Open follow-ups:

| Item | Summary |
|------|---------|
| _(file issue)_ | **Security: the 28 osv advisories stay tracked in [#120](https://github.com/sensei-hq/sensei/issues/120)** (all `medium`, transitive, none code-fixable). Left un-allowlisted per owner decision — the dashboard keeps showing them as known/accepted rather than suppressed. |
| _(file issue)_ | **~~sumi-palette → shared design-tokens package~~ DONE (c64a56b6).** `packages/sumi-palette` now holds the canonical Zen/Sumi OKLCH palette, consumed via a build-time relative import from all three `rokkit.config.js` (no workspace/lockfile change). Removed the mass-467 app↔dojo duplication AND unified `website` (which had drifted to a separate hex palette) onto the one brand palette. All three build clean. **Follow-up: a visual QA pass on the website** — its rendered colours change from the old hex values to the canonical OKLCH brand palette (same values app/dojo already ship). |
| _(file issue)_ | **Remaining high/medium smells (~388 after the pass).** The `view.ts` "hotspot" was an attribution artifact — its only structural smell is `scopeLabel`'s guard-clause returns (house style, correct as-is). **Partially done:** `project_detail.rs` — extracted `resolve_existing_project` (DRY'd 18 handlers; dup 4→2, commit `6c90110d`). **Assessed as NOT worth forcing:** the remaining api-handler dup pairs (`query` query_functions/query_types, `workspace`, project_detail's `decide_*` pair, `gateway_embedded`, `insight_copy`) are near-identical async handlers wrapping a *differing store call* — Rust extraction needs async closures/generics that trade duplication for indirection (readability wash, against "simplest design"). `many-parameters` on the language `make_sym`/`resolve_call`/`collect_calls` (bundle into a param struct) is still a clean candidate if pursued. The language tree-walkers (`rust_lang` 21 / `python` 20 / `typescript`/`swift`/`kotlin`/`java`) hold most structural smells but are inherently complex + well-tested — low-value/high-risk to refactor for the metric. |
| _(file issue)_ | **Coverage follow-ups to push past ~80.7%.** (a) The DB-coupled senseid remainder still isn't measured — needs a CI Postgres service (the documented parallel-DB flakiness must be handled first: per-test schema/txn isolation). (b) Longer-term, extracting the pure senseid modules into their own crate would drop the hand-maintained allowlist in `scripts/senseid-pure-coverage.sh`. (c) app coverage is now runnable (~45%) but stays OUT of the aggregate — the CI `--coverage` job still hits a linux-only rolldown `node:module` crash; fixing that would upload app at ~45% and *drop* the aggregate, so improve app coverage first. |

| Item | Summary |
|------|---------|
| _(file issue)_ | **Rename `setup` remnants after the entry-gate simplification.** The entry gate was simplified to health-gate → folder-scan → dōjō-auto-discover; everything else moved to a separate Configuration surface (see [`features/changes.md`](features/changes.md)). The config routes still live under `app/src/routes/(config)/setup/` and share `setup`-named modules (`app/src/lib/setup/*`, `stages.ts`). Rename/reorganize so the code reflects Setup (the gate) vs Configuration, and drop the leftover wizard-only stages. |
| _(file issue)_ | **`measure_pending_verdicts` computes verdicts against a fabricated `0.0` baseline (never-fabricate violation).** `PgStore::measure_pending_verdicts` (`crates/senseid/src/db/pg_store.rs`) reads `COALESCE(r.baseline_ftr, 0)` and classifies the verdict on `current_ftr - baseline_ftr`. But `accept_recommendation` never stamps `inference.recommendations.baseline_ftr`, so it's NULL on ~100% of rows (confirmed in `docs/analysis/2026-08-04-deep-dive/08-instrumentation-gaps.md`: 0/1478 populated) → every verdict is measured against a fabricated 0.0 baseline, so any project with real FTR looks like a massive "positive". Fix: stamp `baseline_ftr` at accept time from `PgStore::get_project_ftr_rate(project_id)` (the consolidated FTR number), and in `measure_pending_verdicts` SKIP a rec whose `baseline_ftr IS NULL` (honest-absent — never a fabricated 0 delta) rather than defaulting it. Surfaced during metrics Phase 8 (FTR consolidation); the verdict path was verified untouched by the view retirement but carries this pre-existing fabrication. |
| _(file issue)_ | **`docs/spec/pipeline/impact.md` is stale — names a non-existent `measure_verdicts.rs` + wrong tables/enum.** The impact/verdict spec references an owner file / tables / enum values that don't match the shipped code (the real handler is `verdicts.rs::measure_verdicts → PgStore::measure_pending_verdicts`, storing on `inference.recommendations` + `inference.reasoning_traces.consensus`). Rewrite `impact.md` to the shipped reality: the acted→measured loop, the ±0.05 FTR band in `crate::verdicts::Verdict::from_ftr_delta`, and (once the baseline bug above is fixed) the honest baseline source. Surfaced during metrics Phase 8. |
| _(file issue)_ | **Non-isolated DB tests fail under the full parallel suite (shared `sensei_test`).** `cargo test -p senseid` shows failures that pass in isolation: `publish_run::full_bridge_authors_plan_graph_segments` + `full_bridge_publishes_status_segments_and_persists_session_id` assert absolute row counts (got 5 vs 3, 2 vs 1) polluted by leftover rows / concurrent writers; `metrics::session_outcomes::ftr_parity_store_vs_views`, `library_update_scheduler::security_bump_*`, `version_rescan::rescan_is_crash_safe_*` fail only under parallel load. Verified pre-existing (same failures on pre-change code via `git stash`, 2026-08-09). Fix: per-test schema/txn isolation or serialized DB-test lane; these assert against a shared DB without cleanup. |
| [#120](https://github.com/sensei-hq/sensei/issues/120) | **osv-scanner: 28 remaining dependency advisories (all `medium`), none fixable by us.** After the sweep — `anyhow`→1.0.104 (RUSTSEC-2026-0190), `crossbeam-epoch`→0.9.20 (RUSTSEC-2026-0204), `event-listener`→5.4.2 (RUSTSEC-2026-0221), `tar`→0.4.46 (GHSA-3pv8-6f4r-ffg2) — all cleared. The rest are transitive + unactionable: **(a) Tauri Linux GTK stack** — `gtk`/`gtk-sys`/`gdk`/`gdk-sys`/`gdkwayland-sys`/`gdkx11`/`atk`/`glib 0.18.x` + `gtk3-macros`, plus unmaintained `paste`/`proc-macro-error` on the same chain — pinned by `tao 0.35`/`muda 0.19`/`wry`→`webkit2gtk 2.0.2` inside **`tauri 2.11.1`** (the whole Tauri 2.x line still rides gtk-rs 0.18). **Linux-only — NOT compiled into the shipped macOS `.app`** (`tao` uses Cocoa on macOS; `cargo tree` shows nothing for the macOS target). Clears only when Tauri/tao/wry adopt gtk-rs 0.20 upstream — this is **not a Tauri 1→2 migration** (the app has always been on Tauri 2.x). **(b)** `quick-xml` 0.36/0.37/0.39 — RUSTSEC-2026-0194/0195, patched in 0.41.0 but dependents pin ≤0.39. **(c)** `unic-*` (via `urlpattern`→tauri), `number_prefix` — unmaintained Tauri-owned transitives. **(d)** `rsa 0.9.10` (CVE-2023-49092 Marvin) — no fixed release exists. **Action:** consider allowlisting the Linux-only / no-fix advisories in `.qlty/qlty.toml` with rationale; revisit the GTK stack when Tauri ships gtk-rs 0.20. (Aside: Dependabot reported the `tar`/`glib` alerts as "fixed" while the lockfile still pinned the vulnerable versions — its graph was out of sync.) |
| _(file issue)_ | **Deploy `time_to_useful_result` metric** — implemented + tested (definition (B) time-to-first-useful-turn), see [`plan/2026-08-09-time-to-useful-result-metric.md`](plan/2026-08-09-time-to-useful-result-metric.md) §6. Remaining: `dbd import metrics` + apply the 3 `project_metric_*` views to the live `sensei` DB, then redeploy the daemon so the computer ships. The two sibling candidates (accuracy_improvement, revisions_needed) were assessed derivable (FTR trend + rework family) and NOT added. |

---

## Website

| Item | Summary |
|------|---------|
| _(file issue)_ | **On-page SEO** — canonical, OpenGraph, Twitter Card tags + a generated `sitemap.xml` (root `<svelte:head>`); submit to Search Console. |
| _(file issue)_ | **Website redesign** — screenshots→flows + a "For teams · 結 Dōjō" section + a Teams nav; **reconcile the "0 external requests / local-first" promise with the opt-in networked Dōjō**; trim Dōjō copy to shipped reality. |
| _(file issue)_ | **Componentize `routes/sensei/+page.svelte`, then finish its responsive pass** — the last desktop-first `@media` block in `website/` (11 of 12 converted to mobile-first prefixes in 95d2208d). This one is a 1194-line monolith whose single `@media (max-width: 900px)` block carries 36 rules over ~36 section classes. Converting it in place is mechanical but makes the file *less* readable (36 elements each gaining 2–4 responsive utilities) without addressing the real deviation: every sibling section on the hub page is already a component (`lib/components/hub/Hero.svelte`, `Footer.svelte`, `Approach.svelte`, …), so this page — and `routes/torii-seiki/+page.svelte`, 395 lines — are the house-style outliers. Extract the ~12 sections into components first (§1.1/§1.5), converting each one's responsive rules as it moves. The block also carries literal-px debt §1.3 forbids (`padding: 16px 24px`, `font-size: 180px` / `280px`), which the extraction should clear at the same time. |

---

## Front door — deploy + tooling findings (2026-07-20, from intake e2e verification)

Surfaced while verifying the app intake form end-to-end. The feature itself is fine (4/4 e2e green); these are deploy/tooling gaps that broke the *live* daemon + the e2e harness.

| Item | Detail |
|---|---|
| **Deployed `sensei` DB behind `develop`** | Live DB (from released v0.6.0 bundle) was missing `sensei.playbook_run` (whole table), `playbook_rules.base_priority`, the learned index, and the catalog/guide/rules seeds → `recommend` 500'd (`column base_priority does not exist`). **Interim fix applied** to the local `sensei` DB (surgical additive DDL + seed of playbooks/intake_guide/playbook_rules) so intake works for manual test. **The durable fix is a proper deploy** — a `dbd reconcile` also wants to apply a ⚠ **`gateway.models.capabilities` type change** (`sensei.model_capability[] → model_capability[]`, flagged for a two-snapshot migration) + `sensei.memories` anchoring cols. Fold into the next `make bump`, or reconcile deliberately with the gateway.models change reviewed. |
| ~~**`dbd import` jsonb path broken (dbd-rs 0.8.10)**~~ ✅ **RESOLVED** — fixed by the dbd-core `0.8.1→0.10.5` upgrade (`ea163702`, which "unblock[ed] e2e DB setup" — the jsonb import path was the blocker; the staging import path is now used, e.g. `215e72f3`). Live `dbd` is `0.10.4`. [`sensei-hq/dbd#6`](https://github.com/sensei-hq/dbd/issues/6) | ~~**Root-caused → external dbd-rs bug** (`~/Developer/dbd-rs`), not a sensei schema issue. dbd 0.8.10 manages `import_jsonb_to_table` internally ("now managed internally by dbd") and does `CREATE TABLE IF NOT EXISTS _temp(data jsonb)` → `COPY` → `CALL <schema>.import_jsonb_to_table('_temp', <target>)` → `DROP _temp`. The `_temp` table + proc + CALL aren't consistently schema-qualified vs the target table's schema → `relation "_temp" does not exist` (full deploy) / `activity.import_jsonb_to_table does not exist` (explicit `-f`). Hits **every jsonb-column staging table** (assistant_events, **models, routers, models_in_router, libraries, benchmark_reports** — incl. essential gateway config), so it can't be worked around by excluding demo data. Impact: fresh installs + e2e can't self-provision seeds until dbd is fixed (workaround = pre-provision via `dbd apply` + hand-seed, as done for the intake e2e). **Fix in dbd-rs** (qualify `_temp`/proc against the target schema), then repin. The live `sensei` DB is unaffected (already seeded + reconciled). |
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
| **D4 — community-id determinism: add parent_id/id tiebreak when D5c lands** | D4a made `community_id` deterministic by ranking communities via each community's earliest node in `natural_key` order `(file_path, line_start, kind, name)`. Today that key is injective over live rows (two nodes sharing it would also share `parent_id` and collide on `nodes_unique_identity`), so it's fully deterministic. Once **D5c** nests symbols under their class/impl (distinct `parent_id`), two same-file/line/kind/name symbols under different parents become possible and their relative order is unspecified — add `parent_id` (and `id` as a final total-order tiebreak) to both `community::natural_key` and `get_nodes_scoped`'s `ORDER BY` then. Also: cross-reindex/cross-process determinism (vs HashMap iteration) isn't exercised by the unit suite — the `graph_scan_end_to_end` integration test (spec) is the intended proof. |
| **D3 — full move-resilience deferred (identity key kept on `line_start`)** | The spec's D3 wanted the node identity keyed on `signature` (not `line_start`) so a symbol that MOVES lines keeps its id. That was implemented, then REVERTED after review: `signature` is the raw declaration-line TEXT (rust_lang.rs `line_at`) and symbols are parented to the file node (not their class/impl — that nesting is D5c), so two same-name methods with identical decl lines (`fn fmt(&self, …)` across impl blocks — ubiquitous) would collapse to one node, silently dropping symbols. So D3 ships upsert-then-prune + per-file edge reconcile on the EXISTING `line_start` key: a symbol that doesn't move keeps its id (+community_id/degree/embedding); a moved symbol re-mints (re-embeds). **Full move-resilience needs D5c symbol nesting first** (parent_id per class/impl, so `signature` can safely replace `line_start`); revisit the key change then. |
| **D3 — content_hash omitted (deviation from spec D3, justified)** | The spec added a `nodes.content_hash` column to re-null a symbol's embedding when its *body* changed. But `embed_text` (embed.rs) embeds only `kind + name + signature + file_path` — never the body. On a same-identity re-upsert (same `line_start`), `kind`/`name`/`file_path` are fixed by the key, so `signature` is the ONLY embed input that can change — `upsert_node`'s DO UPDATE re-nulls `embedding` exactly when `signature` changes (and preserves it otherwise). So a content_hash column is redundant and was omitted. **Revisit IF D5b `section` nodes embed their body** (then a body hash would matter for sections). |
| **D2 — per-file out-edge reconcile (lands with D3)** | D2 made the folder-derived `covers` set replace-not-append (`replace_edges_of_kind`). The per-file source edges (`calls`/`imports`/`extends`/`references`) also need replace-not-append so a removed call/import drops its stale edge — but TODAY `process_file` still does `delete_nodes_by_file` (cascades the file's out-edges) then re-inserts, so they're already replaced via cascade. Once **D3** switches `process_file` to upsert-then-prune (surviving nodes keep their id, so their out-edges are NOT cascade-deleted), add the per-file replace: `replace_edges_of_kind` scoped to the file's node ids (or a `replace_edges_of_source(folder, source_ids, edges)` variant). Do it as part of D3. |
| **D5b rationale — code-comment source + finer parenting (follow-up)** | D5b wired `rationale` nodes (NOTE/WHY/HACK/TODO/IMPORTANT) from the DOC path only (`doc::process` → `extract_rationale_pub`), parented to the file node. The spec also wants rationale from CODE comments (`function → rationale`) and finer parenting to the enclosing function/section. Deferred as a forward-only follow-up: extract rationale in the router (single DRY point, covers all text files) and resolve each marker's line to its enclosing symbol/section for `parent_id`. The P1 Done-gate (`count(rationale) > 0`) is met by doc rationale; this adds coverage + nesting fidelity. |
| **Barrier tasks don't fail-closed on a watchdog ABORT (systemic, low-pri)** | A barrier task killed by the executor watchdog (`tasks/executor.rs` `tokio::time::timeout` drops the future) writes NO `folder_status` — so a folder can be left at `indexing` if the abort lands mid-handler. This is systemic to the whole chain (`ResolveEdges`/`BuildConnections`/`ProcessGitFolder`/`DetectCommunities`); the D4-review fix closed the ORDINARY-`Err` path for DetectCommunities (→ `failed`), but not the abort path. Well-mitigated today: bounded retry re-drives it, detect is now DB-only (fast, unlikely to hit the 600s cap), and boot-reconcile / `resume.rs` re-enqueues `indexing` folders on restart. Proper fix: on a watchdog abort, mark the folder `failed` (or have resume treat a long-stale `indexing` as recoverable) — a worker-robustness increment, not D4 scope. |
| **⚠ D4c DEPLOY GATE — reconcile the new `inference.communities.props` column onto the live DB** | D4c-ii added `props jsonb not null default '{}'` to `communities.ddl` (stores `props.source ∈ {'insight-copy','null'}` — description provenance, never a template). `communities` is `create table if not exists`, so a plain boot-time `dbd apply` will NOT add the column to an existing live `sensei` DB → `INSERT … props` fails. **Before/at deploy, run `dbd reconcile -d <live sensei url>`** (pre-release workflow) to ALTER-in the column — same class as the D1 unique-index gate and the line-127 missing-column findings. `sensei_test` already reconciled for CI. Folds into the same deploy pass as the D1 graph-clear gate. |
| **D2/D6d — build_connections should be the sole terminal `indexed` marker + fail-closed on covers** | `resolve_libs` marks `indexed` BEFORE `build_connections` runs the covers replace (barrier order: ResolveEdges→ResolveLibs→BuildConnections). So a covers-replace FAILURE can't truly fail-close — the folder is already `indexed`. Fix: make `build_connections` (the terminal barrier) the ONLY writer of `indexed`; `resolve_libs` should persist libs without marking indexed (needs a set-libs-only path, since `mark_folder_indexed` does both). Then wire the D6d checks: covers-replace failure → leave non-terminal (recovery re-drives), and "0 covers but ≥1 doc+code stem match" → `failed`. The atomic `replace_edges_of_kind` already prevents the zero-covers-on-crash case; this is the remaining ordering/fail-closed polish. |
| **W3 — tolerated parse errors should advance `scan_state`** | inc5 (D6c-trigger) implemented the fatal side (a fatal DB write → `Err` + folder `failed` + `scan_state` NOT advanced). The spec's W3 also says a **tolerated** parse/read error should keep `Ok` **and advance `scan_state`** so a persistently-unparseable file isn't re-parsed on every scan. Currently the tolerated path (binary skip / parse error / parser panic) returns `Ok(0)` early **without** writing `scan_state` (pre-existing behaviour, unchanged by inc5) — so such files are re-parsed each scan (wasteful, not incorrect). Land the tolerated-advance (write the fingerprint on the tolerated-skip paths) with a test; low-risk, deferred from inc5 to keep that increment scoped to the load-bearing fatal/fail-closed behaviour. |
| **Fault-injection seam for `build_connections` status-read error** | inc5's shared fail-closed writer (`helpers::mark_folder_indexed_fail_closed`) declines to mark `indexed` when `get_folder_status` itself errors, but that branch has no test (no seam to force a read error). The recovery re-drive (a `failed`/`indexing` folder is re-driven next scan) self-heals a transient read blip, so it's low-risk; add a seam + test when convenient. Also: an end-to-end `fatal_file_failure_is_recorded_and_retried` driving the `process_file` fault seam through the real worker (asserting `task_executions.retry_number` increments for `ProcessFile` specifically) — currently the retry wiring is proven generically via `ProcessGitFolder`. |

### FQN symbol-table rebuild — Phase 3 deferrals (2026-08-07)

Plan [`plan/2026-08-06-fqn-symbol-table-plan.md`](plan/2026-08-06-fqn-symbol-table-plan.md). Phases 1–3 shipped (FQN core + Rust producer + emit wiring + partial identity index + language-scoped fallback). Deferred out of Phase 3:

| Item | Detail |
|---|---|
| **0.5 demote-to-stub prune (FQN nodes)** | The plan's 0.5 decision — a removed-but-still-referenced FQN def should DEMOTE to a stub (`resolved=false`, clear file/line/signature, keep `fqn`), not be deleted — is NOT yet wired. Today `prune_file_nodes` deletes a vanished def; for an FQN def its inbound `calls` edges (target_name NULL) then cascade-delete (edges FK is `on delete cascade`) rather than survive as unresolved. Low severity in practice: compiling code doesn't hold dangling calls to a removed symbol (the caller is edited + re-scanned too), and a full reindex heals it. Proper fix: a prune variant that demotes an inbound-referenced fqn-bearing node to a stub instead of deleting. Do it with Phase 5 (D5c) or Phase 7. |
| **⚠ DEPLOY GATE — `nodes_unique_identity` constraint → partial index** | Phase 3.0 converts `nodes_unique_identity` from a table CONSTRAINT to a partial unique index (`where file_path is not null`), and switches `upsert_node_ex`'s `ON CONFLICT` to column inference. A live DB currently has it as a constraint; a plain boot-time `dbd apply` won't drop the constraint + create the index. **At deploy, `ALTER TABLE sensei.nodes DROP CONSTRAINT nodes_unique_identity; CREATE UNIQUE INDEX … NULLS NOT DISTINCT WHERE file_path IS NOT NULL;`** (or `dbd reconcile`). Behaviour-preserving (every live row has a non-null file_path). Folds into the same graph-clear deploy pass as the `nodes_unique_fqn` gate (plan 0.6). `sensei_test` already migrated for CI. **Applied to the live `sensei` DB 2026-08-07** ([`plan/2026-08-07-fqn-live-migration.sql`](plan/2026-08-07-fqn-live-migration.sql)) as part of the Phase-7.3 scoped verify (below).** |

### FQN symbol-table rebuild — Phase 7 status + deferred full live migration (2026-08-07)

Phases 1–7 are **code-complete on `develop`** (not merged to `main`): `resolve_edges` + the interim guard `2c520f2d` are retired (7.1, commit `243e4fc5`); `graph/nodes` projects `fqn`/`resolved` (7.2, `e2ead815`). Phase 7.3 was run as a **scoped verify** on the live `sensei` DB (one repo) — the FQN deploy gates were applied surgically ([`plan/2026-08-07-fqn-live-migration.sql`](plan/2026-08-07-fqn-live-migration.sql)), the develop-HEAD daemon installed, and the sensei monorepo reindexed: the `new` mega-hub (1230 inbound) collapsed across **308** FQN nodes (worst 105), **677** first-class `lib_symbol` deps, file→type→method nesting, folder → `indexed`. **The live daemon is now the develop debug build** — it must stay until the release (an old release binary would write NULL-fqn nodes against the migrated schema).

| Deferred item | Detail |
|---|---|
| ~~**Full-graph live migration**~~ ✅ **DONE 2026-08-07 (v0.7.0 deploy).** | `develop`→`main` merged (`5e83fcca`), released `0.7.0` (`make bump v=minor`), release binary installed. Live deploy gate applied: graph-cleared (`TRUNCATE sensei.nodes, sensei.edges CASCADE; TRUNCATE inference.communities; TRUNCATE sensei.scan_state;` + all folders → `discovered`) + `dbd reconcile --scope default` (additive-only) → full reindex of all 8,664 folders running on the 0.7.0 daemon (multi-hour; watch roots `~/Developer` + `~/Work`). Verified starting clean: FQN nodes emitting, no schema errors. |
| ~~**`edges_target_id_idx` missing on live**~~ ✅ **DONE** — added by the v0.7.0 `dbd reconcile`; degree-recompute (in `detect_communities`, 7.1) is index-backed again. |
| **Unresolved-calls residual vs plan's "tiny"** | On the sensei repo, 43.5% of `calls` stay `target_id IS NULL` — the honest dyn / out-of-0.7-binding residual (trait dispatch, chained/reassigned receivers), NOT false edges. The plan's "unresolved = tiny (dyn only)" was optimistic; 0.7 deliberately stubs out-of-scope receivers. Tightening this (more binding forms / light type inference) is a future increment, not a regression. |
