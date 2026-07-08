# Vacation run — live state

**Purpose:** durable, cross-session checkpoint of the autonomous run driven by
`docs/llm-spec/EXECUTION-PLAN.md`. Any pickup (scheduled wakeup, phone session,
cold restart after a usage limit) reads THIS file first, then the plan, and
resumes from the current slot/gate. Updated after every gate step.

**Run started:** 2026-07-07
**Branch:** develop (never merge to main autonomously)
**Data:** real — daemon PID live on :7744, released binary. `/health` 200.

## Limit-resilience contract
- Transient 429s → harness auto-retries.
- Hard usage cap → harness pauses/resumes the turn; if the session is killed,
  resume cold from the last committed gate checkpoint below.
- Checkpoint cadence: update this file after every gate; commit+push per doc.
- Self-paced loop reschedules a wakeup at each pause so the run keeps advancing.

## Queue (from EXECUTION-PLAN.md)
1. Observatory · Today — `screen/observatory-today.md`   ✅ SHIPPED (commit 35a438ce, pushed develop)
2. Observatory · Instruments · Health — `screen/observatory-instruments-health.md`  ⛔ PARKED
   (data-model gap: registry↔usage don't join; no tools_registered for used MCPs → share_invoked
   uncomputable. See park/observatory-instruments-health.md. AWAITS Jerry: unify/redefine/descope.)
3. Observatory · Projects (list view) — `screen/observatory-projects.md`  ✅ SHIPPED (ead8f971)
4. Project window · Overview — `screen/project-overview.md`  ✅ SHIPPED (all 4 gates; P0-B drift-source fixed)
5. Observatory · Insights — `screen/observatory-insights.md`  ← CURRENT
6. Observatory · Sessions — `screen/observatory-sessions.md`
(overflow: 7 Observatory·Memories, 8 Project·Sessions+Memories)

## Current position
- **Slot 4 — Project window · Overview** — gate1 ✅ (2 rounds: not-ready→needs-fixes→all fixed;
  doc-drift source consistency + to_merge/casing recs applied without 3rd round). BACKEND
  (assembling endpoint) delegated to fork; then frontend overview pane; then gates 2/3/4; commit.
  Endpoint `GET /api/projects/{id}/overview` = NEW server assembler. Sources: project (get_project
  + project_ftr_metrics.ftr_14d + kanji from icon->>'value' + folders w/ folder_role), top_rec
  (top pending inference.recommendations w/ default_acp), stats (sessions7d+corrected, memory
  counts total/ready_to_share/to_merge via pipeline/memory statuses, doc_drift open from
  sensei.project_drift + referenced_docs=COUNT(DISTINCT doc_node_id) from drift_items),
  recentSessions (recent activity.sessions w/ role). camelCase wire. Mockup: project-lite-panes.jsx
  ProjOverviewLite. App: (observatory)/projects/[id]/.
- BACKEND ✅ VERIFIED: new `project_overview.rs` pure module (4 tests) + get_project_overview
  handler + route + pg_store helpers + main.rs. clippy clean (4 pre-existing warns, none new).
  Live /api/projects/{id}/overview: project{id,name,kanji,client,goal,ftr,warn,sessions7d,
  folders[{id,name,role,primary}]}, top_recommendation{id,title,why,evidence,defaultAcp}|null,
  stats{sessions7d,sessions7dCorrected,memories{total,readyToShare,toMerge},docDrift{open,referencedDocs}},
  recentSessions[{id,title,startedAt,completedAt,corrections,ftr,role}]. sensei: ftr0.2 warn true
  drift open1541. rokkit role"library". CAVEATS for frontend: role can be null (sensei), defaultAcp
  can be null, top_recommendation can be null (all-quiet state), sessions give startedAt/completedAt
  (derive duration/time). FRONTEND now (svelte-file-editor): [id]/+page.svelte is the overview pane
  (only +page.svelte exists, no loader/tabs yet).
