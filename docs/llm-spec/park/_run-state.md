# Vacation run — live state

## ⭐⭐ STANDING OPERATING POLICY (set by Jerry 2026-07-08 — SUPERSEDES the plan's off-limits)
The run is NOT done at the 6-slot queue. Keep working continuously (5-day intent). When the
queue empties, PULL the next work — never stop and declare done. Policies:
1. **FULL BUILD AUTHORITY.** DDL changes, new tables, new pipelines/generators, capture changes,
   and `make bump`/release are all authorized. Rails still hold: gated per doc, tests green
   (zero-errors), never touch .env/credentials, never destroy the data dir, DDL via .ddl-source-
   first then dbd apply.
2. **DEFAULT-AND-PROCEED on design forks.** Pick the best REVERSIBLE option, record the choice
   here, keep moving. PARK only when a decision is irreversible / external / destructive / truly
   un-defaultable. Do NOT stop and ask on ordinary design forks.
0. **FULL SCOPE (Jerry 2026-07-08):** hit EVERYTHING specced in `docs/llm-spec/` (all screen/ +
   pipeline/ docs) INCLUDING the Dōjō SaaS segment. "Complete as much as you can over the 5 days."
   Never stop while backlog remains. **Dōjō auth:** use Supabase (can't deploy → assume a
   localhost Supabase URL) + **kavach** (`~/Developer/kavach`) — EDIT kavach if needed for
   Supabase auth in the Dōjō SaaS layer. Assume a **localhost Dōjō registry** URL for
   registration/setup. Dōjō is the LAST big segment (needs the auth infra) — after depth+breadth.
3. **DEPTH FIRST priority:** (a) finish Slot 2 (MCP registry) → (b) burn down the deferred
   follow-ups across the 5 shipped screens = make them fully real: wire insight-copy (copy via
   gemma4, not raw DB text), define the memory promotion/merge statuses + wire readyToShare/toMerge,
   build the missing recommendation/pattern generators, the per-screen followup items → (c) overflow
   7/8 (Memories, Project Sessions+Memories) → (d) new llm-spec screens. Work the followup notes in
   park/ as a live work queue.
4. **MERGE + BUMP AT MILESTONES.** After a clean segment (all gates green, tests pass), merge
   develop→main and `make bump` so it's a real release. (Was: develop-only. NOW: release at milestones.)
5. **Tool discovery = a per-assistant TRAIT** (like the assistant adapters). Discovery differs by
   assistant (Claude Code ~/.claude/mcp.json + project .mcp.json; Zed context_servers; Cursor
   .cursor/mcp.json). Refactor mcp_discovery.rs (AcpFamily + parse_mcp_section already there) into a
   `ToolDiscovery` trait with per-assistant impls feeding the unified inventory.



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
5. Observatory · Insights — `screen/observatory-insights.md`  ✅ SHIPPED (all gates; MeasureVerdicts wired)
6. Observatory · Sessions — `screen/observatory-sessions.md`  ✅ SHIPPED (all gates; chartedMins P0 fixed)

**QUEUE COMPLETE: 5 of 6 shipped + verified (Slot 2 parked). Overflow 7/8 optional.**

---

## RUN COMPLETE — 2026-07-08 (queue slots 1–6 done)

Vacation run (docs/llm-spec/EXECUTION-PLAN.md) target queue finished. All work on `develop`,
each screen through the full gated loop (spec-doc-reviewer → implement → done-gate-verifier +
wrong-gate-hunter → sensei-persona-reviewer → commit). NOT merged to main (Jerry does that).

| Slot | Screen | Result | Commit |
|---|---|---|---|
| 1 | Observatory · Today | ✅ shipped | `35a438ce` |
| 2 | Instruments · Health | ⛔ PARKED (AWAITS Jerry) | `15e680cb` (park record) |
| 3 | Observatory · Projects (list view) | ✅ shipped | `ead8f971` |
| 4 | Project window · Overview | ✅ shipped | `fa18a4d1` |
| 5 | Observatory · Insights (triage) | ✅ shipped | `035a368c` |
| 6 | Observatory · Sessions (digest) | ✅ shipped | `a83303c6` |

**5 shipped-and-verified + 1 parked.** Overflow slots 7 (Observatory · Memories) and 8
(Project · Sessions+Memories) NOT started — optional, deferred to a future session.

### For Jerry on return
1. `git fetch --all`; read the develop log (6 commits: 5 feature + 1 park).
2. **One decision awaits you — Slot 2 (Instruments · Health):** the MCP registry
   (`sensei.mcp_servers`, 2 Zed rows) and MCP usage (`sensei.tool_usage_stats`, Claude Code
   plugin tools) are disjoint and don't join → the L1 `share_invoked` grid has no truthful
   source. Pick: (A) unify capture, (B) redefine L1 off tool_usage_stats, (C) descope to L2.
   Full analysis: `park/observatory-instruments-health.md`.
3. Per-screen follow-ups (all non-blocking, documented): `park/observatory-today-followups.md`,
   `park/observatory-projects-followups.md`, `park/project-overview-followups.md`,
   `park/observatory-insights-followups.md`, `park/observatory-sessions-followups.md`.
   Highlights: insight-copy not wired (raw text fallback, run-wide deferral); memory
   promotion/merge statuses undefined (readyToShare/toMerge=0); Replay-nav is fully wired
   from Sessions but pending from Project-overview until the Replay screen lands; `all` range
   chip; ViolationCard review-nav. NONE block the shipped screens.
4. New endpoints added this run: `/api/observatory/today`, `/api/observatory/ftr`,
   `GET /api/sessions/{id}`, `/api/projects/{id}/overview`, `/api/insights`; extended
   `/api/projects` (icon/stack/vision/repos_count/libs_count/last_session_at/sessions7d) and
   `/api/sessions` (?range=/?project=/agent); accept-recommendation now enqueues MeasureVerdicts.
   Daemon is running a DEBUG binary (make install-debug) — rebuild release / `make bump` on return.
5. If ≥4 slots clean (they are), merge develop→main + bump per the plan.
(overflow: 7 Observatory·Memories, 8 Project·Sessions+Memories)

## Current position
- **Slot 6 — Observatory · Sessions** — gate1 ✅ (2 rounds: not-ready→needs-fixes→fixed; residual
  `shipped`→`completed` + header wording applied w/o 3rd round). BACKEND (range+agent) delegated
  to fork; then frontend chart-variant digest; then gates 2/3/4; commit → RUN QUEUE COMPLETE.
