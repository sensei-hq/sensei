# PRD ↔ implementation adversarial review

**Date:** 2026-10-14
**Scope:** the whole `docs/` tree, with `docs/prd/` as the core — every module `README.md`
and every `scenarios/*.feature` under `docs/prd/modules/**`, checked against the shipped
code in `crates/`, `database/ddl/`, `app/`, `dojo/` and `packages/`.
**Question asked:** *for completeness and detail, and for any inconsistency with the
implementation. Where the implementation is better than the docs, the docs need updating;
where the docs demand behaviour the implementation lacks, the gap belongs under `roadmap`.*

This is a **documentation** review. It does not close backlog items and it does not write
product code. Every finding is a claim about a doc, backed by a file/line.

---

## 0. How this review measures truth

The PRD already declares its own truth markers (`docs/reorg.md` §6):

| Marker | Meaning |
|---|---|
| `# pinned: <test>` | a test **proves it today** — the comment names a test that exists |
| `# pending: <issue>` | no test yet; issue filed |
| `NO TEST` | accepted as unproven (must be justified) |
| `# gap: <gap-id>` | points into `roadmap/06-gaps.md` |

Status emoji (`✅ Built · 🟡 Partial · 🖼️ Mock-only · 📄 Designed · 💤 Dormant · ⛔ Gap`) sit
on module READMEs. A doc is **consistent** when its pins name real tests, its cited paths
resolve, its cited tables/tools/routes exist, and its status matches the code. Anything
else is drift in one of two directions:

- **DOCS-WRONG** — the implementation is different (usually further along); the doc must
  change to match reality.
- **IMPL-GAP** — the doc specifies behaviour the code does not have; the gap goes to the
  roadmap register (`docs/roadmap/06-gaps.md`, plus the per-plane files).

A third, benign class exists and must **not** be "fixed":

- **SELF-FLAGGED** — the module already carries a *Status honesty* blockquote that names
  the drift precisely. These modules are honest; they need no edit (only a check that the
  flag is still complete).

### Reproducibility

The mechanical passes below are reproducible with plain `git`-clean `grep`/Python over the
working tree (excluding `node_modules`, `target`, `dist`, `.svelte-kit`, `database/backup`):

1. extract every `pinned:` name from `docs/prd/**/*.feature` and test membership against
   the concatenated source of `crates/ packages/ app/ tools/ marketplace/ scripts/
   supabase/ database/ dojo/`;
2. resolve every backticked file path against the citing file's directory plus a set of
   known crate roots;
3. collect every backticked `<schema>.<name>` (schemas `sensei dojo history activity
   inference gateway staging`) and test membership against the `database/ddl/**` file
   inventory;
4. extract every `tool("name")` from `crates/mcp/src/lib.rs` and test the doc-claimed tool
   names against it;
5. check backticked route literals against `crates/senseid/src/api/` and the
   `dojo/src/routes` / `app/src/routes` trees.

> **Caveat on tooling.** Some shells in this environment redact repeated long identifiers
> in *display* output (e.g. printing `n` in place of `challenge_source_memory_for_rec`).
> All identifier-level claims below were re-checked by reading the files directly, not from
> piped console text.

---

## 1. Summary

| Pass | Universe | Findings |
|---|---|---|
| `pinned:` tests | 626 unique pins | **6** unresolvable — all in module 19 |
| Cited file paths | ~1 000 backticked paths | **39** with no resolution; of these ~20 are genuine, the rest are root-relative shorthand |
| Schema-qualified names | 32 not in DDL | **~18** asserted as live (DOCS-WRONG / IMPL-GAP) · **~14** already self-flagged |
| MCP tool names | 59 real tools | **6** doc-claimed tools do not exist |
| Route literals | — | **3** asserted routes absent from the daemon |