- FRONTEND ✅ GREEN. ROUTE CORRECTION: real project window is `(project)/project/[id]/overview/`
  (NOT observatory/projects/[id] which is DEAD/legacy TabBar — candidate for deletion). Built there.
  check 0/0 (839 files), test 767 (745+22, 0 regress), MCP-validated. Files: types.ts (ProjectOverview),
  api.ts (getProjectOverview), overview-view.svelte.ts (+22-test spec), +page.ts (loader),
  +page.svelte (rebuilt to ProjOverviewLite). Handles null role/defaultAcp/top_recommendation
  (all-quiet 静). FTR consistency verified (project.ftr==/ftr). DEVIATIONS (accepted): kept
  Accept/Reject hero controls (only UI feeding MeasureVerdicts, has live e2e) + added send-to-acp;
  mockup shows only send-to-acp — note for Jerry. Minor: duration/time in 3 places → future
  src/lib/time.ts. Gates 2(done)+3(wrong) LAUNCHED @ correct route + /api/projects/{id}/overview.
- Gate 2 done-gate-verifier: all 12 curl/code gates PASS (partially-verified only for 4 Tauri-visual;
  FTR single-source confirmed sensei/rokkit). Gate 3 wrong-gate-hunter: CLEAN (multi-repo verified
  live documentation.wiki→"2 repos"). Flags→project-overview-followups.md: session-row nav→sessions
  list not Replay (Replay screen UNBUILT — working link, defer); readyToShare/toMerge=0 (no
  memory_status promotion value); Accept/Reject kept (e2e/MeasureVerdicts); time DRY; legacy dead
  route. None block. Gate 4 persona LAUNCHED → then commit Slot 4 + push → Slot 5.

### Slot 4 detail (superseded)
- **Slot 4 — Project window · Overview** — gate1 `not-ready` (4 FAIL, all field/enum naming,
  FEASIBLE — reviewer verified all tables/views exist). Spec fix DELEGATED; then re-review;
  then impl (new assembling endpoint `/api/projects/{id}/overview` + frontend overview pane).
  FIXES: (1) declare `/overview` a NEW assembling endpoint (server assembles project+
  top_recommendation+stats+recentSessions), map each key to source; (2) vision→`goal` (alias in
  API, no migration); (3) `scope_project_id`→`memories.project_id`+status filter; (4) role "web"
  →real folder_role enum (backend/frontend/library/tool/docs/infra/website/desktop/mobile/config/
  packaging); recs: kanji from `projects.icon`→>'value', referenced_docs=COUNT(DISTINCT doc_node_id)
  from drift_items, ready_to_share status → link pipeline/memory.
  Sources verified: project_ftr_metrics.ftr_14d, project_drift view, inference.recommendations
  (has default_acp), folders.role (folder_role enum).
  Plan notes: landing pane inside a project window; deps FTR ✅ + top recommendation (partial)
  + memory counts ✅ + doc-drift (partial); multi-repo folder-role chip on sessions is new.
  `GET /api/projects/{id}/overview` — assemble from existing pieces if absent. NOTE: plan said
  "vision column migration needed" — NOT needed, `sensei.projects.goal` already exists (used in
  Slot 3). Existing per-project endpoints: /ftr, /drift, /patterns, /libraries, /instruments,
  /memories, /recommendations, /impact, /sessions, /maturity, /quality-signals, /hotspots.
  App project window: `app/src/routes/(observatory)/projects/[id]/` (exists). Reuse gate mechanics.

### Slot 3 detail (SHIPPED ead8f971 — superseded)
- **Slot 3 — Observatory · Projects** — gate1 ✅; BACKEND ✅ DONE+VERIFIED (list_projects
  extended additively w/ icon/stack/vision/repos_count/libs_count/last_session_at/sessions7d;
  clippy clean; max repos_count=5 correct; Today not regressed; install-debug live; uncommitted).
  FRONTEND now in flight (svelte-file-editor): update ProjectListItem type, buckets
  (archived=maturity==='archived'), grid/list toggle persisted localStorage
  `sensei:projects:view`, card refactor (active ftr/repos/libs; dormant repos/libs/last-session),
  ProjectRow list component, tests. Then gates 2/3/4 → commit.
