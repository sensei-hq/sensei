# Roadmap vs. state — 2026-10-27

A read-only check of what the roadmap plans against what the code and the running daemon actually do. It was
written before the screen-by-screen feedback walk.

**Sources of intent:** `docs/prd/01–03`, `docs/prd/modules/**`, `docs/roadmap/01–09`, `docs/backlog.md` and
`docs/CHECKPOINT.md`.

**Sources of evidence:**
- the code on `develop` at c33eaf67;
- `rg` over `crates/`, `app/`, `dojo/`, `packages/` and `database/`;
- read-only git;
- GETs to the local daemon on :7744;
- read-only `psql` on the daemon DB and the local dōjō DB;
- `gh issue list`, `gh run list` and `gh workflow list`.

Nothing was changed.

**Caveats.**
- **The running daemon is an overlay build, not v0.21.0 as tagged.** `/health` reports `0.21.0`, but
  `/opt/homebrew/bin/senseid` is `HEAD-4e1faed`, a build from the morning of 10-27. Afternoon commits are not
  live, so the "built, not installed" list in `docs/CHECKPOINT.md` holds: get_duplicates from qlty, Make
  default, sync rows by identity, Settings as its own window, and the others.
- **Timings are unreliable.** The machine's load average was about 200 while I probed: a VM was using
  1,300% CPU, and four review agents were querying the daemon. One `GET …/diagrams/codemap` timed out at
  60 s; another call earlier returned 60 blocks. Read every latency here as an upper bound.

Status words: **done** means merged to develop with code and tests. **Partial** means part of it exists.
**Not started** means no code. **Dropped** means removed, or replaced by something else.

---

## 0. The headline before the tables

1. **The roadmap's own status cells were never filled in.** All 30 module rows in
   `docs/roadmap/01-foundation.md` through `05-platform.md` still read **"🚧 Phase 3"**.
   `06-gaps.md` still says "Remaining population (Phase 3)". The planes therefore carry no evidenced status
   at all, and the tables below are the first. Wherever a plane file and this review disagree, the plane
   file is simply empty.
2. **`docs/backlog.md` is dated 2026-10-13 and lists about 14 issues. GitHub has 87 open.** This is the same
   defect `07-quality.md` G1 recorded ("the backlog said 12 open issues; there are 67"), recurring.
3. **The burndown expected waves 0–4 to close about 40 of 89 issues. 87 are open today.** Many issues did
   close, but new ones were filed as fast. Waves 6–8 (indexer quality, metrics confidence, epics) have
   **zero commits** that reference their issues since 2026-10-13.
4. **Today's last test commit is red in CI.** Run 40588554951, "dōjō database", failed on 319de1dc:
   `definer_functions_gate_the_caller.sql:91: ERROR: listed but no longer defined: upsert_repository_metrics`
   (1 of 61 files). The Rust runs on develop since 3b8017ed were all *cancelled* (superseded), so no green
   run covers today's afternoon work.

---

## 1. Planned capability → status, by area

### 1.1 Foundation (M1–M6)