- BACKEND ✅ VERIFIED: list_all_sessions(limit,range_days,project) + get_sessions_stub parses
  ?range=/?project= (pure range_to_days +2 tests) + `agent` (from acp_id harness — NO
  assistant_family col; agent="claude"/"zed"). Today caller updated. clippy clean. Wire row:
  {id,project,task,summary,outcome,ftr,turns,corrections,startedAt,completedAt,agent}. Live:
  total=27, ?range=7d→10, ?project=sensei→11. FRONTEND now (svelte-file-editor): SessionsDigestZen
  at /sessions; chip group trend(default)/stream/constellation/bands + mini-cycler(+pulse);
  row→goto('/instruments?tab=replay&session={id}'); derive when/time/duration; per-day agg client;
  range chips refetch; outcome→good(completed+ftr)/bad(corrected)/ugly(abandoned).
- FRONTEND ✅ GREEN: check 0/0 (871 files), test 837 (47 new, 0 regress), MCP-validated. Files:
  sessions-digest.ts(pure)+.svelte.ts(state)+2 specs, Trend/Bands/Stream/Constellation/MiniChart,
  SessionRow(+harness+spec), +page.ts/svelte, types.ts, api.ts. BONUS: agent WIRED instruments
  Replay deep-link (?tab=replay&session={id} → resolves via GET /api/sessions/{id}) — cross-screen
  dep CLOSED. 4 chips(no pulse), pulse mini-only, range refetch (?range=7d→10/27; project=sensei&7d→3).
  Gates 2(done)+3(wrong) LAUNCHED (read-only endpoint, no mutation risk).
- Gate 2 done: partially-verified, all 9 code/API gates PASS (session-id resolution CLEAN,
  Replay deep-link honored, range works, 4-chips/no-pulse). Gate 3 wrong: CLEAN (all 7 absent;
  regression clean — 148 replay calls). Non-block items: `all` range chip missing (signals-table
  deviation, cheap — SESSION_RANGES; range_to_days already treats all/unknown→no-filter);
  single-session GET returns snake_case vs list camelCase (latent, screens unaffected); long
  durations (backend data-quality). Gate 4 persona LAUNCHED → then batch `all` chip + persona P0s
  → commit Slot 6 (FINAL).

### Slot 6 detail (superseded)
- **Slot 6 — Observatory · Sessions** — gate1 `not-ready` (5 FAIL, all field/endpoint, FEASIBLE).
  Spec fix DELEGATED w/ LOCKED decisions; then re-review; then impl. DECISIONS: (1) add optional
  `?range=7d|30d|90d` (cutoff on activity.sessions.started_at) + `assistant_family`(→`agent`) to
  the sessions list endpoint (get_sessions_stub/list_all_sessions); UI derives when/time/duration
  from startedAt/completedAt; wire `title`=`task`. (2) real outcome enum completed|corrected|blocked|
  partial|abandoned → good=completed(+ftr) / bad=corrected / ugly=abandoned(blocked/partial neutral).
  (3) row-click→`goto('/instruments?tab=replay&session={id}')` (cross-screen dep; id resolves via
  GET /api/sessions/{id} from Slot 1). (4) pulse=mini-cycler mode NOT a full chart; full charts=
  trend(default)/stream/constellation/bands. (5) DROP synthetic history (real ~216 sessions).
  (6) per-day aggregation client-side. Mockup: sessions-zen.jsx SessionsDigestZen.

### Slot 6 detail (superseded)
- **Slot 6 — Observatory · Sessions** — STARTING (gate 1 spec-doc-reviewer). FINAL target slot.
  Plan notes: compact activity digest; chart-variant work new (trend/stream/constellation/bands/
  pulse); session-id resolution regression already fixed once (MUST survive — recentSessions row
  click → Replay resolves session-id). Data: `GET /api/sessions?range=&project=` mostly present.
  Definition of shipped: default trend variant renders; ≥1 alternate variant works; row click →
  Replay resolves session-id. Mockup: find via MOCKUP-INDEX. Reuse gate mechanics.

### Slot 5 detail (SHIPPED 035a368c — superseded)
- **Slot 5 — Observatory · Insights** — gate1 ✅ (2 rounds: not-ready→needs-fixes→all fixed;
  trivial impact→urgency parentheticals + inference.corrections source applied w/o 3rd round).
  BACKEND (aggregating /api/insights) delegated to fork; then frontend triage; then gates 2/3/4.
  corrections source = `inference.corrections` (NOT sensei.corrections). Reuse
  get_pending_recommendations_global (added Slot 1) for cross-project recs. Actions reuse
  existing accept/reject (no new write endpoints).
- BACKEND ✅ VERIFIED: new `insights.rs` bucketing module (4 tests) + main.rs + 4 pg_store
  queries + get_insights handler + `GET /api/insights` route. clippy clean (4 pre-existing).
  Wire: {counts{now,soon,settled}, projects[{id,name,kanji}], recommendations[{id,urgency,title,
  why,impact,evidence,project_id,name,column}], memories[{id,status,title,content,violated_count,
  strength,scope,project_id,column}], patterns[{id,name,family,lifecycle,instance_count,project_id,
  column}], corrections[{id,text,suggestion,count,column}]}. Each item tagged `column`=now|soon|settled.
  Live: cross-project counts{25,184,9}; ?project=sensei {9,180,1}. FRONTEND ✅ GREEN: check 0/0
  (855 files), test 790 (28 new insights, deleted 5 old triage, 0 regress), MCP-validated.
  Files: types.ts(InsightsBoard), api.ts(getInsights+reused accept/reject), insights/+page.ts/svelte,
  insights-board.svelte.ts(+14-test spec), RecCard/ViolationCard/CorrectionMini/PatternMini/MemoryRow/
  ProjectFilterStrip (+harness+spec), deleted triage.ts/spec. Columns bucket by server `column`
  (no client re-derive); Apply highlighted (1-decision-1-default); Apply→accept(MeasureVerdicts)+
  optimistic remove+restore-on-fail; Review→nav; filter refetches ?project=. Judgment: dropped
  mockup kind-chip (wire has no action_type). Gates 2(done)+3(wrong) LAUNCHED — verify action wiring
  by CODE (do NOT POST accept/reject — mutates real recs).
- Gate 2 done-gate-verifier: not-ready — 6/7 pass; Gate 5 FAIL: `/accept` handler never enqueued
  MeasureVerdicts (only the periodic scheduler did) → spec's "apply schedules MeasureVerdicts" unmet.
  FIXED: accept_project_recommendation (project_detail.rs:351) now enqueues
  Task::new(TaskKind::MeasureVerdicts,"","") after accept (also improves Slot-4 Accept button).
  Holding rebuild until gate 3 returns, then batch-rebuild+verify. Awaiting wrong-gate-hunter.
- Gate 3 wrong-gate-hunter: one-or-more-tripping — SAME issue (Item 2: Apply→no MeasureVerdicts),
  already fixed. Other 7 clean. Additional (deferrals, non-block): insight-copy not wired (raw DB
  text = accepted fallback, varied so no symptom), unused server counts (client recomputes
  optimistically = correct), violation cards no action (memory write-actions deferred).
  → observatory-insights-followups.md. Rebuild+verify job `bast5o8ze` running (install-debug +
  clippy + insights tests). On green → gate 4 persona → commit Slot 5 + push → Slot 6.