- FRONTEND ✅ GREEN: check 0/0 (837 files), test 744 pass (701+43, 0 regress), all svelte
  MCP-validated. New: ProjPill/ProjectDot/ProjectCard(refactor)/ProjectRow (+harness+spec),
  projects-view.svelte.ts (+spec), buckets rewritten (+spec), types/storage-keys/+page.ts/svelte.
  Decisions: stack kept object shape (4 other consumers use stack.languages — DRY); localStorage
  feature-detected (no silent errors). Gates 2(done)+3(wrong) LAUNCHED. Backend clippy clean
  per fork; frontend green ⇒ zero-errors checkpoint-2 satisfied (no separate rust re-run).
- Gate 2 done-gate-verifier: **ready-to-ship** (297 total = 5 active + 292 dormant + 0 archived,
  sum ok; card ftr/repos/libs; view persists; vision truncates; Tauri-visual not-verifiable).
- Gate 3 wrong-gate-hunter: **CLEAN** (all 11 absent). Flags → observatory-projects-followups.md:
  stack shape mismatch (harmless/unused), list-vs-Overview repo kind filters (equivalent today),
  N+1 folders enrichment (pre-existing perf). None block.
- Gate 4 sensei-persona-reviewer: ran. P0 finding FIXED before commit — isWarning fired amber
  on all 292 dormant projects (ftr_7d:0 fallback); now guarded on sessions7d + fanout collapsed
  to active-only (594→~10 req). Verified check 0/0, 745 tests. Deferred persona items (adaptive
  default deviates from spec, dormant-pill noise, recency sort, minor) → followups. ALL GATES CLEARED.
- **Slot 3 COMPLETE.** Committing (spec + pg_store + all projects/ frontend + followups) + push develop.
  KEY DATA MODEL: repos_count = `sensei.folders` WHERE `kind IN ('git','standalone')` (NOT
  kind='folder' — those are 10.7k nested dirs); libs_count = `sensei.project_libraries`;
  last_session_at=MAX/sessions7d=COUNT7d over `activity.sessions.project_id/started_at`;
  vision=`projects.goal`; icon/stack are jsonb cols. list_projects @ pg_store.rs ~4109;
  handler `observatory::list_solutions` already enriches `folders[]` (keep; ADD scalar fields).
  Reviewer clarifications to honor: `status` derived (sessions7d==0||maturity=='archived');
  `ftr14d` stays in +page.ts /ftr fanout (NOT this endpoint); `warn` from quality-signals.
  Frontend: EXISTING `app/src/routes/(observatory)/projects/` (+page.svelte/+page.ts/buckets.ts,
  ProjectListItem type) — add list-view toggle + card refactor.

### Slot 3 detail (superseded)
- **Slot 3 — Observatory · Projects (list-view addition)** — gate1 `needs-fixes` (3 FAIL, all
  field-availability, FEASIBLE — verified data sources exist); spec FIXED; re-review launched.
  Fixes: vision→`sensei.projects.goal` (aliased); added "Daemon extension required" note to
  extend `list_projects` (pg_store.rs) w/ icon+stack+goal + join aggregates repos_count
  (folders, repo-roots only), libs_count (project_libraries), last_session_at+sessions7d
  (activity.sessions); archived = `maturity='archived'` (no bool col); localStorage key
  `sensei:projects:view`; done-gate curl prereq note.
  IMPL when ready: backend list_projects SQL extension + frontend list-view toggle + card
  refactor (drop 7d, add repos/libs) on EXISTING `app/src/routes/(observatory)/projects/`.
  project-icon image icons deferred (kanji fallback ok). Reuse Slot-1 gate mechanics.
- **Slot 2 PARKED** (see park/observatory-instruments-health.md) — 3-try-then-park invoked;
  AWAITS Jerry. Continued to Slot 3 per plan.