By plane, the drift concentrates in **02-insight** (M9, M10, M11, M12, M13) and
**03-delivery** (M16, M18, M19, M21), with hits in **01-foundation** (M1, M3, M5, M6),
**04-team** (T1, T2, T4) and **05-platform** (P1, P3).

---

## 2. Findings by module

Legend: **DW** = DOCS-WRONG · **IG** = IMPL-GAP · **SF** = already self-flagged (no edit
needed) · **conf** = confidence.

### 01-foundation

#### M01 Bootstrap & provisioning — DW (conf: high)
- `app/src-tauri/src/bootstrap_sidecar.rs` is cited as the Tauri sidecar
  (`README.md:37`). **No such file exists.** `app/src-tauri/src/commands/` contains
  `assistants.rs bootstrap.rs logs.rs mod.rs platform_info.rs repos.rs update.rs`; the
  word "sidecar" appears only in `app/src-tauri/src/lib.rs`, `log_collector.rs` and two
  command files — there is no dedicated sidecar module.
- `crates/senseid/src/api/handlers/bootstrap.rs` is cited (`README.md:112`). **Absent.**
  The API handler set has no `bootstrap.rs`.

#### M03 Capture & sessions — DW (conf: high)
- Scanner cited as `crates/senseid/src/scan/` and branch handler as `scan/branch.rs`
  (`README.md:33-34`). **`crates/senseid/src/scan/` does not exist.** Real scan entry
  points: `crates/senseid/src/indexer/scan_repo.rs`, `crates/senseid/src/tasks/handlers/scan.rs`.
- `crates/senseid/src/api/handlers/hooks.rs` cited as the assistant-event ingest
  (`README.md:32`). **Absent.** The real handlers are `sessions::ingest_hook_event`,
  `sessions::hook_gate`, `sessions::hook_nudge` in
  `crates/senseid/src/api/handlers/sessions.rs` (wired at `/hook/event`, `/hook/gate`,
  `/hook/nudge`).
- Adapters cited as `crates/senseid/src/adapters/{claude,zed,cursor…}/` behind
  `adapters/mod.rs`'s trait (`README.md:35-36`). The trait module is
  `crates/senseid/src/adapters.rs` (there is **no** `adapters/mod.rs`), and the only
  children under `adapters/` are `config` and `manifest` — **no `claude`, `zed` or `cursor`
  directories exist**. The adapter surface named here is aspirational.

#### M05 Inference gateway — DW (conf: high)
- `sensei.inference_calls` is the cost ledger (`README.md:21,47,64,76,81,95`).
  **No `inference_calls` table exists in any schema in this repo**, and the name appears
  nowhere in this tree (`.rs`/`.sql`/`.ddl`/`.toml`). The `gateway` schema holds only
  `fallback_chains fallback_chain_models models models_in_router providers routers` — no
  call/cost log. The gateway crate itself is an external git dep (`sensei-hq/gateway`
  v0.7.0, not vendored here), so if the ledger lives there it is **not** in the `sensei`
  schema as written. Either way the doc's `sensei.`-qualified ledger is wrong → **IG/DW**.
- Note the sibling files `consensus.rs · budget.rs · breaker.rs` (`README.md:31-32`) are
  **not** a finding: the same line says they live in the external `sensei-hq/gateway` repo.

#### M06 Storage & data model — DW (conf: high)
- "One PostgreSQL database, `sensei`, **at port 7744**" (`README.md:8`). The daemon is
  7744; **Postgres is 5432** (`crates/bootstrap/src/config.rs`: `DAEMON_PORT = 7744`,
  `POSTGRES_PORT = 5432`) — and the default DSN is built from `POSTGRES_PORT`.

### 02-insight

#### M09 Memory & knowledge — DW + IG (conf: high)
This module is built on three MCP tools and a table that **do not exist**:
- **Tools absent from `crates/mcp/src/lib.rs`:** `get_memories`, `report_memory_use`,
  `report_learning` (`README.md:15,26,37,96,119,154`). The 59 real tools include
  `save_memory/propose_memory/promote_memory`, but **no memory-read tool**. The canonical
  consumer described here ("the LLM … calling `get_memories(project=X)`") therefore does
  not exist as written. *(IG for the tool; DW for the name — but since nothing equivalent
  exists, this is primarily a real gap.)*