| Capability | Status | Evidence | Note |
|---|---|---|---|
| **M1** Six-gate health check (brew, pg, ollama, components, db, daemon) | done | `GET /health` → every component `ready` | |
| M1 `GET /api/bootstrap/status` state machine (spec surface) | dropped / moved | `curl` → 404; no such route in `routes.rs` | Replaced by `/health` and the Tauri `health_check*` commands (`app/src-tauri/src/commands/bootstrap.rs`). The spec still names the old route. |
| M1 Per-component resolvers, bounded waits | done | `crates/bootstrap/src/health/resolvers/*.rs`, `health/process_util.rs output_with_timeout` | The spec's path `crates/bootstrap/src/resolvers/` is wrong |
| M1 Hardware tier → model provisioning | done | `crates/bootstrap/src/hardware.rs`, `models.rs`; `/api/gateway/models/provision/status` | |
| **M2** Preference panes | done (reshaped) | 11 panes under `app/src/routes/(preferences)/settings/`; own window in 2677767a (not installed) | The spec has 8 panes, including Profile and Assignments. Neither exists as a pane. |
| M2 `GET/PUT /api/preferences`, cues, restore-default | not started | `/api/preferences` → 404, `/api/preferences/cues` → 404. Preferences live in `/api/config` `setup.preferences`. | |
| M2 Search across every setting (F3: "searchable") | not started | No search in `SettingsSidebar.svelte` | Objective F3 is not met |
| M2 First run as a single gate | partial | `(config)/setup/{welcome,roots,assistants,scan,done}` | A 5-step wizard rather than "value before setup" (F1) |
| M2 Kanji ⇄ icons (A-1) | done | `ui.glyphs`, `settings/general/+page.svelte` | |
| **M3** Claude Code live capture (hooks and mods) | done | `/hook/event`, `/hook/gate`, `/hook/nudge`; 194 Claude sessions live | C-13 (mods migration) is still unverified |
| M3 Six transcript adapters | done | `crates/senseid/src/transcript/{claude,opencode,copilot_cli,vscode,cursor,zed}.rs`; `assert_eq!(ads.len(), 6)` | Live sessions: zed 176, claude 194, opencode 27, cursor 1 |
| M3 Extension seam (C-14) | done (code) | `trait TranscriptAdapter` (`transcript/mod.rs:369`) | **The gap register still says "seam unspecified"** |
| M3 Copilot tool_success / turn time / codeChanges (C-16..18, C-20) | not started | `SynthEvent` has no success field | |
| M3 Copilot cache tokens (C-19) | done | `copilot_cli.rs:292`, test `parse_session_shutdown_tokens` | **The register still lists it open** |
| M3 Cost beyond Claude (C-25), Cursor data (C-24) | partial | Zed cache tokens `zed.rs:177`; 1 Cursor session | |
| **M4** Per-language adapters | done | `indexer/lang/{rust,csharp,java,python,kotlin,php,c,sql,javascript,svelte,vue,…}` | No Swift |
| M4 Placement floor ratchet | done | `.github/workflows/placement-floor.yml`, nightly and green | |
| M4 Resolution target (07-quality A4) | not started | Still "40.3%, no target"; #146 ("call resolution is 18%") is open with 0 commits | |
| M4 Incremental re-index | done | `watcher/root_watcher.rs` → `incremental::owning_root` | The header of `indexer/incremental.rs` still says "no caller until cutover" |
| M4 Ignore mechanism (G-2) | partial | `.gitignore` and `.ignore` honoured (`indexer/repo.rs:278`); watch-root exclusions | No stored or synced ignore config, and no qlty seeding. **The register says "no ignore mechanism", which overstates the gap.** |
| M4 config/generated file classes | not started | `repo.rs classify` | `analysis::codemap::file_role` derives roles at read time |
| **M5** Chains, adapters, consensus, local model | done | `/api/gateway/chains`; `/api/gateway/status` lists 10 adapters; `/api/gateway/consensus` | |
| M5 Budget cuts, breaker surface | not started | `budget: None` (`handlers/gateway.rs:112`); the status has no `.healthy`/`.chains` | The spec's done-check would fail |
| M5 `inference_calls` cost ledger (G-COST-LEDGER) | not started | Absent from `database/ddl` | |
| **M6** Declarative DDL; deploy and upgrade tested | done | 487 `.ddl` files; `crates/bootstrap/tests/deploy_upgrade.rs` | |
| M6 SQL planned against the deployed schema in CI; retention | done | rust.yml "Plan every SQL statement…"; `tasks/activity_pruner.rs` | |

### 1.2 Insight (M7–M13)

