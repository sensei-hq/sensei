# Observatory Gap Analysis

> Mockup data requirements vs current DDL, views, pipeline, and API.
> Date: 2026-05-12

## Methodology

Every data point visible in the observatory mockups (`docs/mockups/lib/observatory.jsx`, `project-shared.jsx`, `project-data.js`, `impact.jsx`, `libraries.jsx`, `instruments-simple.jsx`, `sessions-zen.jsx`) was traced to the DDL table or view that would serve it. Status:

- **HAVE** — DDL + PgStore method + API endpoint exist today
- **VIEW** — DDL tables exist; need a SQL view or PgStore query to shape the data
- **DDL** — need a new table, column, index, or enum value
- **PIPE** — need a scan/task/hook change to capture the data
- **BLOCKED** — requires ACP feature that doesn't exist (token counts, headless sessions)

---

## 1. Observatory Home (`ObsHome`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| FTR 14-day aggregate | `activity.sessions` | **HAVE** | `project_ftr_metrics` view computes `ftr_14d`, `ftr_14d_prev` |
| FTR 14-day sparkline (14 data points) | `activity.sessions` | **VIEW** | Need `ftr_daily` view: one row per day per project with daily FTR rate |
| Holistic FTR sparkline (all projects) | `activity.sessions` | **VIEW** | Same view but without project filter, or a `holistic_ftr_daily` variant |
| Session count (7d) | `project_ftr_metrics.sessions_7d` | **HAVE** | Already in view |
| Hero koan (most important insight) | `inference.recommendations` | **VIEW** | Table exists with full lifecycle. Need query to pick highest-urgency pending recommendation |
| Insights list (up to 3) | `inference.recommendations` + `inference.detected_patterns` | **VIEW** | Both tables exist. Need a ranked union query |
| Adopted teachings | `inference.detected_patterns` | **VIEW** | `WHERE lifecycle = 'rule' ORDER BY modified_at DESC` |
| Recent sessions list | `activity.sessions` | **HAVE** | `list_sessions_by_folder` / `list_sessions_by_project` exist |
| Per-project sidebar (FTR + warn) | `project_ftr_metrics` | **HAVE** | FTR per project computed; warn = FTR declining |
| Greeting (time-of-day) | N/A | **N/A** | Pure client-side logic |

### Summary: 0 DDL changes, 3 views needed

---

## 2. Project Overview (`ProjOverview`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| FTR 14d + sparkline per project | `activity.sessions` | **VIEW** | Same `ftr_daily` view scoped to project |
| Sessions 7d | `project_ftr_metrics` | **HAVE** | Already computed |
| Repos in project | `sensei.folders` + `sensei.projects` | **HAVE** | Repository listing exists |
| Hotspots (files by rework count) | `activity.events` (type=`edit`) | **VIEW+PIPE** | `edit` events exist in enum but not systematically captured. Need: (a) hook to write `edit` events with file_path, (b) `project_hotspots` view aggregating edit+correction events per file |
| Recommendations list | `inference.recommendations` | **HAVE** | Full table with urgency, evidence, prompts |
| Patterns followed | `inference.detected_patterns` | **HAVE** | `WHERE NOT is_anti_pattern` |
| Anti-patterns | `inference.detected_patterns` | **HAVE** | `WHERE is_anti_pattern` with severity, instances, fix_pattern_id |
| Quality signals (zero-errors, pattern compliance, doc drift, test coverage) | Mixed | **VIEW** | Need `project_quality_signals` view combining: FTR as zero-errors proxy, pattern compliance from `detected_patterns`, drift count from `drift_items`, test results from `events` (type=`test`) |

### Summary: 0 DDL changes, 2 views needed, 1 pipeline change (edit event capture)

---

