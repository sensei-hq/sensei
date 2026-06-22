---
title: Session/Log Analyzer
description: Periodic daemon subsystem that turns captured hook events into enriched session metrics, detected patterns, learnings (memories), recommendations, and a per-project maturity signal.
type: blueprint
status: blueprint
created: 2026-06-22
depends_on: []
related_issues: [65, 59, 31, 38]
references:
  - inference.recommendations, inference.detected_patterns, inference.reasoning_traces (output tables — DDL exists, no writers)
  - sensei.memories, sensei.project_patterns, sensei.memory_outcomes (output tables)
  - activity.hook_events, activity.sessions (input)
  - crates/senseid/src/tasks (task queue + TaskKind), progress_emitter.rs (long-lived tokio task pattern)
  - crates/senseid/src/governance.rs + api/handlers/knowledge.rs (Tier-2 consolidation / inference-role pattern)
  - docs/mockups/Sensei/lib/learnings-v2.jsx + observatory.jsx (output contract — redesign themes A & F)
---

# Session/Log Analyzer

## Objective
Stand up the missing **generation layer**: a periodic daemon subsystem that reads captured activity and writes the learnings, patterns, recommendations, and maturity signal that the observatory and learnings screens render. The shaping design decision — forced by the data: `activity.events` is empty and `activity.sessions` carry no metrics, so **everything is derived from `activity.hook_events`**, and the pipeline's first and load-bearing stage is *session enrichment* (without it every FTR/outcome in the product is null).

## Data availability (grounding — the analysis this is built on)
Live counts (2026-06-22, `sensei` DB):