| Capability | Status | Evidence | Note |
|---|---|---|---|
| **M7** Analyzer scheduler, signals, recommendations, communities, narration | done | `tasks/analyzer_scheduler.rs`; `GET /api/tasks/scheduled` (16 tasks); `inference.recommendations` 27,674 rows; `/api/observatory/tool-signals` 200 | |
| **M8** Registry, value store, roll-up views, status, health score | done | `/api/metrics/registry` (32 active); `repository_metrics.ddl`; `workspace_health_score.ddl`; `/api/metrics/status/summary` (157 repos) | The spec still names `project_*` views and `/api/projects/…`, none of which exist |
| M8 Master-detail model and `gating` field (M-7) | not started | No parent/detail or gating column in `metrics.ddl` | P1 in the register, and it blocks T3 |
| M8 Coverage-gated presentation (M-8) | partial | `workspace_health_trend.coverage` exists; `weightCoverage` in `app/src/lib/metrics/health-radar.ts` is used only by specs | The DB half exists; no screen reads it |
| M8 Per-metric design/coverage record (M-9) | not started | No table or column | |
| M8 Confidence chain #158 / #159 / #160 | not started | Issues open, 0 commits since 10-13 | Wave 7 has not begun |
| M8 Architecture family #156 | partial | Registry `architecture` family: `graph_confidence`, `public_surface_ratio`, `symbol_size_p95` | Layer violations, cycles and zones are computed (`analysis/layering.rs`, `/diagrams/zones`) but **only as diagrams, not as metrics**, so T3's gates stay blocked |
| **M9** Memory lifecycle, HTTP curation, MCP save/propose/promote | done | `memories.ddl`; `routes.rs:486–499`; MCP tool list | |
| M9 `report_memory_use` / `report_learning` (G-MEM-FEEDBACK) | not started | Not in `crates/mcp/src/lib.rs` | |
| M9 user/org rungs (G-MEM-SCOPE) | partial | `memory_scope` enum lacks them, but promotion to user/organization works through namespaces (`knowledge.rs resolve_target_namespace`) | **The register overstates this gap** |
| **M10** Pattern engine (`crates/senseid/src/patterns/`) | not started | The directory does not exist; `POST /api/patterns/{ws}/detect` only lists stored rows | The 681 `detected_patterns` rows are all behavioural (rework, corrections) |
| M10 `pattern_choices` / registries (G-PATTERN-STORE) | not started | No DDL | |
| M10 Duplication | partial | `code_duplicates.ddl` + qlty scan (140e28d7, not installed; live table 0 rows) | |
| **M11** Doc drift | partial (inert) | `analysis/doc_drift.rs`; live `…/drift` → `{"items":[],"indexedDocs":0}` | No markdown adapter (#292, decided 10-20, no code), so nothing is ever scanned |
| M11 `trace_links`, `get_trace_links`, apply-fix | not started | No DDL, no tool, no route | |
| **M12** Libraries: see §2.3 | partial | | |
| **M13** Rule resolution, scope ladder, enforcement, packs | done | `pg_store/governance.rs resolve_rules_raw`; `sensei.scopes` general 0 < user 10 < organization 20 < workspace 60 < repository 70; 14 seeded packs | |
| M13 Checker | partial | `checker.rs`, `run_checkers`; `rule_check_runs` 0 rows live | |
| M13 `rule_violations` / `governance_overrides` (G-GOV-AUDIT) | not started | No DDL | |

### 1.3 Delivery (M14–M22)

| Capability | Status | Evidence | Note |
|---|---|---|---|
| **M14** MCP tool catalog | done | 56 tools in `crates/mcp/src/lib.rs`; 31 of 32 read-only tools match an oracle (`docs/baseline/2026-10-27-app.md`) | The README says "~40" |
| M14 `context_pack`, `search` | done / partial | `query.rs context_pack`; `search` fuses ILIKE and embeddings by RRF | No full-text search (D-8's remaining half); 55% of nodes have embeddings (the README says 26%) |
| M14 L0–L3 resolution levels, per-session dedup log | not started | No `node_summaries` / `session_context_log` DDL | |
| **M15** Today's one thing | done | `GET /api/observatory/today` | **The hero has an empty receipt live** (`source:""`, `impact:null`), so O3 "every claim carries a receipt" is not met |
| M15 Consolidation | done | `/api/knowledge/rules/consolidate[d]`, app `(observatory)/consolidation` | **The README says mock-only** |
| M15 Insights Reasoning | partial | No `/insights/{id}/reasoning`; `MoeReasoningPanel` on Impact | |
| M15 Sessions list | done, with a defect | `GET /api/sessions` hard-codes `toolUsage: []`, `benchmarkPairs: []` (`handlers/sessions.rs:57`) | A fabricated empty result; this breaks the "never fabricate" hard rule |
| **M16** Overview, Sessions, Memories, Traceability, Libraries, Instruments, Patterns, Impact, About | done | Routes under `(workspace)/workspace/[id]/` | Also built but unlisted in the README: Intake, Metrics, Tickets, Integrations |
| M16 Tasks section; solution-track roll-up (C-7) | not started | No route | Waits on M20 plan tables |
| M16 Dōjō binding (G-DOJO-AUDIT) | partial | `POST /api/workspaces/{id}/dojo-binding` exists, with no GET and no audit | **The register says no endpoint** |
| **M17** Code map: see §2.1 | done (desktop) / not started (dōjō) | | |
| **M18** Instruments: see §2.2 | done except benchmark | | |
| **M19** Verdicts, per-workspace impact, accept/reject | done | `verdicts.rs`; `/api/workspaces/{id}/impact`; `impact_verdicts.ddl` | |
| M19 `insights` / `applied_recommendations` / `impact_regressions`, `GET /api/impact` | not started | No DDL; `/api/impact` → 404 | The loop close "applied → measured → reinforced" (vision) is absent |
| **M20** Planner v0, plan graph, register plan | done | `/api/planner/generate`, `plan_graph.rs`, `/api/runs/plan` | |
| M20 Plan tables, baseline FTR, ratchet | not started | No DDL | |
| **M21** Run engine, watchdog, dōjō gate | done | `advance_run.rs drive_run`; `dojo/gate.rs` (36 tests) | `SENSEI_RUN_DRIVE` defaults to off |
| M21 ACP and fallback drivers | partial (unreachable) | `relay_drivers/mod.rs driver_for` always returns `ClaudeDriver` | **The README says "three relay drivers built"**; only Claude is reachable. ACP-1..6 open. |
| M21 Mobile relay (PWA, push) | partial | dōjō `/you/(inbox)/runs/[run_id]`, `service-worker.ts`, VAPID | **The README calls it greenfield.** It is unverified in production (D27). |
| **M22** Agentic execution | not started | No `agents/` directory, no `agent_runs` DDL | Design-only, as documented |

### 1.4 Team (T1–T6)

| Capability | Status | Evidence | Note |
|---|---|---|---|
| **T1** Membership, tenancy, federation wire, rules pulled down | done | `/v1/t/*/rules/resolved` pulled by `dojo/client.rs`; `tenancy_access.sql` | Production unverified (#314) |
| T1 Knowledge travels up (contribute → triage → approve, k ≥ 3) | partial (never delivered) | `dojo/contribute.rs`, `k_anonymity_min.ddl`; live `sensei.dojo_outbox` 1 row `held` since 2026-08-29; all memberships `sync_status=authenticating`, `last_seq 0` | #184 is open: this path has never sent a row |
| **T2** Four role consoles | partial | `dojo/src/lib/nav.ts` role-gated groups; lead → Teams (R8) | The org "Scopes" section loads nothing (`[section]/+page.ts:90`) |
| **T3** Workspace planning against a tracker, gating on code health | partial | Daemon Jira adapter `crates/senseid/src/tracker/`, several boards (R9) | No GitHub Issues or Azure Boards adapter; no gate targets or ratchet code; 0 trackers live |
| **T4** Billing and seats | partial | `billing_accounts.ddl`, `seats.ddl`, `/v1/t/*/billing` | No payment provider. **Nothing writes `dojo.seats`**, yet the PRD calls metering built. |
| **T5** Releases, DORA, release gates | not started | No `deployments` / `production_incidents` / DORA in DDL or code | Matches the doc |
| **T6** Workspace viewer | partial | `workspace_viewers`, `dojo_viewers`, `viewer_audit.sql` | No `/v/{tenant}` shell; no expiry, export flag or relationship (D15) |

### 1.5 Platform (P1–P3)

| Capability | Status | Evidence | Note |
|---|---|---|---|
| **P1** Homebrew formula and cask, `.dmg` | done | `homebrew/Formula/sensei.rb`, `release.yml` | |
| P1 Linux tarballs | done | `release.yml` x86_64/aarch64-unknown-linux-gnu | Linuxbrew `service do` is the only Linux service path |
| P1 systemd service, Windows build and service, `install.sh` | not started | No `systemd` outside docs; no Windows target in the matrix; no `install.sh` | P-5 |
| P1 Developer ID signing (#282) | not started | Ad-hoc signed; tomorrow's agenda item 1 | This is what revokes Keychain approval on every install |
| **P2** Marketing site | done | `website/src/routes/{+page,sensei,torii-seiki,privacy,terms}` | |
| P2 Only true claims (D-11) | not met | `website/src/routes/sensei/+page.svelte:31-32` offers `Sensei_x86_64-setup.exe` and `sensei_amd64.AppImage`, which no release builds; L97 and L350 list Codex and Aider | Codex and Aider can reach the MCP, but there is no capture adapter for either |
| **P3** Rust fmt, clippy, audit, tests in CI; branch protection | done | rust.yml `lint` + `test`, required on main | |
| P3 App and dōjō unit tests in CI | not running | They run only in `coverage.yml`, which is `disabled_manually` (`gh workflow list`) | **The P3 README says they "run on every PR and are green"** |
| P3 e2e in CI | not started | No Playwright step in any workflow | e2e runs only locally (`test:inspect`) |
| P3 Release from main tags | done | `release.yml` on `v*`, `scripts/release-guard.sh` | The README's "not fully automated" is stale |

### 1.6 Quality (`07-quality.md` A–H)

| Item | Status | Evidence | Note |
|---|---|---|---|
| A1–A3 (node adoption, cost capture, cache-read tokens) | done | #175, #176, #177 closed | |
| A4 Graph 40.3% resolved, no target | not started | #146 open, 0 commits; no per-language target published | |
| B1 Unbounded parse stack | done | #178 closed, `oversized` skip (2e5c6c1f) | |
| B2 Watchdog abandonment routine | done | #188 closed; `GET /api/tasks/abandoned`, `sensei doctor` Watchdog | #336 (one 1.28 GB transcript) landed, waiting on a week of releases |
| B3, B4 Pre-bind exit, Full/Degraded states | done | #179 closed; B4 marked fixed | |
| C1, C2 `public.logs` 25 GB, accumulating backups | done | #180, #181 closed | #339 (31.7M duplicate task log rows a month) is the new C-class item |
| C3 `make install` runs `cargo clean` | open (by design) | `Makefile:827 cargo clean` | |
| D1 No Rust CI | done | rust.yml | |
| D2 DB tests not isolated | done | #183, #204 closed (a database per test) | |
| D3, D3b e2e reds; retired hook contract | done | #134 closed; no `sensei-hook-dev` left in `app/e2e` | |
| D4 e2e outside static gates | done (issue still open) | `tsc -p tsconfig.e2e.json` in `check` (18eb0cde) | #187 stays open "until the changed specs pass" |
| D5 Unpushed commits | done | `git status -sb` → `develop...origin/develop` | |
| E1 Enrollment never completed | **still open** | Memberships `authenticating`, outbox `held` (#184) | 07-quality's banner "works against the local dōjō" blurs E1: the repository and metrics path works, the federation path does not |
| E2, E3 IPv4/IPv6, local setup | done | #185, #186 closed | |
| F Dependencies | done, issue stale | rokkit ^1.9.1 and kavach 1.3.0 are current | #191 ("rokkit and kavach are behind") is still open |
| G1 Backlog says 12, there are 67 | **recurred** | `backlog.md` (2026-10-13) vs 87 open issues | |
| H Kotlin/Swift adapters, SQL flip, full index | partial | A Kotlin v2 adapter exists; no Swift; wave 9's full re-index not run (130 workspaces without a checkout) | |

### 1.7 Dōjō expansion (`09-dojo-expansion.md`)

| Item | Status | Evidence | Note |
|---|---|---|---|
| R1 Retire the hand-made dōjō | done | 7478fe8d (v0.16.0) | |
| R2 The dōjō says workspace, no self-hosting | done | 65eede4d, 5da293d5, f080e38a, e6c58885 | |
| R3 Sensei says workspace | done | Merge 32c00e69 (v0.17.0) | **The doc cites branch SHAs that are not on develop** (0ec8271c, 91894026, d20a5b72) |
| R4 The words (PRD, specs, guides, site) | not started as a slice | "project" still appears 87× in `docs/prd` and 9× in `website/src` | Partly legitimate uses of other systems' word |
| R5a GitHub import / R5b Azure via Entra / R5c Microsoft sign-in | done (unverified live) | 8a584e51, 18f9311c, ed078710 | R5b and R5c wait on the Entra app registration. **The doc still says R5a is "not merged".** |
| R6 Ladder dōjō → workspace → repository | done | `ladder_rungs_retired.sql` (v0.18.0) | |
| R7 No Clients section | done | Merge 67e48bad (v0.18.0) | **The doc still says "on branch … (not merged)"** |
| R8 Lead team metrics, Teams per role | done | 21a5d1a3; `lead_team_metrics.sql`, `team_metric_totals.sql` | 0 teams locally |
| R9 Trackers by level | done (unverified live) | aad7ed90 | **06-gaps T-1 and G-FUN-3 still say "one tracker per project"; #318 is open** |
| A1 One role ladder; A2 repository roles; A3 teams, invites, viewers | done | 5564841d; 3a0712d9…6aa2d690; f01422ee…734cb6ca | A2 is live locally: `repository_roles` 60 |
| A4 Workspaces are writable in the dōjō | not started | `/v1/t/*/workspaces` is GET only; `workspaces_write.sql` pins "the sync is the only creator" | Only "drill-in off fixtures" is done (#106) |
| B1 Dōjō → sensei workspaces, teams, roles | not started | `dojo_client` pulls no `/workspaces` or `/teams`; no `origin` column | |
| B2 Sensei → dōjō writes; B3 conflicts visible | not started | No code | |
| C1 Dōjō-owned connections | partial | `tenant_connections` (forge identity + Vault token) | No per-workspace tracker or CI connection. **06-gaps G-SEC-2 says "no secret store"; Supabase Vault is in use** (`forge_token_vault.sql`). |
| C2 Tracker adapters | partial (moved to the daemon by D22) | Jira only (`tracker/jira.rs`) | No GitHub Issues or Azure Boards |
| C3 Sensei reads the dōjō's mirror | not started / superseded by D22 | The daemon reads Jira directly | #317 (dedupe) open |
| D-1..D-4 Deployments, incidents, five DORA keys, release gates | not started | Nothing in DDL or code | #316 open |

---

## 2. The five core targets

### 2.1 Diagrams — the Code map (M17)

- **What the roadmap wanted:** the code graph legible from bird's-eye down to symbol (G-1, P1). That means L0
  modules (≤ 60 blocks), L1 files by role and L2 symbols, plus seven lenses: Structure, Wheel, Zones, Matrix,
  Cycles, Hidden coupling and Complexity. One map, embedded in the workspace window **and** in the dōjō's
  Workspace › Code health tab. The user's decision on 2026-10-27: the daemon computes the map and pushes it
  with the graph sync.
- **What exists:**
  - Daemon routes `…/diagrams/codemap`, `/codemap/module`, `/codemap/file`, `/codemap/cochange` and
    `/diagrams/zones` (`routes.rs:137-142`), with route tests (`routes.rs:1213-1476`).
  - The shared package `packages/codemap/` (CodeMap, ModuleMap, FileColumns, SymbolView, Matrix/Wheel/Zones
    lenses, specs) and the app route `(workspace)/workspace/[id]/diagrams/codemap`.
  - An e2e test where every lens draws against the real daemon (76963eea).
  - Live: cochange answers (`session-report/model ↔ parse`, together 19), zones answers 190 points, and
    `sensei.file_complexity` has 1,445 rows.
  - Atlas and World are retired (6a1e1de8).
  - G-1 is effectively closed, though `06-gaps.md` still lists it open (P1).
- **What is missing:**
  - **The dōjō embed and the daemon-pushed map: not started.** There is no `codemap` anywhere in `dojo/src`,
    `dojo-protocol` or `dojo_client`. The graph push that would carry it is **skipped for every repository
    right now**: `/api/dojo/sync-state` shows `repository_graph … @develop push skipped not_a_tracked_branch`
    for sensei, dbd, gateway and torii.
  - **The Layers diagram freezes the page** (baseline). The older Structure and Layers diagrams still sit
    beside the map, so the map has not yet replaced them as the PRD says.
  - Wheel cannot group by Communities: it waits on Fix 2 (M4), and #149 still awaits a decision. The
    Structure lens has no role bar at L0. Zones is illustrative until abstractness is defined per language.
  - **Decision drift:** `08-issue-burndown.md` records "Complexity is computed in the indexer … not qlty's
    findings" (7a352615, 08:26). Twenty-eight minutes later 3db9ceab built it from local qlty "on the user's
    direction". The M17 README follows qlty; the burndown line was never corrected.
- **Next concrete step:** profile and fix the Layers freeze, or retire Layers into the Structure lens. Then
  start the dōjō embed with the graph actually reaching the dōjō. Per D18, production defaults to the default
  branch, so either track `develop` as the development environment or check the push from `main`. After that,
  push L0 with the graph and render it with `@sensei/codemap` in the dōjō.

### 2.2 Instruments (M18)

- **What the roadmap wanted:** a Playground for any tool; Replay with per-call verdicts; the verdict
  classifier; the tools-health grid; per-tool signals. The benchmark runner moved to deferred epic #279
  (decision, wave 2).
- **What exists:**
  - The Playground lists the discovered MCP servers and runs `tools/call` through each
    (`POST /api/instruments/mcp-servers/{id}/call`). It also handles hand-registered servers (#343) and
    returns 56 sensei tools live.
  - Replay (`/api/sessions/{id}/replay`, test `a_verdict_shows_on_its_own_call_in_the_replay_timeline`).
  - Verdicts with `unknown` (#278), the tools-health grid (live: 84 built-ins registered, 20 invoked in 14
    days) and tool-signals.
  - Tool correctness is baselined: 31 of 32 read-only tools match an independent oracle
    (`app/e2e/tests-inspect/tool-oracles.ts`).
- **What is missing:**
  - The `get_duplicates` fix (140e28d7) is not installed. Live, it still times out (42 s seen today).
  - **24 tools are unverified:** the 19 that write and the 5 that spend on a model.
  - Replay scrolls as one page (UI walk 8). The Instruments switch is not rokkit's (UI walk 5).
  - The tools-health grid has no daemon test.
  - The fix for "Runs in" offering every workspace (3b8017ed) is not installed.
  - Benchmarks are deferred (#279), not silently dropped.
- **Next concrete step:** after the signed install (#282), rerun `bun run test:inspect -- tool-calls` so
  `get_duplicates` flips to right. Then run the 19 write tools against a scratch workspace (the user's call,
  baseline "What is open").

### 2.3 Libraries (M12)

- **What the roadmap wanted:** detect → **wrap** → query → watch. Unmanaged dependencies are detected, their
  docs indexed at the lockfile's version, and a library can be wrapped. `search_lib_docs` answers queries,
  upgrades are watched, and skills are generated for heavily used libraries. Manifests
  (`sensei.library.json`) are discovered both ways.
- **What exists:**
  - Detection of 209 libraries (`/api/libs`), with packages folded into their library (#341).
  - Manifest ingestion on self-scan and from dependencies (`tasks/handlers/discover_library.rs`).
  - rokkit 15 packages / 95 pages / 5 skills / 3 agents; dbd 2/50/1/1; kavach 13/15/4/2.
  - Six library MCP tools, all matching their oracles.
  - Version pins and OSV advisories daily (`libraries/version.rs`, `advisory.rs`).
  - Today's fix for ecosystem names and design.yaml filing (14535af6).
- **What is missing:**
  - **Only 3 of 209 libraries have any doc pages** (`pageCount > 0`). "Query" works only for libraries that
    ship a manifest.
  - **Wrap is not built:** no `wrap_library`, so the G-LIB-WRAP gap stands. Yet the app says sensei "wraps
    them" (`app/src/lib/components/settings/LibrariesSection.svelte:27`), and `wizard-state.svelte.ts:220`
    says "The daemon reads this" for `setup.libraries`, which no daemon code reads. That is UI walk item 4
    ("199 · sensei will wrap").
  - `.well-known` site discovery (#342).
  - Skill generation.
  - Version-aware doc queries (backlog).
  - An ambiguous bare name answers a bare 500 (baseline; the target is a 409).
  - **The M12 status block (2026-10-26) is stale in the other direction.** It lists as unbuilt what #341
    built.
- **Next concrete step:** make the Libraries screens say what is held (pages, skills, agents per library)
  and remove the "wrap" claim and the dead `setup.libraries` comment. Then decide with the user whether
  "wrap" means indexing a dependency's published docs (`llms.txt` via #342) or is dropped from M12.

### 2.4 Metrics (M8)

- **What the roadmap wanted:** every advertised metric computes, is rated, "and cannot lie by omission".
  That means a master-detail model, explicit gating eligibility and coverage-gated presentation (M-7/8/9,
  all P1), then the confidence chain (#159 → #160 → #158) and the #156 architecture family that unblocks T3's
  gates. In the dōjō: consolidated metrics, and per-person metrics for leads.
- **What exists:**
  - The registry (32 active) and the repository-day store.
  - The views, the status reader and the health score.
  - Tier-1 architecture metrics.
  - Sealed coverage on the watched repositories: sensei and dbd each show 26 sealed, 5 snapshot and 1 never
    computed (`coverage`).
  - The dōjō's My metrics (`/v1/you/metrics`), the lead's team metrics and the k-anonymous team totals (R8).
- **What is missing:**
  - **M-7, M-8 (UI half) and M-9, and #158/#159/#160: none started**, with zero commits since 10-13.
  - Cycles, layer violations and zones are computed for the Code map but are not registry metrics, so no
    gate can read them.
  - **The estate summary reads as alarming:** 3,952 `behind` ("check the metrics task for a failing group")
    across 154 repositories that nothing watches (baseline open item).
  - sensei's own `coverage` metric has never computed.
  - A ResizeObserver console error on workspace metrics.
- **Next concrete step:**
  1. The cheap, visible one: give unwatched repositories a `not_watched` reason, so the status stops
     reporting faults that do not exist.
  2. Then start wave 7 with #159 (the method tier on the registry). It is a column and a seed, and it is
     the base #160 and #158 build on.
  3. In parallel, register layering violations and cycles (already computed by `analysis/layering.rs`) as
     `architecture` metrics. This is the first of T3's blocked gates.

### 2.5 Sync with the dōjō

- **What the roadmap wanted:**
  - Knowledge travels up and comes back down (T1).
  - Workspaces, teams and roles authored on either side and synced both ways (B1–B3), with integrations
    owned by the dōjō (C).
  - Everything verified in production (F-3 / G-OPS-1 #314, P0).
  - "Sign in → sync must just work" (#301).
- **What exists today, live against the local dōjō:**
  - Sensei → dōjō: repository registration and `repository_metric` pushes. The 4 sensei-hq repositories
    synced 21:01–21:11Z today; 15 personal repositories last synced 10-20.
  - Dōjō → sensei: the sync plan, the forge role mirror (synced 18:40Z), memberships, resolved rules,
    preferences (glyphs) and relay replies.
  - Relay sessions push separately.
  - `/api/dojo/sync-state` reports 26 entities: 19 synced, 5 skipped, 2 errors.
  - Built today, not installed: sync records name their identity (ebcfe469); Make default (f5eaa279);
    Settings · Dōjō grouped by identity → day → repository (02444525). The live `sync_state.persona` column
    exists but 0 of 26 rows carry it.
  - Advisor hardening for the production dōjō is in DDL (0bbbc75c) but undeployed. Its new test file is red
    in CI (§0.4).
- **What is missing:**
  - **Production: every one of the daemon's 8 memberships points at `127.0.0.1:5173` or `localhost:5173`.**
    None points at dojo.sensei-hq.com, so nothing has been verified in production (#314, #240). This is why
    the production dōjō shows no workspaces. The baseline puts it down to Keychain, but even with Keychain
    approved this daemon does not target production.
  - The federation path has never sent: memberships stay `authenticating` with `last_seq 0`, and the outbox
    row has been `held` since 2026-08-29 (#184, open since wave 0).
  - The graph push is skipped (`not_a_tracked_branch`), so no code map and no graph-backed view in the dōjō.
  - B1–B3 and A4 are not started. Workspaces exist in the dōjō only as the sync files them.
  - Every reinstall revokes Keychain approval (#282).
- **A note on the baseline:** its line "the dōjō sync has failed every minute since 2026-10-22" is now only
  partly true. The `jerrythomas` identity pushed metrics successfully today. The two errors are for persona
  `default` and say the local dōjō could not be reached.
- **Next concrete step:** #282 signing, then the signed install. Then enroll this daemon with production
  (sign in to dojo.sensei-hq.com from Settings · Dōjō) and observe one repository and metric push land
  there. That closes the evidence gap in #314 and #240. Then decide #184: write `sync_status` from the sync,
  or retire the outbox path.

---

## 3. Drift

### 3.1 Work done that the roadmap did not plan

| Work | Evidence | Comment |
|---|---|---|
| Playground generalised to every discovered MCP server, hand-registered servers | cb7ca987, ef296c15, #343 | User-directed on 10-26 ("list every MCP"); reshapes M18 beyond its spec |
| `get_duplicates` stored from qlty's scan | 140e28d7 | A response to the baseline's 200 s timeout |
| Settings as its own window (⌘,), Settings · Dōjō by identity, Make default | 2677767a, 02444525, f5eaa279 | From the UI walk; not in M2's spec, whose 8-pane plan is now stale |
| Supabase advisor hardening (RLS on 13 tables, search_path, definer-function test) | 0bbbc75c, 319de1dc | Production security, not in any wave; the test is red in CI |
| dōjō database CI (`dojo-db.yml`), placement-floor nightly | caf47a19, `placement-floor.yml` | Good: real gates the roadmap did not list |
| Transcript-read consent per assistant; cross-platform `keyring` | `transcript/consent.rs`, Cargo `keyring 4.2` | |
| Demo prep: My metrics, community labels, Tokens without cache reads | 87bc0bbc, 00e84641, v0.21.0 | Pulled the dōjō metrics forward of T3 |
| Metric correlations, per-repository metric activation | `/api/metrics/correlations`, `/api/dojo/metric-activation` | |
| Image generation route | `/api/gateway/image/generate` | The M5 spec lists image as deferred |
| Terms of Service / Privacy pages | #330 | Forced by the Entra app registration |
| Unused benchmark tables and store functions | `benchmark_runs`, `benchmark_reports` (0 rows) | Groundwork for deferred C-5; dead in production |

### 3.2 Planned work silently abandoned or stalled

| Plan | Where it was planned | State |
|---|---|---|
| Populate the roadmap statuses (Phase 3) | `roadmap/README.md`, every plane file, `06-gaps.md` | Never done; every module still reads "🚧 Phase 3" |
| Keep `backlog.md` as the open-issue index | `backlog.md` (decision 9) | Frozen at 2026-10-13 |
| Wave 6, indexer quality (#146, #152, #147, #148, #229, #272, #192) | `08-issue-burndown.md` | 0 commits since 10-13; #146 says call resolution is 18% |
| Wave 7, metrics confidence (#159 → #160 → #158) | burndown; M8 | 0 commits |
| Wave 8 epics (#43–#47, #81, #91, #102, #105, #108, #111, #114, #115, …) | burndown | 0 commits |
| Wave 9 full re-index | burndown | Not run; 130 workspaces still without a checkout |
| Wave 10 language parity (#275, #276, #237) | burndown | About 1 commit |
| #292 markdown adapter (doc drift) | Decided 2026-10-20 | No code; traceability scans 0 documents |
| 09 A4 and waves B, C (dōjō-owned), D | `09-dojo-expansion.md` | None started. D22 moved tracker pulling to the daemon: a legitimate, recorded change. |
| R4, "the words" | 09 | No slice |
| Searchable preferences, cues, restore-default (M2, objective F3) | M2 README | Not started; `/api/preferences` 404 |
| `GET /api/bootstrap/status` | M1 README | Dropped for `/health`; the spec was not updated |
| ACP and fallback relay drivers | M21 (P5.2/P5.3) | Built but unreachable (`driver_for` → Claude only) |
| Complexity in the indexer | Burndown "Code map decisions" (2) | Reversed to qlty the same morning; the burndown was not corrected |
| App and dōjō unit tests in CI | P3 README | `coverage.yml` disabled (qlty out of minutes); now pre-commit only |

### 3.3 Doc statements the code contradicts (fix when touching the doc)

- **Done in code, still listed open:**
  - `06-gaps`: G-1, C-14, C-19, D-12, G-DOJO-AUDIT ("no endpoint") and G-SEC-2 ("no secret store").
  - G-2 and G-MEM-SCOPE overstate their gaps.
  - T-1 / G-FUN-3 still say "one tracker per project".
- **Built, but the module READMEs say otherwise:**
  - M15 calls Consolidation mock-only.
  - M21 calls the mobile relay greenfield.
  - M16 says Atlas and World ship.
  - M12's 10-26 status block lists #341's work as unbuilt.
- **READMEs that overstate:**
  - M21 says "three drivers built"; only Claude is reachable.
  - P3 says app tests run on every PR; they run in no CI workflow.
  - T4 says seat metering is built; `dojo.seats` has no writer.
- **Dead names:** the M7, M8, M11, M1 and M2 READMEs cite `/api/projects/…`, `project_*` views,
  `/api/bootstrap/status`, `/api/preferences` and `…/apply-fix`, none of which exist.
- **`09-dojo-expansion.md`:** R5a and R7 still say "not merged"; R3's SHAs are dead.
- **`CLAUDE.md`:** says senseid consumes `gateway-embedded`; `Cargo.toml` pins `sensei-gateway` v0.7.0.

---

## 4. The biggest gaps between the roadmap and the product as a user experiences it

Ranked by how much a user would notice.

1. **The production dōjō is empty, and sync has only been proven locally.** No daemon is enrolled with
   production (all 8 memberships are localhost). The federation outbox has never sent (#184). The graph does
   not reach the dōjō (`not_a_tracked_branch`). The dōjō cannot author a workspace (A4/B1). For the "sync
   with the dōjō" target, the user opening dojo.sensei-hq.com sees nothing of their work.
2. **Every upgrade breaks trust in the machine.** Ad-hoc signing revokes Keychain on every reinstall (#282),
   which breaks sign-in and sync. Twelve of today's commits are built but not installed, so nothing from
   today is verified live.
3. **Metrics do not yet say how far to trust them.**
   - There is no confidence, gating or coverage presentation (M-7/8/9, #158–160, none started).
   - 154 repositories that nothing watches read as faults.
   - Architecture health exists only as pictures, so no gate can use it (T3).
4. **Libraries promise more than they hold.** 3 of 209 libraries have docs, and the app says sensei "wraps"
   them, which no code does.
5. **The diagrams are not finished where users meet them.** The Layers diagram freezes the page. The old
   diagrams sit beside the Code map. There is no Communities grouping. There is no map in the dōjō, which
   the PRD names as the manager's L0 view.
6. **The learning loop the vision is built on does not close.**
   - There is no `applied_recommendations` / `impact_regressions` / `GET /api/impact` (M19).
   - There is no pattern engine (M10).
   - Doc drift scans zero documents (M11).
   - Today's hero shows an empty receipt (O3).
   - `/api/sessions` fabricates empty `toolUsage` / `benchmarkPairs` arrays.
7. **Status cannot be read from the docs.** The plane tables are empty, the backlog is two weeks stale, and
   about 20 doc statements contradict the code (§3.3). The website offers Windows and Linux installers that
   no release builds (D-11).
8. **CI does not cover the UI.** App and dōjō unit tests and e2e run only locally (`coverage.yml` disabled),
   and today's last commit is red in the dōjō database job.