## 3. Project Graph (`ProjGraphLens`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| Node list (file-level) | `sensei.nodes` | **HAVE** | `get_nodes_by_folder` |
| Edges (calls, imports, implements) | `sensei.edges` | **HAVE** | Full edge model |
| Fan-in / fan-out per node | `sensei.nodes.degree` + edges | **HAVE** | `degree` precomputed on nodes |
| Rework overlay (sessions touching a file) | `activity.events` | **VIEW+PIPE** | Same as hotspots — need edit events with file_path |
| Staleness overlay (days since modified) | `sensei.nodes.modified_at` | **HAVE** | `now() - modified_at` |
| Community coloring | `sensei.nodes.community_id` | **HAVE** | Just implemented label propagation |
| Duplicate clusters | `inference.detected_patterns` | **HAVE** | Anti-patterns with type=duplication, instances field |
| Pattern overlay | `inference.detected_patterns.instances` | **HAVE** | Instances have file+line |

### Summary: 0 DDL changes, 0 new views (reuses hotspot view), 0 pipeline changes

---

## 4. Impact Reports (`ObsImpact`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| Report list with verdict | `inference.recommendations` | **HAVE** | `verdict` enum: pending, positive, negative, neutral |
| Before/after FTR | `recommendations.baseline_ftr` / `current_ftr` | **HAVE** | Columns exist |
| Before/after corrections | `recommendations.props` | **HAVE** | Extensible JSONB for `{baseline_corrections_avg, current_corrections_avg}` |
| Measurement window | `recommendations.acted_at` / `measured_at` | **HAVE** | Both columns exist |
| FTR delta | Derived | **HAVE** | `current_ftr - baseline_ftr` |
| Reasoning narrative | `inference.reasoning_traces` | **HAVE** | Linked via `reasoning_trace_id` |

### Summary: 0 DDL changes, 0 views needed. Table is fully designed.

---

## 5. Sessions (`SessionsDigestZen`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| Session list (outcome, FTR, corrections, duration) | `activity.sessions` | **HAVE** | All columns present |
| Event timeline per session | `activity.events` | **HAVE** | Ordered by `created_at` |
| Tool calls with params | `activity.events` (type=`tool_call`) | **HAVE** | `data.tool_name`, `data.input_params` |
| Corrections detail | `activity.events` (type=`correction`) | **HAVE** | `data.description`, `data.module` |
| Token usage | `sessions.tokens_in/out` | **BLOCKED** | Columns exist but ACP doesn't provide token counts |
| Duration | `sessions.duration_ms` | **HAVE** | Column exists |
| Summary | `sessions.summary` | **HAVE** | Column exists |

### Summary: 0 DDL changes. Token tracking blocked by ACP (FR-1).

---

## 6. Libraries (`LibrariesVariantA`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| Library list (name, version, ecosystem) | `sensei.libraries` | **HAVE** | Full table with `upsert_library` |
| Page count per library | `libraries.page_count` | **HAVE** | Denormalized column, updated by `update_library_page_count` |
| Library pages/sections | `sensei.library_pages` | **HAVE** | Just wired up IndexLibrary pipeline |
| Detected libraries (from manifests) | `sensei.referenced_libraries` | **VIEW** | Table exists. `extract_dep_versions` dead code parses manifests but isn't wired |
| Imported libraries (user-added) | `sensei.libraries` | **HAVE** | `add_library` MCP tool |
| Usage per library (call sites) | `sensei.edges` + `sensei.nodes` | **VIEW** | Need query: edges where target is external + group by lib name |
| Top symbols from library | Same | **VIEW** | Same edge data, grouped by symbol |
| Last indexed timestamp | `libraries.modified_at` / `library_pages.fetched_at` | **HAVE** | Columns exist |

### Summary: 0 DDL changes, 2 views needed, 1 pipeline change (wire `extract_dep_versions`)

---

## 7. Instruments (`InstrumentsPlaygroundSimple`, `InstrumentsReplaySimple`, `InstrumentsHealthSimple`)

| Mockup element | Source table(s) | Status | Gap |
|---|---|---|---|
| MCP tool catalog | MCP list_tools endpoint | **HAVE** | `mcp_list_tools` handler |
| Tool descriptions + params | Same | **HAVE** | Already in response |
| Usage stats per tool | `activity.events` (type=`tool_call`) | **VIEW** | Events exist; need aggregation query |
| Try-it simulation | `/api/mcp/call` | **HAVE** | Endpoint exists |
| Error rates | `activity.events` (type=`error`) | **VIEW** | Events with error type exist |
| Replay (session tool timeline) | `activity.events` per session | **HAVE** | Full event log |
| Health (signals + anomalies) | `activity.events` + heuristics | **VIEW** | Need aggregation: tool error rates, session failure trends |