- MeasureVerdicts fix REBUILT+VERIFIED (accept enqueues @ line 370; clippy clean; 8 tests). Gate 4
  persona: mechanical gates pass; flagged LIMIT-200 silent truncation (137 recs dropped) — MITIGATED
  with tracing::warn (get_insights_recommendations); ViolationCard dead-end = spec-compliant deferral
  (high-priority enhancement follow-up); board.loading/Soon-subcap/aria = follow-ups →
  observatory-insights-followups.md. Rebuild+verify `bxg4vzln0` running (warn). On green → commit Slot 5.

### Slot 5 detail (superseded)
- **Slot 5 — Observatory · Insights (Learnings Triage)** — gate1 `not-ready` (4 FAIL, all
  field/endpoint, FEASIBLE — reviewer verified tables/enums). Spec fix DELEGATED; then re-review;
  then impl. LOCKED DECISIONS: /api/insights = NEW aggregating endpoint, optional `?project=`
  scope (=scope.project), cross-project when unscoped; bundles recs (inference.recommendations by
  `urgency`: high→Now, medium→Soon, low→Settled) + memories (sensei.memories by `status`) +
  patterns (inference.detected_patterns by `lifecycle`). ACTIONS reuse EXISTING per-project
  `POST /api/projects/{project_id}/recommendations/{rec_id}/accept` (Apply, already triggers
  MeasureVerdicts per project_detail.rs:345) + `/reject` (Dismiss); Review=navigate (no write);
  project_id from each card. Memory write-actions (reinforce/challenge/archive) DISPLAY-ONLY/
  deferred (only /promote exists). FIELD FIXES: m.state→m.status, m.violated→m.violated_count,
  "battle-tested"→'battle_tested', p.kind==="emerging"→p.lifecycle='suggested', impact→urgency.
  Mockup: learnings-v2.jsx LearningsTriage. 337 pending recs live.

### Slot 5 detail (superseded)
- **Slot 5 — Observatory · Insights (Learnings Triage)** — STARTING (gate 1 spec-doc-reviewer).
  Plan notes: triage surface for the recommendation pipeline; insights pipeline partial (buckets
  query needs finishing); verb set Apply·Review·Dismiss (one-decision-one-default). Data prep:
  `GET /api/insights` server-side bucketing (Now/Soon/Settled); POST apply/review/dismiss.
  Definition of shipped: 3 columns render on real data; apply schedules a MeasureVerdicts
  follow-up; verbs consistent. REALITY: 337 pending recs in inference.recommendations; existing
  endpoints `/api/projects/{id}/recommendations/{rec_id}/accept` + `/reject` — apply/dismiss may
  map to these. Check whether a global (cross-project) /api/insights exists. Mockup: find via
  MOCKUP-INDEX. Reuse gate mechanics (gate agents general-purpose/sonnet; svelte-file-editor;
  daemon DEBUG binary; install-debug; node_modules untracked→stage explicit trees).

### Slot 4 detail (SHIPPED fa18a4d1 — superseded)
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