- `crates/mcp/src/tools/get_memories.rs` (`README.md:154`) — **no `crates/mcp/src/tools/`
  tree exists**; all tools are declared in `crates/mcp/src/lib.rs`.
- `crates/senseid/src/api/handlers/memories.rs` (`README.md:40`) — **absent.** The real
  handler is `crates/senseid/src/api/handlers/knowledge.rs`
  (`list_memories, get_memory, get_rules, save_memory, promote_memory, reinforce_memory,
  challenge_memory, dismiss_memory, merge_memory, generalise_memory, …`).
- `/api/memories?project=…` (`README.md:40`) — **no `/api/memories` route.** Real routes:
  `GET /api/projects/{id}/memories`, `GET /api/knowledge/memories/{id}`, plus
  `/api/knowledge/memories/{id}/{promote,generalise,archive,reinforce,challenge,dismiss,merge}`.
- `GET /api/observatory/memory-health` (`README.md:41,92`) — **no such route.**
- `activity.memory_loads` **does** exist (good) — but it can only be populated if a
  `get_memories`-style read path exists; today the read path is `/api/projects/{id}/memories`.

#### M10 Patterns & conventions — DW + IG (conf: high)
- `sensei.pattern_registries` and `sensei.pattern_choices` are asserted as tables
  (`README.md:33,66,68,97,113`). **Neither exists in the DDL.** The nearest real objects are
  `inference.detected_patterns` (table) and the `project_patterns` view. The "architectural
  choice persists and is read back next session" behaviour has no storage → **IG** (and the
  prose should stop calling them tables until built).
- `get_pattern_options` is claimed as an MCP tool (`README.md:32,46`). **Absent.** Real
  pattern tools: `get_patterns`, `get_pattern_for`, `match_pattern`,
  `get_project_conventions`.

#### M11 Traceability & doc-drift — DW + IG (conf: high)
- `sensei.trace_links` and `sensei.doc_lint_findings` are asserted as tables
  (`README.md:31,63,65`). **Neither exists in the DDL** → **IG**.
- `get_trace_links(node)` is claimed as an MCP tool (`README.md:41,76`). **Absent** — the
  real tool set has no `get_trace_links`.

#### M12 Libraries — DW + IG (conf: high)
- `sensei.library_pages` / `sensei.library_skills` (`README.md:26-27,44-47`). The DDL has
  **`sensei.library_content`** and **`sensei.library_packages`** (tables) plus the
  `libraries_in_project` / `project_libraries_resolved` / `library_usage` views — not
  `library_pages`/`library_skills` → **DW** (rename to the real objects) or **IG** if the
  described shape genuinely differs.
- `crates/senseid/src/mcp/library_pages.rs` (`README.md:36`). **`crates/senseid/src/mcp/`
  does not exist.** Real handler: `crates/senseid/src/api/handlers/libraries.rs`.
- `libraries/custom.rs` and `libraries/skills.rs` (`README.md:39-40`). The real
  `crates/senseid/src/libraries/` holds `advisory.rs docs_source.rs manifest.rs mod.rs
  registry.rs version.rs` — no `custom.rs`, no `skills.rs`.
- `wrap_library` claimed as an MCP tool (`README.md:35`). **Absent.**
- `package-lock.json` (`README.md:50`) is fine as a *generic* lockfile example (the repo
  uses `bun.lock`), but if read as "this file exists here" it misleads — retag as examples.