### Summary: 0 DDL changes, 2 views needed

---

## Consolidated Change List

### DDL Changes (tables, columns, indexes, enums)

None required. The schema is complete for all mockup data points except token tracking (blocked by ACP).

### New Views

| # | View name | Powers | SQL shape |
|---|---|---|---|
| V1 | `sensei.ftr_daily` | Home sparkline, project sparkline | `(project_id, day, ftr_rate, session_count)` from `activity.sessions` grouped by `date_trunc('day', started_at)` |
| V2 | `sensei.project_hotspots` | Hotspots section, rework graph overlay | `(project_id, file_path, folder, edit_count, correction_count, last_touched)` from `activity.events` where type in (edit, correction) |
| V3 | `sensei.project_quality_signals` | Quality signals row | `(project_id, ftr_7d, pattern_compliance, drift_count, test_pass_rate)` combining sessions, patterns, drift_items, events |
| V4 | `sensei.tool_usage_stats` | Instruments health, tool usage in libraries | `(tool_name, call_count, error_count, avg_duration_ms, last_used)` from events where type=tool_call |
| V5 | `sensei.library_usage` | Library detail: top symbols, call sites | `(library_name, symbol_name, call_count, files)` from edges+nodes where target is external |

### Pipeline / Task Changes

| # | Change | Where | Dependency |
|---|---|---|---|
| P1 | Wire `extract_dep_versions` as `ExtractDeps` TaskKind | `tasks/handlers/libraries.rs` + `indexer/lib_indexer.rs` | Populates `referenced_libraries` for "detected" library group |
| P2 | Capture `edit` events with file paths during sessions | Hook configuration / event logging | Powers hotspot analysis and rework overlay |
| P3 | Verdict measurement job — periodically recompute recommendation FTR deltas | New task or cron | Powers impact reports |

### API / PgStore Changes

| # | Method | Powers |
|---|---|---|
| A1 | `get_ftr_daily(project_id, days) -> Vec<(date, ftr, count)>` | Sparkline data for V1 |
| A2 | `get_holistic_ftr_daily(days) -> Vec<(date, ftr, count)>` | Home sparkline (all projects) |
| A3 | `get_hotspots(project_id, days) -> Vec<(file, edits, corrections)>` | V2 query |
| A4 | `get_quality_signals(project_id) -> QualitySignals` | V3 query |
| A5 | `get_tool_usage_stats() -> Vec<ToolUsage>` | V4 query |
| A6 | `get_library_usage(library_id) -> Vec<(symbol, count, files)>` | V5 query |
| A7 | `get_pending_recommendations(project_id) -> Vec<Recommendation>` | Hero koan + insights |
| A8 | `get_adopted_teachings(project_id, limit) -> Vec<Pattern>` | Adopted teachings section |
| A9 | `measure_recommendation_verdict(rec_id)` | P3 verdict computation |

### Blocked (ACP Feature Requests)

| # | What | Impact | Workaround |
|---|---|---|---|
| B1 | Token counts per session (FR-1) | Token usage column, cost tracking, burn rate | Estimate from turn count |
| B2 | Headless session execution (FR-4) | Automated benchmarking | Manual benchmarking |

---

## Build Order

Following the requested sequence: DDL → views → pipeline → API → screens.

### Phase 1: DDL
No changes needed. Schema is complete.

### Phase 2: Views (V1-V5)
1. `ftr_daily` — unlocks sparklines everywhere
2. `project_hotspots` — unlocks hotspot section + graph overlay
3. `project_quality_signals` — unlocks quality row
4. `tool_usage_stats` — unlocks instruments health
5. `library_usage` — unlocks library detail

### Phase 3: Pipeline
1. Wire `extract_dep_versions` into task system (P1)
2. Configure edit event capture in hooks (P2)
3. Build verdict measurement logic (P3)

### Phase 4: API
1. PgStore methods for each view (A1-A9)
2. HTTP endpoints for desktop app consumption
3. MCP tool wrappers where needed

### Phase 5: Screens
Wire the SvelteKit app pages to the new API endpoints.