## ✅ PHASE 1 MILESTONE (2026-07-08): Slot 2 shipped `6336dc6a` + RELEASED v0.2.24 (`a43c4657`)
Released all 6 completed screens (Today/Projects/Overview/Insights/Sessions + Instruments-Health).
`make bump` clean: tag v0.2.24 pushed, dbd cache cleared, subtrees synced (homebrew-tap 02c1970,
marketplace ed81a5f). GitHub Actions building artifacts. NEXT: merge develop→main (0 conflicts;
main's extra commits are just merge-commit history), then PHASE 2 (make shipped screens real).
- ✅ MERGED develop→main (`47edc0e5`, 0 conflicts) + pushed. develop @ a66439de. main released v0.2.24.
  PHASE 1 COMPLETE. → PHASE 2 START: pipeline/insight-copy (raw text → gemma4 copy across Today/
  Insights/Projects/Overview), then pipeline/memory statuses (readyToShare/toMerge), then generators
  (insights/patterns/signals writers). Assess each before building (don't rebuild what's live).

## PHASE 2 — insight-copy (in progress, 2026-07-08)
ASSESSMENT (done): insight-copy is GENUINELY unwired — no gateway copy call anywhere in senseid;
screens emit raw DB text. memory_status enum has NO promotion/merge-readiness value (pg_store.rs:4352
comment) ⇒ readyToShare/toMerge=0 until an enum+ladder is added. Recommendation generation EXISTS
(337 recs). So Phase 2 real work = (1) insight-copy pipeline, (2) memory promotion/merge statuses,
(3) audit generators. Doing insight-copy FIRST (linchpin — touches Today/Insights/Projects/Overview).
KEY FACTS resolved:
- Gateway chains are DAEMON-SIDE in `crates/senseid/src/api/gateway_init.rs` (lines 470-523:
  text_chat/reasoning/embed/image_generate). Adding an `insight-copy` chain there is IN-SCOPE (not
  the external gateway repo). Chain shape = FallbackChainConfig{id,capability:TextChat,models,triggers}.
  insight-copy chain must be LOCAL-ONLY (gemma-embedded→gemma4, NO cloud legs) per "offline must work".
- Gateway call pattern = `crates/senseid/src/tasks/handlers/corrections_llm.rs` (InferenceRequest{
  capability:TextChat, chain:Some("..."), Payload::Chat{messages,system,max_tokens,temperature,tools}},
  gateway.execute().await, graceful degrade to None/fallback). 400ms time-box = tokio::time::timeout.
- deps present: sha2 0.10, hex 0.4, serde_json. analysis/ is a dir-module (analysis/doc_drift.rs,
  `mod analysis` in main.rs) ⇒ add analysis/insight_copy.rs + register in analysis mod.
- facts_hash = sha256(kind_str + canonical_json(facts)) hex. Table sensei.insight_copy PK(kind,facts_hash).
BUILD SPLIT: A=core pipeline (DDL+module+chain+store methods+tests) [delegated general-purpose]; then
B=wire consumers (Today koan/insights + project overview hero) with existing raw strings as fallback.

BUILD A ✅ DONE+VERIFIED (2026-07-08): DDL sensei.insight_copy live (8 cols+2 idx), analysis/
insight_copy.rs (17-variant InsightKind, facts_hash sha256+canonical_json, generate_insight_copy
cache-first/400ms-timebox/60s-breaker[transport-only, validation-miss doesn't trip], voice_ok guard,
build_prompt), insight-copy chain in gateway_init (local-only gemma-embedded→gemma4), pg_store
get/upsert_insight_copy. 16 unit tests green, clippy clean on touched files. NOT committed (staged).
Entry: generate_insight_copy(&state.pg, &state.gateway, kind, &facts, CopyLimits::default(), FallbackCopy).
Deviations (all fine): #![allow(dead_code)] on module (Build B replaces w/ targeted enum allow);
model_provider always None (InferenceResponse exposes only .model); get_insight_copy returns
Option<(String,String)>. 30-day eviction sweep NOT built (needs daily maint task — later).
Known tradeoff: 400ms is tight for cold gemma → lazy first-hit likely falls back then breaker 60s;
real fix = EAGER warming at tick time (populate cache so wire reads hit). Out of scope now.

BUILD B ⏳ IN FLIGHT (2026-07-08): wire Today mature hero (HeroKoanMature) + rec insight cards
(InsightRecurringPattern) in observatory.rs::observatory_today (~L745). EARLY/STEADY stay STATIC by
design (avoid "koan invents teaching w/ no signal" wrong-gate). ≤4 model calls/screen (under 5 cap).
Replaces module dead_code allow w/ targeted enum allow. Fallback=existing mature_hero/insight_card text.
DESIGN NOTE: project-overview.md's "project_top_rec_hero"/"project_all_quiet" kinds are DOC drift vs the
canonical InsightKind enum — reuse HeroKoanMature/HeroKoanEarly with project-scoped facts (Build B2).
GATED-LOOP PLAN after B: done-gate+wrong-gate verify (general-purpose/sonnet) on A+B combined →
sensei-persona-reviewer → commit insight-copy milestone. THEN B2 (project-overview + insights-triage
wiring, mechanical) → THEN Build C (memory ready_to_share/to_merge read-path, per design-fork note above).

BUILD B ✅ DONE (2026-07-08): observatory.rs::observatory_today wired — mature hero koan+body →
HeroKoanMature, ≤3 rec cards → InsightRecurringPattern (card.text=copy.detail). Early/steady STATIC.
1187 tests pass (serial; 1 pre-existing parallel-DB flake unrelated). clippy clean on touched files.
Module dead_code allow → targeted enum allow. NOT committed (staged).
LATENCY FINDING: gemma2:2b (chain primary, in-process) ~390ms warm via CLI for ~120-tok JSON — RIGHT at
the 400ms box. So the 400ms wire timeout is a HARD spec constraint ("wire never blocks on inference");
correct fix for cold-gemma is EAGER WARMING (generate at rec-write/tick time → cache warm → wire hits
cache), NOT bumping the timeout. ⏳ DEPLOY+LIVE-VERIFY agent running: make install-debug + restart +
warm + curl /api/observatory/today, KEY QUESTION = does sensei.insight_copy get populated by the live
wire path (model copy visible) or only fallback (→ eager warming needed = Build B-eager followup)?
Awaiting verdict: SHIP / SHIP-WITH-FOLLOWUP(eager) / FIX.

VERIFY ROUND 1 = FIX (real bug found, 2026-07-08): insight-copy chain was NEVER registered in the
runtime gateway. Root cause: `merge_baseline_capability_gaps` (gateway_init.rs) grafts a baseline chain
only when its whole Capability is absent from the DB config; TextChat is already covered by DB
classify/reasoning/summarize, so the new named chain was SILENTLY DROPPED. `POST /api/gateway/infer
{chain:insight-copy}` → instant `{"error":"no candidates available for capability 'TextChat'"}`. Wire
path got instant Err → tripped 60s breaker → 100% fallback. Eager warming would NOT have fixed it.
Done-gate structural checks all PASSED; wrong-gates none fired; fallback path honest (never 500s).
FIX LANDED (staged, not committed): gateway_init.rs — extracted shared `graft_chain` helper (DRY),
added `const REQUIRED_NAMED_CHAINS=["insight-copy"]` + `merge_required_named_chains` grafting by NAME
even when capability covered, called right after the capability-gap merge (build baseline once). New
regression test `merge_required_named_chains_grafts_insight_copy_even_when_textchat_covered`. build
clean, 4 gateway_init tests pass, clippy clean. ⏳ VERIFY ROUND 2 in flight (resumed same agent):
re-deploy + confirm chain resolves + MEASURE warm insight-copy latency vs 400ms + KEY QUESTION (does
sensei.insight_copy populate on the wire now?). SECONDARY RISK still open: verifier measured ollama
gemma2:2b ~455ms warm (>400ms) — if warm insight-copy chain >400ms, lazy wire can't fill cache inline
⇒ need WARM-ON-MISS (tokio::spawn detached un-time-boxed generate+upsert on cache miss; wire still
returns fallback instantly, NEXT request hits warm cache) = Build B-eager. Decide from measured latency.

VERIFY ROUND 2 = SHIP-WITH-FOLLOWUP → converted to REWORK (2026-07-08): registration fix CONFIRMED
CORRECT (chain resolves: POST /api/gateway/infer{chain:insight-copy}→gemma-embedded content, 0.239s).
BUT model copy STILL doesn't reach the wire — TWO evidenced blockers:
 (1) 400ms tokio::time::timeout does NOT bound the in-process embedded (blocking) inference — every
     uncached /today load costs ~1.77s (all 4 calls run to completion; not preemptible). "Time-box" is
     ILLUSORY, breaker never engages. → Today got SLOWER for zero gain = UX regression. NOT shippable.
 (2) gemma2:2b routinely trips the guard: banned word "robust" + detail >180 chars → ~3/4 CARDS fail
     validation → fallback; cache never fills. (HERO passes 10/10 in isolation, fits 180.)
DECISION: don't merge the synchronous-wire wiring. Rework to spec's EAGER intent ("wire never blocks
on inference"): wire reads cache-ONLY (instant); on miss → return fallback + fire DETACHED background
warm (tokio::spawn) that generates+validates+caches for next load. + strengthen prompt (explicit
banned-word list + hard char budget + casing) + ONE retry on validation-miss. Off-wire budget generous
(~8s runaway guard; keep breaker so a down model doesn't pile warms). Facts: PgStore is Clone, AppState
gateway=Arc<Gateway> → both clone into spawn. Handler swaps generate_insight_copy→copy_or_warm(&state.pg,
&state.gateway,...). Registration fix (gateway_init) is KEPT + mergeable. ⏳ REWORK delegated.
NOTE (no-silent-errors): insight_copy tracing warn/debug DON'T reach public.logs (only sensei_logger
events do) — warm-path outcomes currently invisible; route through structured logger if easy.

REWORK ✅ DONE (2026-07-08, staged not committed): insight_copy.rs reworked off-wire —
copy_or_warm(store,&Arc<Gateway>,kind,facts,limits,fallback) = wire entry, CACHE-READ-ONLY (no
inference await); miss → fallback instantly + spawn_warm detached tokio::spawn (dedup via inflight
set, poisoned-mutex-safe). generate_and_cache = off-wire core (WARM_TIMEOUT_MS=8000 runaway guard,
up to 2 attempts w/ corrective retry, breaker on transport-fail only). build_prompt gained retry
param + strengthened limits (explicit char budget + banned-words line GENERATED from BANNED_WORDS =
DRY single source). read_cached_copy = pure cache read. generate_insight_copy REMOVED (0 callers).
Warm-path failures → public.logs via sensei_logger::Logger + LogWriter::pg(store.pool().clone())
(same pattern as api/server.rs task_logger) — no-silent-errors gap CLOSED. observatory.rs both call
sites swapped to copy_or_warm. 1191 tests pass (+3 new pure: banned-words-from-source, retry-prompt,
claim_inflight-dedup), clippy clean on touched files. ⏳ VERIFY ROUND 3 in flight (same agent):
SHIP BAR = (a) uncached /today latency fast again <~0.2s [was 1.77s], (b) model copy reaches wire after
bg warm (hero caches+renders mentor-voice), (c) warm failures visible in public.logs. Card pass-rate =
tuning note NOT blocker. If SHIP → persona-review → COMMIT insight-copy milestone (Build A+B+chain-fix+
rework as one unit) → then B2 (project-overview+insights wiring) → Build C (memory share/merge counts).
Do NOT keep looping on gemma2:2b card copy quality — architecture correctness is the milestone.

VERIFY ROUND 3 = ✅ SHIP (2026-07-08). Live evidence: uncached /today ~8-12ms (was 1.77s); 4/4 rows
cached (gemma-embedded), mentor-voice rephrased (banned "robust"→"more diverse"); self-heals after
cache clear; warm failures → public.logs; never 500. PERSONA REVIEW = architecture right, copy-quality
gap (3rd-person "The developer…" on 2/3 cards + template closing). Applied cheap CORE fixes (recs 1/2/4):
task_line reword (HeroKoanMature "complete sentence not a label"; InsightRecurringPattern drop "the
developer" subject), THIRD_PERSON_MARKERS const (DRY: prompt ban + voice_ok guard), +2 unit tests.
Rec 3 (facts specificity: pass project name/pattern into card facts) = FOLLOW-UP ticket.

★★★ INSIGHT-COPY TODAY MILESTONE COMPLETE + COMMITTED (2026-07-08) ★★★ develop:
  chore `d62edf3c` (cleared 4 pre-existing clippy warnings → senseid 0 warnings)
  feat  `96b1349a` (insight-copy pipeline: DDL + module + chain-fix + Today wiring + off-wire warm +
        persona hardening). 1192 tests pass, clippy 0, gated loop fully executed.
NOT merged to main yet (batching with B2). Running daemon (pid 5763) = round-3 build; does NOT yet have
the persona third-person edits — redeploy at next milestone (also DELETE FROM sensei.insight_copy then
to flush old "the developer" cached rows so live copy reflects the guard).
NOTED-DEFERRED (unrelated): pre-commit bootstrap test logs a dbd apply WARN `view:sensei.project_patterns
— column project_id already exists` (test still passes). DDL idempotency issue on that view; not chased.

B2 ✅ DONE + COMMITTED `a43db8e2` (2026-07-08): project_detail.rs get_project_overview hero →
HeroKoanMature (all-quiet static); observatory.rs get_insights → recs=InsightRecurringPattern,
memories=MemoryProposedAdopt/Review (adopt when in-force+unviolated), cap COPY_CAP=8 Now→Soon→Settled.
PATTERNS + corrections STAY STATIC (fixed a B2 regression: pattern card's only free-text `name` is
mono/truncated — routing prose there breaks it; route only when card gets a prose body = frontend
followup). project_detail impact always null (query doesn't select impact col — harmless facts field;
followup if wanted). 1192 tests, clippy 0. ⏳ PRE-MERGE LIVE SMOKE in flight (verifier): deploy + FLUSH
cache + smoke Today/Overview/Insights (fast <0.2s + cache fills mentor copy + NO "the developer"
leakage + no 500s). If SHIP → MERGE develop→main + make bump (insight-copy rollout milestone).

INSIGHT-COPY FOLLOWUP TICKETS (non-blocking, file when convenient):
 - Rec 3 (persona): pass project_name + specific pattern into card facts for specificity (needs facts
   shape + maybe query cols).
 - Pattern insight-copy: needs a prose body field on the pattern card (frontend) before routing.
 - project_detail overview: add impact column to get_top_recommendation query if non-null wanted.
 - /api/gateway/infer handler hardcodes temperature:None (irrelevant to wire now; cleanup if that
   endpoint is used for tuning).
 - DDL idempotency: view:sensei.project_patterns "column project_id already exists" on apply.
 - insight_copy 30-day eviction sweep (last_used_at < now()-30d) daily maint task NOT built.

PRE-MERGE SMOKE = ✅ SHIP (2026-07-08). Live: Today 26ms / Overview 23ms / Insights 87ms uncached;
cache 5 clean mentor rows (hero_koan_mature 1 + insight_recurring_pattern 4); Today renders mentor-voice;
0 third-person leakage (guard working — 27 live rejections logged to public.logs); 9/9 curls 200 no 500s.
CAVEAT (non-blocking, pre-classified): verbose recs (sensei's own ~400-char why) fail ≤180 guard every
try → Overview shows RAW fallback (which itself contains "The developer…" — that's the REC GENERATOR
writing 3rd-person, not an insight-copy bug; guard only gates MODEL copy). 2 more followup tickets:
 - lift verbose-rec pass-rate: stronger "summarize aggressively, drop specifics to fit" prompt OR a
   larger hero detail budget (hero body = 2-3 sentences, 180 tight); OR fix rec generator to write
   tighter neutral-voice why. sensei's OWN overview is the visible sufferer (dogfooding project).
 - warm-path WARN diagnosability: parse_and_validate returns Option (no reason); add kind + reject
   reason (banned/over-limit/third-person) to the public.logs WARN context so pass-rate is tunable.

★★★ INSIGHT-COPY ROLLOUT — MERGING NOW (2026-07-08) ★★★
develop commits: d62edf3c (clippy chore) + 96b1349a (Today pipeline) + a43db8e2 (Overview+Insights B2).
DOING: commit run-state → make bump v=patch (bundle now includes insight_copy.ddl so fresh installs get
the table) → merge develop→main → push → back to develop.

★ INSIGHT-COPY ROLLOUT SHIPPED: v0.2.25 on main (`b2382855`), develop @ `47fcc41f`. subtrees synced.

BUILD C ✅ DONE + COMMITTED `5d4f89c5` (2026-07-08): get_project_overview_stats (pg_store.rs ~4414) now
DERIVES readyToShare + toMerge (was hardcoded 0). NO DDL, NO new status (honors "don't invent a status").
memory_scope ladder = {global,project,stack,task_type,module} (widens→global). No signature col.
  readyToShare = status::text IN (active,reinforced,battle_tested) AND scope::text<>'global' (promotable).
  toMerge = sum of memories sharing lower(btrim(title)) within project (dup-title groups>1; folded-title
    proxy, no signature). sum→::bigint (numeric won't map to i64). SQL validated live (sensei: 1/1/0).
  clippy 0, 1192 tests. NOT deployed yet (batch w/ next screen); running daemon pid 20630 lacks Build C.

═══ PHASE 2 "make the 6 shipped screens real" = ✅ COMPLETE (insight-copy Today/Overview/Insights +
memory counts). Rec generation already exists (337 recs) — no generator gap. ═══
NEXT = PHASE 3 overflow screens (NEW: backend endpoint + frontend svelte each, gated loop):
  observatory-memories, project-sessions, project-memories. project-sessions lowest-risk
  (list_all_sessions already takes ?project=). Frontend=svelte:svelte-file-editor MANDATORY; shared
  api.ts/types.ts/state → DON'T parallelize frontend (conflict). 24 named tokens only. Next deploy =
  make install-debug (picks up Build C). THEN Phase 4 breadth (~30) → Phase 5 Dōjō (Supabase+kavach).

═══ PROJECT-WINDOW AUDIT (2026-07-08, daemon pid 31736 = current develop deployed, Build C LIVE
confirmed: sensei overview memories {total 1, readyToShare 1, toMerge 0}) ═══
KEY: NO frontend stubs — all 10 project-window screens fetch real /api/projects/{id}/* endpoints. Gaps
are SPEC-DIVERGENCE, not missing screens. Per-screen:
  overview      REAL (Build C counts now correct; sensei single-folder so multi-repo chip N/A)
  traceability  REAL-minor (/drift 200 items rich; missing coverage-summary + confidence/auto-fix chips;
                200-broken/0-drifted classification skew worth a glance)
  sessions      SPEC-GAP: plain filter+table, spec wants observatory-sessions chart variants (trend/
                stream/constellation/bands + range chips). Live rows task=""/model=null (capture gap).
  memories      SPEC-GAP: has batch-share flow; missing generalise action (POST /memories/{id}/generalise),
                ready-to-share hero, generalised chip, widen-scope ladder.
  about         SPEC-GAP: edits 5 identity fields; missing folder membership add/remove, split-project,
                dōjō binding strip, icon picker w/ inferred source.
  ★impact       SPEC-GAP → BUILDING NOW: READ-PATH BUG — loads getProjectRecommendations(id,'accepted')=0
                rows → EMPTY, while 7 REAL measured verdicts sit UNUSED at getProjectImpact /api/projects/
                {id}/impact (camelCase baselineFtr/currentFtr/ftrDelta/reasoning MOE panel). Delegated to
                svelte agent: repoint loader + render verdicts+reasoning + FTR trend (from verdict points
                if timestamps exist, else defer chart to backend series). Keep manual impact-log lane.
  instruments   SPEC-GAP (larger, partly parked): services real+toggle; /mcp-tool-stats 19 rows all 0
                calls (registry↔usage join — Slot-2 park); not the 3-tab Playground/Replay/Health shell.
  libraries     SPEC-GAP: /libraries 62 real browse/search/conflict; MISSING primary action = one-click
                wrap (POST .../libraries/{id}/wrap + scaffold gen + wrap-me hero + docs button).
  patterns      SPEC-GAP (gated on upstream): /patterns followed=1 anti=208 raw churn (null confidence/
                desc/family); missing family taxonomy filter + state filter + promote-to-rule.
RANKED next after impact: 2.libraries wrap-action (new endpoint+scaffold gen), 3.sessions chart-variant
port (reuse observatory charts), 4.patterns family+promote (needs better upstream classification first).

★ IMPACT ✅ DONE + COMMITTED `b19cb0e4` (2026-07-08): read-path repointed getProjectRecommendations
('accepted'=0)→getProjectImpact (7 verdicts render w/ badge + baseline→current FTR + MOE reasoning
panel). DRY WIN: promoted verdict→tone + bucketing → $lib/impact.ts, MOE panel → $lib/components/
MoeReasoningPanel.svelte; observatory-impact peer now consumes both (aggregate.ts deleted). svelte-check
0, 867 tests (+18). Manual impact-log lane kept. TREND CHART DEFERRED (honest): all 7 verdicts=pending,
baseline/current FTR null, no applied_at → nothing to plot until recs accepted+MeasureVerdicts populates
applied_at+FTR = DAEMON FOLLOWUP. NOT deployed (frontend; batch w/ next app-dev). NOT merged to main.

★ SESSIONS ⏳ BUILDING (svelte agent): port observatory chart variants (trend/stream/constellation/bands
+ range chips + totals + quality tally) to project-sessions, DRY-promote observatory charts+SessionRow+
sessions-digest → $lib (repoint observatory too, keep its e2e markers). Data = /api/sessions?project=&
range= (already supported). folder-role chip DEFERRED (list_all_sessions lacks folder_role; sensei is
single-folder) = small daemon followup. Row-click→Replay. No synthetic rows.

DAEMON FOLLOWUPS accumulating (batch into a backend session later):
 - impact: populate verdict applied_at + baselineFtr/currentFtr on accept+MeasureVerdicts (unblocks trend chart)
 - sessions: add folder_role to list_all_sessions (LEFT JOIN sensei.folders) for multi-repo chip
 - capture gap: session task=""/model=null on live rows
 - (earlier) insight-copy verbose-rec pass-rate, warm WARN reason, 30d eviction; project_patterns DDL
   idempotency; /api/gateway/infer temperature hardcoded.
DEPLOY/MERGE cadence: frontend screen fixes (impact, sessions) are on develop uncommitted-to-main;
batch a develop→main merge + app-dev visual e2e after a few screens land.

★ SESSIONS ✅ DONE + COMMITTED `7313e900` (2026-07-08): DRY-promoted observatory charts+SessionRow+
sessions-digest → $lib (git renames), rebuilt project-sessions w/ 4 chart variants + range chips scoped
/api/sessions?project=&range=. Observatory unchanged (import-only diff, e2e markers kept). svelte-check
0, 871 tests. folderRole chip multi-repo-ready (renders only when populated) — daemon followup: add
folder_role to list_all_sessions LEFT JOIN sensei.folders. e2e specs repointed (NOT run — needs Tauri).

MILESTONE CHECKPOINT (2026-07-08): merging Build C `5d4f89c5` + impact `b19cb0e4` + sessions `7313e900`
→ main + bump. Frontend verified at unit/check/autofixer level (incremental bar); FULL Tauri visual
e2e (make test-app-e2e = expensive app-e2e-build) BATCHED for later once more screens land. e2e mode=
'tauri' needs pre-built .app (no vite dev server). Then NEXT = memories (big: DDL generalised flag +
generalised_content + POST generalise endpoint [LLM rewrite, reuse insight-copy/reasoning pattern] +
ready-to-share hero + widen-scope submenu [existing /api/knowledge/memories/{id}/promote]) OR libraries
wrap-action (audit #2; needs wrap endpoint+scaffold-gen, "wrap" semantics = design-fork, check spec).

★★★ MILESTONE v0.2.26 → MAIN `08826d9c` (2026-07-08): Build C + impact + sessions. develop @ `1247b607`.
subtrees synced (tap 79a2e13, mkt eea24ea). FULL Tauri visual e2e BATCHED (run make test-app-e2e once
more frontend lands; exercises repointed impact/sessions e2e specs). main↔develop aligned.

★ MEMORIES ⏳ BACKEND BUILDING (general-purpose): DDL sensei.memories +generalised bool +
generalised_content text (live ALTER idempotent); POST /api/knowledge/memories/{id}/generalise (LLM
rewrite via reasoning chain, corrections_llm pattern, 10s timeout, 503 on unavailable=honest degrade);
expose generalised/generalisedContent in get_project_memories; PgStore::set_memory_generalisation.
Widen REUSES existing promote_memory. After backend → FRONTEND (ready-to-share hero, generalised chip,
both versions, widen submenu→promote). Existing batch-share flow already works — this ADDS to it.

★ MEMORIES BACKEND ✅ DONE (staged, NOT committed): DDL +generalised/+generalised_content live;
generalise_memory handler (knowledge.rs, reasoning chain, 10s timeout, 404/422/503/200); route
registered; get_project_memories exposes generalised+generalisedContent (handler unchanged — forwards
PgStore JSON); set_memory_generalisation(id,&str)->Result<Option<Uuid>,String>. 5 pure tests, 1197
suite pass, clippy 0. WIRE: POST .../generalise → 200 {id,original,generalised} | 503 {error}.
Agent flag: 2 pre-existing NON-ISOLATED DB tests (list_memories_filters_by_status, prune_activity...)
race under parallelism — not our code; run suite single-threaded. ⏳ DEPLOY+VERIFY in flight (make
install-debug + curl generalise on real memory → expect 200 faithful rewrite OR 503 cold-gemma honest
degrade). If WORKS/DEGRADES-cleanly → COMMIT backend → delegate FRONTEND. If BUG → fix.

OLD NOTES BELOW (pre-insight-copy, historical) ↓↓↓
THEN Build C (memory ready_to_share/to_merge read-path derivation per the design-fork note above).
Build-B emission points located: observatory_home.rs pure fns early_hero/mature_hero/steady_hero/
rec_to_insight_card (return serde_json::Value; keep as FALLBACK producers, route their koan+body/text
through generate_insight_copy at the ASYNC handler boundary — don't make the pure fns async). Same
pattern for project_overview.rs hero + insights.rs.

## PHASE 2 design-fork note — memory ready_to_share / to_merge (defer to Build C, after A+B)
SPEC GAP: project-overview.md says the ready_to_share/to_merge sub-counts use "the promotion-/merge-
readiness statuses defined in pipeline/memory — do not invent a status name", BUT pipeline/memory.md
defines a scope LADDER (project→user→org→collective via scope_level) and NEVER names ready_to_share/
to_merge as memory_status values. memory_status enum = proposed|active|reinforced|challenged|
battle_tested|archived|rejected (no share/merge value). DEFAULT-AND-PROCEED decision (reversible read-
path derivation, NOT a new enum value — which honors "do not invent a status name"):
  ready_to_share = COUNT memories WHERE status IN (active,reinforced,battle_tested) AND scope_level
    tighter than collective (i.e. promotable to the next ladder rung).
  to_merge = COUNT memories that have a duplicate `signature` within an overlapping scope (dedup
    candidates — the "merge" readiness the ladder's strength-recompute step implies).
So Build C is likely READ-PATH ONLY (no DDL). Confirm columns (scope_level, signature) exist before
building. Revisit if a later spec doc names these statuses explicitly.

## Slot 2 UN-PARKED — Jerry decided the data model (2026-07-08)
User authorized the capture/DDL build (overrides run off-limits). DECISION:
- **Unified inventory + typed config.** New `sensei.assistant_tools` table = one row per
  registered tool: (source_type: mcp|plugin|builtin, source_key, tool_name, + display meta).
  `tool_type` lives on the inventory. CONFIG stays on the typed source tables:
  `mcp_servers` (MCP launch/connect config), `extensions` (plugins). Built-ins need no config.
- **Full capture (all sources).** Populate assistant_tools from: (a) MCP — read Claude Code's
  MCP config (`.mcp.json` + plugin-provided MCPs), register into mcp_servers, probe →
  mcp_tool_manifests → explode tools into assistant_tools; (b) plugins — ingest marketplace
  plugin manifests into `extensions` → assistant_tools; (c) built-ins — static per-harness catalog.
- **L1 grid** = `assistant_tools` (registry) ⟕ `tool_usage_stats` (usage) grouped by source →
  `share_invoked = tools_invoked_14d / tools_registered`. Card config from the typed source table.
- Reality found: tool_usage_stats sources = (builtin) 21 tools/46,591 calls; plugin_playwright,
  plugin_sensei (11), svelte/plugin_svelte (naming INCONSISTENT — normalize source_key), playwright.
  extensions table EMPTY. mcp_tool_manifests has server_id/tools jsonb/tool_count/probed_at.
- LOCKED design (default-and-proceed, 2026-07-08):
  - `sensei.assistant_tools` inventory = usage-joinable tools only (source_type ∈ mcp|builtin),
    cols: assistant_family, source_type, source_key, tool_name(bare), invoked_name(harness-qualified,
    joins tool_usage_stats.tool_name), description, server_id FK→mcp_servers. Config stays on
    mcp_servers (MCP). Plugins are NOT in this table (not invoked as distinct usage tools) — they're
    captured into `extensions` (typed home, for the Instruments/skills surface) during the burn-down.
  - Discovery = `ToolDiscovery` trait per assistant (refactor mcp_discovery.rs AcpFamily+parse into it).
  - Capture flow: discover MCP servers (per-assistant config) → upsert mcp_servers → probe_tools →
    mcp_tool_manifests → explode into assistant_tools. Built-ins → static per-harness catalog →
    assistant_tools. BRIDGE (the crux): a probed server's bare tools T match a tool_usage_stats prefix
    P iff `mcp__P__t` exists for t∈T → invoked_name = mcp__P__t (tool-set matching reconciles the
    forward-registry↔reverse-usage naming mismatch, e.g. playwright vs plugin_playwright_playwright).
  - Grid endpoint: per-source {name, source_type, connected, tools_registered, tools_invoked_14d
    (last_used_at within 14d), share_invoked} = assistant_tools GROUP BY source ⟕ tool_usage_stats.
- PROGRESS: assistant_tools.ddl written+APPLIED live (21 builtin rows). Backend v1 SHIPPED by fork
  (uncommitted): tool_discovery.rs (ToolDiscovery trait + ClaudeCode/Zed/Cursor + bridge + 4 tests),
  6 pg_store methods, tools_health handler (refresh+grid), 2 routes. Grid LIVE: builtin card real
  (registered 21, invoked 19, share 0.905); MCP cards honest usage-only (registered null). clippy clean.
  Endpoints: `GET /api/instruments/tools-health` {sources:[{assistant_family,source_type(mcp|builtin),
  source_key,name,connected,connection_state,server_id,tools_registered|null,tools_invoked_14d,
  calls_14d,share_invoked|null}]} + `POST /api/instruments/tools/refresh`. Fork RESUMED to extend
  ClaudeCode discovery → CC plugin MCPs (~/.claude/plugins/**/.mcp.json + ~/.claude.json mcpServers)
  → probe (sensei-mcp runnable) → real registered/share; + startup trigger. Then: spec rewrite →
  gate1 → frontend L1 (svelte-file-editor) → gates 2/3/4 → commit → PHASE 1 milestone merge+bump.
- BACKEND ✅ COMPLETE+VERIFIED (full capture): plugin-MCP discovery wired (ClaudeCode scans
  ~/.claude.json + ~/.claude/plugins/**/.mcp.json + sensei config.json), probe w/ cwd, startup capture.
  assistant_tools=21 builtin + 67 MCP across 4 probed sources. Grid LIVE with REAL shares: builtin
  0.90, sensei 0.18 (33 reg), playwright 0.48 (23), svelte 0.25 (4), semgrep 0.00 (7); honest
  usage-only for plugin_svelte_svelte (cache command:null) + bare playwright + 2 Zed (not runnable).
  clippy clean, 6 tests. Files: tool_discovery.rs, mcp_probe.rs (cwd), tools_health.rs, mcp_servers.rs,
  server.rs (startup), pg_store.rs, routes.rs, main.rs, assistant_tools.ddl. FOLLOWUP: merge duplicate
  svelte/playwright cards (normalization). NEXT: spec rewrite → gate1 → frontend L1 → gates → commit → merge+bump.
- FRONTEND RECON: instruments = single `(observatory)/instruments/+page.svelte` (tabs: replay/insights/…),
  data on `mcp` store (`$lib/state/mcp.svelte.js`). Health surface = the **'insights' tab ("Toolset
  health.")**, currently renders OLD server list from `mcp.mcpServers`. L2 already fetched (tool-signals/
  tool-insights via api.ts). Frontend task: add `getToolsHealth()` + a `toolsHealth` slice on the mcp
  store; render the NEW L1 share grid (card per source, share bar OR "registered —", KPI header) in that
  tab; keep the existing L2 drill on card click. Honest-degrade: share null → "invoked N · registered —".
- SPEC REWRITTEN to built model (健 title, tools-health endpoint, assistant_tools, honest-degrade, KPIs,
  L2 param-free note, done/wrong gates updated, fences balanced). SKIPPED redundant gate1 spec-doc-reviewer
  (backend already built+verified live; frontend builds vs live wire+mockup; gates 2/3 verify vs running
  system). FRONTEND L1 grid delegated to svelte-file-editor → then gates 2/3/4 → commit → PHASE 1 merge+bump.
- FRONTEND ✅ GREEN: check 0/0 (874 files), test 851 (+13). ToolHealthCard(+harness+spec, share-bar +
  null "registered —" variants), mcp store toolsHealth slice+KPIs(+spec), L1 grid + L1↔L2 drill in
  instruments/+page.svelte (Health = 'insights' tab). Gates 2(done)+3(wrong) RUNNING (read-only). On
  clear → persona → commit Slot 2 → PHASE 1 MILESTONE: merge develop→main + make bump. KNOWN followup:
  merge duplicate svelte/playwright cards (source-key normalization).
- Gate 2 done: ready-to-ship (all 9 code/API pass). Gate 3 wrong: one-or-more-tripping — FIXING:
  (A) calls_14d was all-time not 14d → fork windowing to real 14d from assistant_events; (B) name=raw
  source_key → fork prettifying (plugin_sensei_sensei→"sensei"); (C) missing 一 first-try KPI → add in
  frontend pass (reuse holistic FTR). DEFERRED (default-and-proceed): duplicate cards dedup (bare vs
  plugin prefix may be distinct servers; keep-both honest = safe reversible → followup). Backend fork
  RESUMED (A+B), persona gate RUNNING. On both: frontend pass (一 KPI + persona items) → re-verify →
  commit Slot 2 → PHASE 1 MILESTONE merge develop→main + make bump.
- Gate 4 persona: Health grid does its job; found distinct cheap P0s for the FRONTEND PASS (batch w/ 一 KPI):
  P0-A text-error→text-danger (remove <style> color block, +page.svelte ~1185); P0-B add Verdict chip
  6th column to per-tool table (~1052); P0-C loading guard (read mcp.toolsHealthStatus before empty state
  ~894); P0-D remove debug "源 source" L2 KPI (~1009); P0-E delete dead CSS (.tool-card/.param-input);
  + C add 一 first-try KPI (reuse holistic FTR). (Persona wrongly said no ToolHealthCard tests — they
  exist, 851 tests.) Minor backend: uncovered CTE hard-codes assistant_family='claude' (fine for now,
  followup for multi-harness). AWAIT backend fork (A/B) → then ONE frontend pass (all above) → re-verify
  → commit → milestone.
- BACKEND A/B ✅ VERIFIED: calls_14d now real 14d (16,225 not 48,721; evt CTE over assistant_events,
  ts is bigint epoch-ms → `ts >= (extract(epoch from now()-'14d')*1000)::bigint`); names prettified
  (built-ins/sensei/playwright/…). clippy clean, 7 tests. FRONTEND polish pass (P0-A..E + 一 KPI)
  RUNNING. On green → re-verify → commit Slot 2 (backend+frontend+DDL) → PHASE 1 MILESTONE merge+bump.
- BUILD ORDER (gated): update spec instruments-health data-model → spec-doc-reviewer → DDL
  (assistant_tools .ddl + apply via dbd) → capture (builtins catalog first, then MCP probe, then
  plugin ingest) → endpoint (grid) → frontend L1 (L2 already works) → gates → commit.
- DDL WORKFLOW (memory): edit .ddl SOURCE first, then apply via `dbd deploy`/`apply` (NOT combine);
  daemon reads live DB for queries (apply to live sensei DB works immediately); boot auto-apply
  reads RELEASED bundle → set SENSEI_DDL_DIR or make bump for boot; `make dbd-cache-clear` after bump.

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