---
### Slot 2 note: gate mechanics reminder (reuse for all slots)
- Gate agents run as `general-purpose`/sonnet with the .claude/agents/*.md procedure inlined
  (they aren't registered subagent types). svelte-file-editor for .svelte. Daemon on DEBUG
  binary; iterate with `make install-debug`. node_modules is UNTRACKED+not-gitignored → never
  `git add -A`; stage explicit trees. Pre-commit hook runs make test-fast (bootstrap+app).

---
### Slot 1 history (SHIPPED 35a438ce) — kept for reference
- **Slot 1 — Observatory · Today** — gates 2/3 returned; 2 defects fixed; re-verifying.
- **Gate 2 done-gate-verifier: not-ready** — 1 FAIL: hero.action "Review recommendation"
  used forbidden triage verb. FIXED → "Open insights" (observatory_home.rs mature_hero +
  test + guard assertion). Gates 1–6 pass/not-verifiable-here (dark-mode needs live app).
- **Gate 3 wrong-gate-hunter: one-or-more-tripping** — 1 TRIPPING: insight mute tone →
  text-ink-soft, spec wants ink-mute. FIXED → text-ink-mute (today-view.svelte.ts:56 +
  spec). Anti-patterns 1,2,4,5,6,7,8 all clean.
- **Clippy**: 4 warnings ALL pre-existing (mcp_servers.rs, activity_pruner.rs,
  verdict_classifier.rs, pg_store.rs:7158 backfill path) — NONE in my new code. Zero new lint.
- **Non-blocking follow-ups** (note, don't block Slot 1): (a) mature koan hero.source empty
  — the live top rec's evidence has no session UUIDs (session_ids_from_evidence returns []);
  provenance appears when a rec carries session evidence. (b) insight label/kanji all same
  ("繰"/"Recommendation") — insight-copy pipeline #65. (c) adopted `what` is raw prose
  (LLM distillation deferred, insight-copy). (d) adopted empty-state wording differs from
  spec string (not shown on live data). (e) recentSessions wire sends ISO ts+turns not
  human when/duration — RecentSessions computes correctly client-side (DRY reuse).
- BOTH FIXES VERIFIED GREEN (2026-07-07): app-check 0/0; app test:unit 701 pass (fixed
  InsightCard.spec mute assertion too); senseid observatory_home 10/10; install-debug
  restarted daemon; live `hero.action == "Open insights"`. Daemon now on DEBUG binary.
- Gate 2 re-check (done-gate-verifier): **READY-TO-SHIP** — all 7 gates pass (gate 7 now
  "Open insights", route /insights exists), dark-mode flagged for manual visual only.
  NOTE: verifier's "ftr14d null" caveat was a FALSE MISREAD — live /api/observatory/ftr
  returns ftr14d 0.5 / ftr14dPrev 0.733 / trend[14] / sessions7d 10; today payload correctly
  has no ftr fields (separate endpoint); Rust returns unwrap_or(0.0) so never null. Chip = 50% ↓23%. OK.
- Gate 4 (sensei-persona-reviewer): ran. Verdict: screen works structurally + passes both
  mechanical gates; flags trust-proof thinness (empty noticed/source) + minor items. All
  evaluated → documented in `park/observatory-today-followups.md`; none block (Fix A needs
  DDL/off-limits, Fix C already handled via client early-fallback+server-log, Fix B is an
  unreachable state needing payload plumbing). ALL 4 GATES CLEARED.
- **Slot 1 COMPLETE.** Committing spec+backend+frontend+park as one commit, push develop, → Slot 2.
- Frontend GREEN: `bun run check` 0/0 (827 files); `bun run test:unit` 701 pass (was 666,
  +35 new, 0 regress). Files: today-view.svelte.ts (state), HeroKoan/InsightCard/AdoptedCard
  (+harness+spec each), +page.ts (2 endpoints), +page.svelte (mockup rebuild, removed <style>
  color block), types.ts+api.ts, page.spec rewritten. Koan action → /insights route.
- Rust unit tests GREEN: 10/10 observatory_home tests pass (`cargo test -p senseid
  --features senseid/embedded-llama-cpp observatory_home`). Crate compiles clean.
  clippy re-running (`--bins --tests`, not `--lib` — senseid is bin-only crate).
- IN FLIGHT: gate2 done-gate-verifier, gate3 wrong-gate-hunter (parallel), clippy re-check.
- NEXT after all 3 green: gate4 sensei-persona-reviewer → ONE Slot-1 commit (spec+backend+
  frontend) + push develop → Slot 2.
- Old sub-position (superseded):
- **Gate:** 1/5 spec-doc-reviewer ✅ PASSED. Backend code WRITTEN by fork (uncommitted):
  new `crates/senseid/src/observatory_home.rs`; modified observatory.rs, sessions.rs,
  routes.rs, pg_store.rs, main.rs. Fork sanity-checked types. `make crates-debug &&
  make install-service` (release build, LTO) IN PROGRESS — daemon still old binary so
  /today + /ftr 404 until install completes + restart. Background poller `bmjk08ekj`
  waits for routes→200 then curl-verifies today/ftr/sessions.
- IF poller shows green: run gates 2+3 (done-gate-verifier + wrong-gate-hunter) on backend,
  then build FRONTEND (svelte-file-editor: rewrite +page.ts to call the 2 endpoints; rebuild
  +page.svelte to mockup; state in *.svelte.ts; 24 tokens; reuse RecentSessions). Then
  gate 4 persona-reviewer. Then ONE Slot-1 commit (spec+backend+frontend) + push develop.
- IF poller shows build FAILED or routes still 404: read fork's changes, fix, rebuild (≤3 tries then park).
- Spec `screen/observatory-today.md` is corrected + accurate (fence repaired; Purpose no
  longer hardcodes a count; adopted-lane filter matches `list_active_memories`; recentSessions
  shape has ftr(bool)/duration/corrections). Commit spec+impl together as the Slot-1 commit.
- Spec fixes applied to `screen/observatory-today.md`: named real source tables
  (dataMaturity→`activity.sessions.analyzed_at` + shared `maturity_signal`; adopted→
  `sensei.memories` in-force via `list_active_memories`; ftr→`sensei.ftr_daily`);
  added "New backend work" prereq (both endpoints are NEW); insight-copy xref;
  inverse adopted wrong-gate; koan-CTA verb exemption.
- Resolved: adopted lane = `sensei.memories` status IN (active,reinforced,battle_tested),
  NOT `inference.detected_patterns` (that's project-window teachings).
  `/api/observatory/today` + `/ftr` both 404 → build as new handlers.

## Gotchas (carry forward)
- **Gate agents are NOT registered** as subagent types. `.claude/agents/{spec-doc-reviewer,
  done-gate-verifier,wrong-gate-hunter}.md` exist but the runtime can't resolve them by name.
  Run each gate as a `general-purpose` agent (model: sonnet) with the gate .md body inlined
  as instructions + the target doc path. Preserves isolated-context discipline.
- **FTR endpoint**: spec wants `/api/observatory/ftr {ftr14d,ftr14dPrev,ftrTrend[],sessions7d}`
  but code has `/api/observatory/ftr-daily`. Reconcile during impl (assemble new or reuse).
- Existing observatory endpoints: ftr-daily, model-effectiveness, tool-insights, tool-signals,
  tool-usage. `/api/observatory/today` does NOT exist yet.
- zsh: quote grep globs — `--include='*.rs'` (bare `*.rs` triggers "no matches found").
- Daemon is the RELEASED brew binary; to test new Rust endpoints: `make crates-debug &&
  make install-service` (NOT make bump — that's off-limits).

## Slot 1 reuse map (recon done — impl should assemble, not build new pipelines)
- Handler layer: `crates/senseid/src/api/handlers/observatory.rs`; routes mounted in
  `crates/senseid/src/api/routes.rs`. App group: `app/src/routes/(observatory)/`
  (root Today page → `+page.svelte`).
- `dataMaturity`: `maturity::maturity_signal(watched, has_insights, MATURITY_TARGET)` fed by
  `pg.get_project_maturity_inputs(project)` (per-project today; `today` needs it aggregated
  across active projects — server decides, not UI).
- FTR chip+trend: `pg.get_ftr_daily(None, days)` (holistic) / `get_ftr_daily(Some(p), days)`.
  New `/api/observatory/ftr` = assemble {ftr14d, ftr14dPrev, ftrTrend[], sessions7d} from a
  28-day ftr_daily pull.
- adopted lane: `pg.get_adopted_teachings(project, limit)` (per-project — aggregate for today).
- insight cards: tool_signals `derive_signals`+`curate_insights`, or insights pipeline (partial).
- recent sessions: `pg.list_sessions_by_folder`; session-id resolution already a known regression.

## Slot 1 impl notes (pending, after re-review passes)
- FIX SPEC WORDING: adopted-lane real filter is `sensei.memories status='active' AND
  strength>=1.0` (what `list_active_memories` does), NOT the broader
  `IN (active,reinforced,battle_tested)` I wrote. Also `list_active_memories(project,scope)`
  is single-project(+global) — for cross-project "today" need a small aggregate query or
  loop active projects. Correct the spec phrase post-review to avoid over-claiming.
- Route insertion: add `/api/observatory/today` + `/api/observatory/ftr` GET routes in
  `crates/senseid/src/api/routes.rs` right after line 113 (the observatory block, after
  model-effectiveness). Handlers go in `api/handlers/observatory.rs`.
- Done-gate needs `GET /api/sessions/{id}` (name-or-UUID) — verify it exists before wiring
  hero.source links.
- To test new Rust handlers: `make crates-debug && make install-service` then curl :7744.
- `GET /api/sessions/{id}` does NOT exist (only PUT + /replay + /tool-timeline GET). Add a
  small GET handler (name-or-UUID → session row) for hero.source + recentSessions resolution.
  Handlers in `api/handlers/sessions.rs`; recent-sessions row → `/api/sessions/{id}/replay`.
- ftr-daily is SPARSE (only days with sessions). `/ftr` must emit 14 calendar-day trend
  points, tolerating gaps; means computed over present days. Real data starts ~2026-06-11.
- App API client: `app/src/lib/api.ts`. Root Today page → `app/src/routes/(observatory)/+page.svelte`.

## Baseline (zero-errors checkpoint 1) — CLEAN @ Slot 1 start
- app-check (svelte-check): 0 errors, 0 warnings, 816 files.
- app unit (vitest): 666 passed / 49 files.
- rust fast (make test-crates-fast): 175 passed, 0 failed.
- Checkpoint 2 must match (zero) before commit.

## Slot 1 implementation shape (decided)
- Current `+page.ts` load assembles maturity CLIENT-SIDE from many endpoints — VIOLATES
  spec "daemon decides". Refactor: daemon builds `/api/observatory/today` (+ `/ftr`);
  load calls just those two. Move koan/insights/adopted/maturity assembly server-side.
- Root page + loader already EXIST (rebuild, not greenfield): `(observatory)/+page.svelte`
  (13KB), `+page.ts`, plus `RecentSessions.svelte` + `recent-sessions.ts` (reuse).
- app rules (app/CLAUDE.md): 24 named tokens only, no z-scale/OKLCH/hex/`<style>` color;
  state in `*.svelte.ts`; data in `+page.ts`; WIRE API wins over mockup; harness+spec per
  component; svelte-file-editor MANDATORY for .svelte. Canonical:
  `docs/design/frontend-svelte-guidelines.md` (READ before UI work).
- TDD targets (pure, tests-first): ftr aggregation (28 sparse daily → ftr14d/ftr14dPrev/
  ftrTrend[14]/sessions7d); maturity aggregation reuses tested `maturity_signal`.

## Slot 1 BACKEND design (LOCKED — implement exactly this)
Real data: 26 analyzed sessions + has_insights=true ⇒ aggregate = MATURE renders.
337 pending recs, 9 active-strong memories. Recent sessions often blank-label (handled).

New pure module `crates/senseid/src/observatory_home.rs` (register in lib.rs/main mod list):
- `greeting(hour:u32)->&'static str`: 5–11 "Good morning", 12–17 "Good afternoon",
  else "Good evening". TDD.
- `early_hero(watched:i64,target:i64,recent_ids:&[String])->Value`: kanji "観",
  koan "Still listening.", body "sensei has watched {watched} sessions so far…",
  impact "~{max(1,target-watched)} more sessions until the first lesson", action null,
  source = recent_ids joined " · ", noticed "since setup". TDD.
- `mature_hero(top:&RecLite, sources:&[String])->Value`: kanji "聴", koan=top.title,
  body=top.why, impact=top.impact (fallback "" → omit dot), action "Review recommendation"
  (nav CTA → /learnings), source = sources joined " · " (only real session ids; empty→""),
  noticed from age. TDD.
- `insight_card(rec:&RecLite)->Value {kanji,label,text,tag,tone}`: tone by urgency
  (high→warn, medium→mute, low→mute; good reserved for adopted); label "Recommendation".
  Cap insights to 3. Early insights = fixed listening/calibrating pair. TDD.
- `adopted_row(mem)->Value {when,what,scope,source}` mapping active memory. TDD.

New PgStore methods (mirror existing patterns, pg_store.rs):
- `get_holistic_ftr()->Value {ftr14d,ftr14dPrev,ftrTrend[14],sessions7d}`: mirror
  `get_project_ftr` (4283) WITHOUT project filter, session-weighted headline from
  activity.sessions last14/prev14, trend via generate_series(current_date-13,current_date)
  LEFT JOIN day-avg (exactly 14 pts, 0-filled), sessions7d=count last 7d.
- `get_global_maturity_inputs()->(i64,bool)`: mirror get_project_maturity_inputs(4681)
  w/o project filter: count analyzed sessions + EXISTS recs OR EXISTS learned mem.
- `get_pending_recommendations_global(limit)->Vec<Value>`: mirror get_pending_recommendations
  (2866) w/o project filter, order urgency then recency; include evidence.
- `list_active_memories_global(limit)->Vec<Value>`: status='active' AND strength>=1.0
  ALL projects (drop the project_id IS NULL restriction), order strength DESC, limit.
- `get_session(id)->Option<Value>`: single session row by UUID (for GET /api/sessions/{id}).
  Reuse existing single-session query if one exists; else SELECT from activity.sessions.
- recent sessions: reuse the query behind `/api/sessions` list (sessions.rs get_sessions_stub)
  or add `recent_sessions(limit)`; shape {id,when,project,ftr(bool),duration,corrections,summary}.

New handlers (api/handlers/observatory.rs) + routes (routes.rs after line 113):
- `observatory_today` → GET /api/observatory/today (assemble via observatory_home + PgStore).
- `observatory_ftr`   → GET /api/observatory/ftr (get_holistic_ftr).
- `get_session` in sessions.rs → GET /api/sessions/{id} (name-or-UUID; add route at line 195
  area alongside existing PUT). Session-id resolution honored.

Gate targets (done): curl /api/observatory/today shows dataMaturity + hero + insights≥1;
/api/observatory/ftr integer-able ftr14d; GET /api/sessions/{id} resolves a real id.
Build/verify: `make crates-debug && make install-service`, then curl :7744.

## Slot 1 BACKEND — GREEN (verified live 2026-07-07 ~21:25)
- /api/observatory/today: maturity "mature", koan "Recurring corrections in FizzBot",
  action "Review recommendation", insights 3, adopted 5, recent 5, greeting "Good evening",
  today "Tue · 7 Jul", hero.source "" (top rec has no evidence session ids → empty, OK).
- /api/observatory/ftr: ftr14d 0.5, ftr14dPrev 0.733, ftrTrend len 14, sessions7d 10.
- GET /api/sessions/{id}: 200.
- RESOLVED: "FizzBot" is REAL data (/Users/Jerry/Work/ai-labs/FizzBot) — a live ~/Work repo,
  NOT seed junk. Top recs all real repos (FizzBot, rokkit, dbd-rs, minilm-bench). Koan is
  surfacing genuine signal. No wipe needed. (Rec COPY is templated — insight-copy deferred,
  spec allows fallback copy; not a Slot-1 defect.)
- FLAG for gates: hero.source empty → UI must render "· noticed" without a dangling leading "·".
- Files (uncommitted): +observatory_home.rs; ~observatory.rs, sessions.rs, routes.rs,
  pg_store.rs, main.rs. NOT yet clippy/test-verified by me — fold into done-gate.

## Log
- 2026-07-07: run started; env verified; recon done. Gate-1 spec-doc-reviewer PASSED
  (2 rounds). Baseline clean. Backend design LOCKED. Delegating backend impl to a fork
  (inherits full context) with TDD + build + curl evidence.