#### M13 Governance & rules — DW (conf: high)
- The enforcement/audit prose asserts four tables that **do not exist**:
  `sensei.rule_violations`, `sensei.governance_overrides` (`README.md:28,76,80,147,149`),
  `sensei.rule_ladders` (`README.md:61`) and `sensei.rules` (`README.md:187`, and M16:51).
  Real governance storage is: `sensei.memories` (a rule *is* a memory + `namespace_id`,
  `enforcement`, `origin`), `sensei.scopes`, `sensei.namespaces`,
  `sensei.consolidated_rulesets`, `sensei.rule_packs` / `rule_pack_rules` /
  `rule_pack_adoptions`, `sensei.rule_check_runs`, `sensei.stances`,
  `sensei.tool_call_verdicts`, `sensei.playbook_rules`.
  - The `rule_ladders` sentence is **self-aware** ("…is superseded by this") so it is
    historical, but the `psql` recipe at `README.md:187`
    (`select max(state_changed_at) from sensei.rules …`) is a **live command that errors** →
    must be repointed at the real table (`sensei.memories`).
  - The **override / violation audit trail is unbuilt** → **IG**. The wrong gate at
    `README.md:171` ("a `--no-verify` bypass without an override row never ships silently")
    is not dischargeable today.
- `crates/senseid/src/api/handlers/rules.rs` (`README.md:49`). **Absent.** Rules read/write
  lives in `api/handlers/knowledge.rs` (`get_rules`, `materialize_global_rules`) and
  `api/handlers/playbook.rs` / `stance.rs`.

### 03-delivery

#### M14 MCP context delivery — SF (no edit)
- The Status-honesty block (`README.md:23-29,147-157`) already states that
  `node_summaries`, `session_context_log`, `node_embeddings`, `node_fts` are **not in the
  DDL**, that `hybrid.rs` is absent, and that `search` is `ILIKE`-only ("D-8"). Nothing to
  fix — only ensure the D-8 pointer stays live in `roadmap/03-delivery.md`.
- Minor: `crates/mcp/src/server.rs` is cited (`README.md:29`) — the doc itself says it does
  not exist; the real server file is `crates/senseid/src/api/server.rs`. Keep, but it is a
  negative citation.

#### M15 Observatory — SF (no edit)
- Status honesty (`README.md:20-24`) already flags `sensei.ftr_daily` (absent; FTR rolls up
  from `sensei.metrics`), `sensei.reasoning_traces` → `inference.reasoning_traces`, and
  `sensei.dojo_downstream_inbox` → `sensei.dojo_inbox`. Correct.