| Source | Rows | Usable signal |
|--------|------|---------------|
| `activity.hook_events` | **21,093** | PreToolUse 10,057 · PostToolUse 9,876 · UserPromptSubmit 391 · Stop 307 · SubagentStop 227 · SessionStart 48 · SessionEnd 7 · PreCompact 6. Each row: `session_id`, `assistant_family`, `event_type`, `tool_name`, `cwd`, `ts`, `success`, full `payload`. **This is the only rich signal.** |
| `activity.sessions` | 18 | Skeletal — `folder_id`/`project_id`/`started_at`/`completed_at` only (from the #31 hook-derivation). **`outcome`, `ftr`, `turns`, `corrections`, `tokens_*`, `duration_ms`, `module` are all NULL.** |
| `activity.events` | **0** | Empty — do NOT depend on it; derive turn/tool structure from hook_events. |
| `sensei.memories` | 0 | Writer-less (this fills it). |
| `inference.recommendations` | 0 | Writer-less (this fills it). |
| `inference.detected_patterns` / `sensei.project_patterns` | (pattern tables, unpopulated) | Writer-less. |

**Consequence:** `get_project_ftr` reads `sessions.ftr` → returns 0 everywhere; the observatory's FTR strip, session outcomes, and "sessions watched" are all blank *because nothing enriches sessions*. Session enrichment (L0) is therefore the linchpin, independently valuable even before recommendations exist.

**Derivable from hook_events per session** (`session_id` already links them): turns = count(UserPromptSubmit); tool usage = tallies of PreToolUse/PostToolUse by `tool_name` + `success`; corrections = correction heuristics over UserPromptSubmit payload text (revert/undo/"actually"/"no,"/repeat-edit-after-failure) — the same `triage_signal` vocabulary `sensei.memories` already defines; outcome = Stop/SessionEnd present → `completed`, crash/no-end → `abandoned`, error-cluster → `blocked`; ftr = corrections == 0; duration = last_ts − first_ts; module = dominant `cwd`/edited path.

## Architecture
```mermaid
flowchart TD
  HE[activity.hook_events 21k] --> L0
  subgraph Pipeline [TaskKind::AnalyzeProject · per project · idempotent]
    L0[L0 SessionEnricher\nhook_events → session metrics] --> L1
    L1[L1 SignalDeriver\nenriched sessions + code graph → patterns / anti-patterns / FTR trends] --> L2
    L2[L2 Generator\nfindings → memories + recommendations]
    L2 -->|heuristic tier| H[deterministic recs\nreasoning_trace_id = null]
    L2 -->|consolidation tier| C[gateway 'consolidation' role\n→ reasoning_trace → rec]
  end
  CG[(code graph\nnodes/edges)] --> L1
  L0 --> SES[(activity.sessions\nenriched)]
  L1 --> DP[(detected_patterns\nproject_patterns)]
  L2 --> MEM[(sensei.memories\norigin=learned)]
  L2 --> REC[(inference.recommendations\nstatus=pending)]
  Pipeline --> MAT[L3 MaturityModel\nper-project early|mature]
  SCHED[AnalyzerScheduler\nhourly tokio interval + watermark guard] -->|enqueue| Pipeline
  API[/API + MCP readers/] --> SES & REC & MEM & MAT
  REC --> MV[existing MeasureVerdicts task\nbefore/after FTR]
```

## Components

### SessionEnricher (L0) — the linchpin
Per session (or per recently-active session since the watermark), reads its `hook_events` ordered by `ts` and computes the session metrics, then `UPDATE activity.sessions` with `outcome`, `ftr`, `turns`, `corrections`, `tokens_in/out`, `duration_ms`, `module`, and a `props` breakdown (tool_usage map, triage signals). Pure, deterministic, idempotent (recompute overwrites). Owns the correction/outcome heuristics. Depends only on hook_events. **Delivers FTR/outcomes to the whole product immediately** — ship and validate this alone first.

### SignalDeriver (L1)
Consumes enriched sessions + hook_events + the code graph. Produces:
- **detected_patterns** — recurring tool/edit sequences, repeated file/module touches, recurring task shapes; writes `inference.detected_patterns` + rolls project-scoped ones to `sensei.project_patterns` with a confidence/support count.
- **anti-patterns** — correction/failure clusters (same file or tool repeatedly correcting).
- **FTR trends** — per project + per module, over the rolling window, plus deltas.
Read-mostly aggregation; no LLM. Owns "what is worth surfacing" thresholds (min support, recency).

### Generator (L2) — memories + recommendations
Maps findings to the two human-facing outputs, in two tiers (mirrors governance Tier-1 heuristic / Tier-2 consolidation):
- **Memories (learnings)** → `sensei.memories` (`origin='learned'`, `type` ∈ pattern/convention/preference, `triage_signal`, `strength`). New findings create memories at strength 1.0; recurring evidence reinforces (`reinforced_count`, strength +); contradicted evidence challenges (`violated_count`); decayed → archived. This is the reinforce/decay loop the learnings screen's Reinforce/Mute drives.
- **Recommendations** → `inference.recommendations` (`status='pending'`). The redesign's typed `kind` maps onto the existing `action_type` column:

  | redesign kind | `action_type` | trigger |
  |---|---|---|
  | promote-pattern | `promote_pattern` | a detected_pattern crosses support/confidence → propose as a rule/memory |
  | create-agent | `create_persona` | recurring task type with no persona |
  | write-skill | `enable_skill` | repeated manual tool sequence → skill |
  | archive-memory | `audit_stale` | memory decayed / unused N days |
  | enrich-memory | `revise_rule` (reinforce) | memory with accumulating evidence |
  | cross-project | `cross_project` | pattern/memory seen in ≥2 projects |

  Each rec carries `title`, `why`, `impact`, `evidence` (`[{session_id,file,description}]`), `action_detail`, `prompt`, `urgency`. **Heuristic tier**: deterministic, `reasoning_trace_id = null`, cheap, runs every cycle. **Consolidation tier**: batches candidate findings through the gateway `consolidation` inference role (the Tier-2 LLM-merge pattern in `governance.rs`/`knowledge.rs`), writing an `inference.reasoning_traces` row and a higher-confidence rec with a written `why`/`prompt`. Acceptance is human-in-loop via the existing `accept_proposal`/`reject_proposal` MCP tools + recommendations API (redesign theme F Apply/Review/Dismiss → status transitions).

### MaturityModel (L3)
Per-project signal the observatory lands on (theme A): `early` until (enriched sessions ≥ target ~3 AND ≥1 generated insight), else `mature`; plus the raw `sessions_watched` / `target` for the first-session meter. Pure function over counts; exposed read-only.

### AnalyzerScheduler
Long-lived tokio task (same shape as `progress_emitter::spawn`) that ticks on an interval (hourly) and, for each project with `max(sessions.started_at) > last_analyzed_at`, enqueues `TaskKind::AnalyzeProject`. A per-project `last_analyzed_at` watermark (timestamp guard, like the DDL seeding guard) makes re-runs cheap and idempotent. Also triggerable on demand (API/MCP) and opportunistically on `SessionEnd` ingestion.

## Data flow
hook_events (ingested live by #31) → **scheduler tick** detects new sessions per project → enqueues `AnalyzeProject` → **L0** enriches those sessions (UPDATE) → **L1** derives patterns/trends from the now-metriced sessions + code graph → **L2** upserts memories and pending recommendations (heuristic now; consolidation batch for the strongest candidates) → **L3** recomputes maturity → readers (`get_project_ftr`, `get_project_recommendations`, memories, maturity) serve the observatory/learnings UI → user Apply/Dismiss flips rec `status` → existing `MeasureVerdicts` measures before/after FTR.

## Integration points
| Integration | Method | Notes |
|-------------|--------|-------|
| Task queue | new `TaskKind::AnalyzeProject` (+ watchdog timeout = long tier) | reuses executor/retry/watchdog |
| Gateway | `consolidation` inference role via `infer`/gateway | L2 consolidation tier only; heuristic tier has no LLM dep |
| Code graph | read nodes/edges (#57 edges populated) | L1 pattern/module derivation |
| Readers (unchanged) | `get_project_ftr`, `get_project_recommendations`, memories list, new maturity endpoint | outputs match existing wire types |
| Acceptance | `accept_proposal` / `reject_proposal` MCP + recommendations API | redesign theme F |
| Verdict loop | existing `MeasureVerdicts` task | rec before/after FTR (already wired on session end) |

## Dependencies
| Dependency | Status | Impact if missing |
|-----------|--------|-------------------|
| `activity.hook_events` populated | ✅ 21k rows | none |
| Sessions linked to hook stream (#31) | ✅ shipped | L0 can't attribute events |
| Code graph edges (#57) | ✅ shipped | L1 loses call/module patterns (degrade, not block) |
| Gateway `consolidation` role + a local model | ⚠️ exists for governance; confirm a chat model is assigned (cf. gateway-chat-model pref) | consolidation tier degrades to heuristic-only |
| DDL writers for detected_patterns/project_patterns | ❌ this builds them | — |

## Implementation order (bottom-up)
1. **L0 SessionEnricher + `TaskKind::AnalyzeProject` skeleton** — enrich the 18 existing sessions; FTR/outcomes light up across the product. Validate against the observatory before going further. *(Highest value, no LLM, no UI dep.)*
2. **AnalyzerScheduler** — hourly tick + watermark; on-demand trigger.
3. **L1 SignalDeriver** — detected_patterns + project_patterns + FTR trends.
4. **L2 Generator — heuristic tier** — memories (reinforce/decay) + pending recommendations (the 6 kinds), deterministic.
5. **L2 Generator — consolidation tier** — gateway role → reasoning_trace → enriched recs.
6. **L3 MaturityModel + maturity endpoint** — unblocks observatory theme A.

Steps 1–2 are independently shippable and unblock the redesign's metrics; 4 + 6 unblock redesign themes F + A.

## Personas
`.sensei/personas/` not present at blueprint time. When defined, re-check L2 (does a recommendation serve the persona who must act on it?) and L0 (are the derived metrics ones they trust?).

## Out of scope
- The **UI** for themes A/F (built separately against these outputs).
- **Federation/insight sharing** (`inference.insights`/`insight_batches`) — separate path.
- The **verdict measurement** loop internals (`MeasureVerdicts` exists; this only feeds it pending recs).
- Backfilling/altering DDL — tables already exist; this adds writers only.