#### M16 Project window — DW (conf: high)
- `sensei.project_ftr_metrics` (`README.md:47,56`). **Absent** — no view or table by that
  name. The real view is `sensei.project_metrics` (the doc's own parenthetical).
- `sensei.project_libraries` (`README.md:49`). **Absent** — real view
  `project_libraries_resolved`.
- `sensei.promoted_patterns` (view `project_patterns`) (`README.md:50`). **Absent** — real
  view is `project_patterns`.
- `sensei.project_dependencies` (`README.md:50`). **Absent** — nearest real object is
  `folder_dependencies`.
- `sensei.rules` (`README.md:51`) — absent, as in M13.
- `/api/dojo-binding` (`README.md:43`). **No such route.**
- The "single source" claim "every FTR chip reads
  `sensei.project_ftr_metrics.ftr_14d`" (`README.md:55-56`) must be rewritten against the
  real view/column.

#### M17 Atlas — ok (conf: med)
- `atlas-graph.svelte.ts` / `atlas-view.svelte.ts` (`README.md:50-52`) resolve under
  `app/src/routes/(project)/project/[id]/atlas/` — the docs give the basenames, which is
  acceptable; no action.

#### M18 Instruments — DW (conf: high)
- `activity.tool_calls` (`README.md:47`). **Absent** — the real object is the
  `activity.session_tool_calls` **view** (and `tool_usage_stats`).
- `db/pg_store/tool_calls.rs` (`README.md:44,122`). **Absent** — tool-call persistence
  lives in `crates/senseid/src/db/pg_store/sessions.rs` (`tool_calls_tests.rs` tests it).
- The tools-health grid / MCP server listing have no dedicated test — the doc says so
  (`README.md:19-21`), which is honest.

#### M19 Insights & impact — DW (conf: high)
Six of the module's `pinned:` test names **name nothing that exists**. Behaviour is real;
the pins are stale. Repoint as follows:

| Stale pin (feature line) | Real proof |
|---|---|
| `positive_verdict_idempotently_promotes_reinforce_memory` (insights.feature:130) | `reinforce_source_memory_for_rec_bumps_promotes_and_is_idempotent` (`db/pg_store/tests.rs`) |
| `negative_verdict_idempotently_promotes_challenge_memory_with_revision` (insights.feature:136) | `measure_regressed_rec_challenges_source_memory_once` (ibid.) |
| `neutral_verdict_promotes_no_memory` (insights.feature:142) | no equivalent test → `NO TEST` |
| `narration_cache_key_is_sha256_of_kind_plus_canonical_json` (insights.feature:121) | `facts_hash_is_order_independent` (`analysis/narration_cache.rs`; `facts_hash` is literally `sha256(kind + canonical_json)`) |
| `get_project_impact_endpoint` (impact.feature:3) | handler is `get_project_impact` (`api/handlers/project_detail.rs`, `db/pg_store/patterns.rs`); no test of that name → `NO TEST` or pin the real handler test |
| `tool_insights_endpoint` (impact.feature:27) | handler is `observatory::tool_insights`; nearest test `analyzer_scheduler::global_passes_classify_before_aggregate_tool_insights` covers scheduling, not the endpoint |

Also `sensei.project_ftr_metrics` (`README.md:46`) — absent, as in M16.

#### M20 Planner & baseline — SF + DW (conf: high)
- `sensei.plans`, `plan_baselines`, `plan_tasks` (`README.md:37`) are **correctly** named
  as *absent* (`Out:` list). Keep.
- `plan_tasks` / `plan_baselines` appear again in prose; ensure they stay inside the "Out"
  box.

#### M21 Relay — DW (conf: high)
- `sensei.runs`, `sensei.run_events` (`README.md:54`). **Wrong schema** — both live in
  **`activity.*`** (`database/ddl/table/activity/runs.ddl`, `run_events.ddl`).
- `dojo.relay_messages`, `dojo.relay_gates` (`README.md:55`). **Absent** — the real relay
  tables are `dojo.relay_inbox`, `dojo.relay_sessions`, `dojo.relay_segments`.
- `dojo-protocol/src/relay.rs` (`README.md:22,150`) — real path is
  `crates/dojo-protocol/src/relay.rs` (missing `crates/`).

#### M22 Agentic execution — SF (no edit)
- Status honesty (`README.md:18-20,42-49,106-108`) already states there is no
  `crates/senseid/src/agents/`, no `sensei.agent_runs` table, no benchmark runner. Correct.

### 04-team

#### T1 Dōjō core — minor DW (conf: high)
- `dojo.contribute.rs` and `dojo.routing.rs` (`README.md:35,38`) are **typos** — the real
  Rust modules are `crates/senseid/src/dojo/contribute.rs` and `…/dojo/routing.rs` (slash,
  not dot). `attribution.rs` (`README.md:59`) likewise → `crates/senseid/src/dojo/attribution.rs`.

#### T2 Dōjō consoles — DW + IG (conf: med)
- `dojo.audit_ledger` (`README.md:82`). **Absent** — the real tables are `dojo.audit_log`
  and `dojo.audit_events`.
- `dojo-auth.ts` (`README.md:27,77`) resolves at
  `dojo/src/lib/server/dojo-auth.ts` — acceptable shorthand.
- `/console/teams`, `/you/{projects,inbox,relay,packs}` (`README.md:53-54,67-68,331`) exist
  in `dojo/src/routes/(app)/…` and `(console)/console`; treat as live.
- The org-context status ("`/org/[slug]` … unbuilt") must be re-checked: the routes
  `dojo/src/routes/(app)/org/[slug]/{+page.ts,+page.svelte,[section],projects/[id]}` **do
  exist**, so any claim that the org surface has no build contract is stale. *(This is the
  highest-value single edit in the team plane — see §4.1.)*

#### T4 Billing & seats — SF + check (conf: med)
- `dojo.subscriptions`, `dojo.active_contributors`, `dojo.invoices` (`README.md:43-45`) are
  explicitly under **"Data (planned)"**, and the module says "Nothing is built"
  (`README.md:18-20`). Keep — **but** the DDL *does* already carry `dojo.billing_accounts`
  and `dojo.seats`, so "no metering, no billing data at all" should be softened to name
  those two tables. `/org/new` (`README.md:36`) is not a route yet — already implied by
  "planned".

### 05-platform

#### P1 Distribution — DW (conf: high)
- `crates/bootstrap/health/platforms/windows.rs` (`README.md:13`) and `bootstrap.rs`
  (`README.md:43`) — the real paths are `crates/bootstrap/src/health/platforms/windows.rs`
  and the crate entry `crates/bootstrap/src/lib.rs` (there is **no** `bootstrap.rs`); the
  provisioning/upgrade logic lives across `lib.rs`, `engines.rs`, `upgrade.rs`, `database.rs`.

#### P3 Harness & CI — DW (conf: high)
- The module claims the **local pre-commit hook** runs the nine-step chain
  (`README.md:12-16,32-33,56-58`; `harness-ci.feature:480-483`:
  "fmt, clippy, sql-layer, grant-targets, ddl-comments, brew-tokens, dbd-pins, json-wire,
  deploy-upgrade all pass"). The hook is literally `check-no-leaks.sh` then `make test-fast`
  (`.githooks/pre-commit`), and `test-fast` is
  `check-sql-in-layer check-ddl-grants check-ddl-comments check-brew-tokens
  test-crates-fast test-app-unit` — **no fmt, no clippy, no dbd-pins, no json-wire, no
  deploy-upgrade**. The nine steps are real, but they live in **CI** (`rust.yml`), not in
  the local hook. The README conflates the two. → repoint the hook description at
  `make test-fast` and the nine-step chain at `rust.yml`.
- Test counts (`README.md:13` "1,962 tests"; `:21` app "1,764 tests") are point-in-time and
  already drifted: a local `make test-fast` reports the app unit suite at **1,781 tests**
  (129 files). Refresh or de-number.
- `cargo release` is named as the release driver (`README.md:27,34`) but there is **no
  `release.toml`** — releases are driven by `.github/workflows/release.yml`. (Low severity.)

---

## 3. What is *already* right (do not "fix")

- **M14, M15, M18 (tools-health), M19 (status block), M20, M21 (run engine), M22, T2
  (personal screens), T4** carry **Status honesty** blocks that name their own drift. These
  are correct and are the pattern the rest of the PRD should copy.
- **Pins are 99 % clean:** 620 of 626 resolve to a real test. The pin mechanism is working.
- **Module/scenario counts** in the index READMEs (29 module dirs, 5 planes, 19 delivery
  feature files) are accurate.
- **CI workflow** (`rust.yml`) does run the 9-step chain the harness-ci docs describe
  (fmt → clippy → SQL-layer → grants → DDL comments → brew tokens → SQL-against-schema →
  TS type-check → workspace tests → deploy/upgrade). The *chain itself* is real; only its
  attribution to the **local** hook is wrong (see P3 below).

---

## 4. Disposition

### 4.1 Doc edits to make (DOCS-WRONG → update docs)

Ordered by blast radius (how many downstream docs repeat the claim):

1. **Path prefixes (mechanical, high fan-out).** Fix the missing `crates/`/`src/` and the
   moved handlers: M1, M3, M9, M12, M13, M16, M18, T1, T4, P1.
2. **Schema-qualified table names.** Replace the phantom names with their real
   counterparts wherever the doc asserts them as live: M5 (`inference_calls`), M6 (port),
   M9, M10, M11, M12, M13, M16, M18, M21, T2.
3. **MCP tool names.** M9 (`get_memories`, `report_memory_use`, `report_learning`), M10
   (`get_pattern_options`), M11 (`get_trace_links`), M12 (`wrap_library`).
4. **M19 pins.** Repoint the six stale pins (table in §2).
5. **Routes.** M9 (`/api/memories`, `/api/observatory/memory-health`), M16
   (`/api/dojo-binding`).
6. **T2 org-context status.** Re-verify and correct if the org routes are now built.

Each edit is a one-line change plus, where the name was load-bearing, a sentence. None
require new product code.

### 4.2 Gaps to register under `roadmap` (IMPL-GAP)

These are behaviours the PRD specifies that the code does not have. They belong in
`docs/roadmap/06-gaps.md` (with a matching row in the plane file):

| New gap | Module | One line | Evidence |
|---|---|---|---|
| `G-MEM-READ` | M9 | no memory-read MCP tool (`get_memories`); session-start memory load can't happen via MCP | 59 tools, none read memories |
| `G-MEM-REPORT` | M9 | no `report_memory_use` / `report_learning` tool; the feedback loop is one-way | as above |
| `G-PATTERN-STORE` | M10 | `pattern_choices` / `pattern_registries` have no DDL; chosen options can't persist | DDL absent |
| `G-TRACE-STORE` | M11 | `trace_links` / `doc_lint_findings` have no DDL | DDL absent |
| `G-TRACE-TOOL` | M11 | no `get_trace_links` tool | tool absent |
| `G-GOV-AUDIT` | M13 | no override/violation audit trail (`rule_violations`, `governance_overrides`) | DDL + code absent |
| `G-COST-LEDGER` | M5 | `inference_calls` cost ledger has no table; the cost surface can't be real | DDL absent |
| `G-IMPACT-TEST` | M19 | impact + tool-insights endpoints have no endpoint-level test | pins unresolvable |
| `G-DOJO-AUDIT` | T2 | `dojo.audit_ledger` absent (audit_log/audit_events exist instead) | DDL |
| `G-BILLING-DATA` | T4 | billing tables (`subscriptions`/`invoices`/`active_contributors`) absent; `billing_accounts`/`seats` are the real, partial set | DDL |

(D-8 for `search` FTS/hybrid already exists in `roadmap/03-delivery.md`; no new row needed.)

### 4.3 Things this review did **not** do

- It did not close backlog epics (#85, #91) — that is implementation, not documentation.
- It did not attempt to prove *behavioural* correctness; it checks the PRD's own
  truth-markers (pins, paths, tables, tools, routes, status) against the tree.

---

## 5. Verification log

- Pins: 626 unique; 6 unresolvable — re-confirmed by direct read
  (`db/pg_store/tests.rs`, `analysis/narration_cache.rs`, `api/handlers/project_detail.rs`).
- Postgres port: `config.rs` `POSTGRES_PORT = 5432`, `DAEMON_PORT = 7744`.
- Org routes exist: `dojo/src/routes/(app)/org/[slug]/{+page.ts,+page.svelte}`.
- Gate hooks exist: `/hook/event → sessions::ingest_hook_event`, `/hook/gate`,
  `/hook/nudge`.
- MCP tool set enumerated from `tool("…")` declarations: 59 names.
- DDL object inventory: 376 files across `table/view/enum/function/procedure`.

_Reviewed against `develop` on 2026-10-14 (started at `dbe82713`; the tree advanced to
`f4b0788e` during the review — no finding depends on the difference)._
