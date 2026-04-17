---
id: implementation-phases
type: design
implements: []
---

# Implementation Phases

> A phase-based delivery plan for building Sensei incrementally — each phase ships one working capability, is TDD-first, and has a concrete acceptance criterion that can be verified by running the system.

## Approach

Each phase delivers a **complete, working capability** that a developer can observe end-to-end. Phases are not backend-only milestones. Every phase spans multiple packages because a capability is only real when it flows from code to storage to UI (or CLI output). No phase depends on work defined in a later phase.

**TDD-first** means: write the unit test for a package component first, implement until it passes, then wire up the integration. The e2e test for each phase is written against the "done when" acceptance criterion before implementation begins. A phase is not complete until all unit tests, integration tests, and the e2e test pass.

Package test strategy follows a consistent pattern:
- `shared` and `engine` tests run against a local Supabase Docker instance (`supabase start`)
- `collector` tests use a test HTTP server and a local Supabase instance
- `server` tests use the MCP SDK test client
- `cli` tests use child process spawning and assert on stdout/exit code
- Dashboard e2e uses Playwright against a dev server

---

## Phase 1: Foundation

### Goal
A developer runs `sensei init` on a TypeScript repository. Symbols are indexed into Supabase. The dashboard shows the repo with basic symbol stats and traceability status. An MCP-enabled agent can call `get_session_context`, `search`, and `load_context`.

### Packages Involved
`shared`, `engine`, `collector`, `server`, `cli`, `apps/dashboard`

### What Gets Built

- **packages/shared:**
  - Supabase client factory (`createClient`, connection config from env or config.yaml)
  - Config schema: parse `~/.config/sensei/config.yaml` and `.sensei/config.yaml` with Zod
  - Core domain types: `Repo`, `Symbol`, `CallEdge`, `Import`, `FileEntry`, `ScanResult`, `IndexResult`
  - Supabase migration: `repos`, `symbols`, `call_edges`, `imports`, `scan_state` tables

- **packages/engine:**
  - Scan stage: `Scanner` class — file discovery with fast-glob, mtime/hash fingerprinting, diff against `scan_state` in Supabase
  - Parse stage: `LanguageAdapterRegistry`, `TypeScriptAdapter` (ts-morph: extract exported functions, classes, types, interfaces; call edges from import analysis)
  - Index stage: `Indexer` — upsert symbols, call edges, imports; update scan_state; generate text embeddings via `ModelBackend.embed()` and write to `embeddings` table (pgvector)

- **packages/cli:**
  - `sensei init` command: detect stack (read package.json/pyproject.toml), prompt for Supabase credentials, run first index, write `.sensei/config.yaml`, generate `CLAUDE.md` and `AGENTS.md` scaffolds, install collector hooks

- **packages/server:**
  - MCP server entry point with stdio transport
  - `get_session_context` tool: returns repo name, symbol count, recent sessions, and current `.sensei/config.yaml` summary
  - `search` tool: BM25 text search against symbol names and file paths in Supabase
  - `load_context` tool: returns file content and extracted symbols for a given path

- **packages/collector:**
  - HTTP daemon on `localhost:51789` (Express or Bun.serve)
  - `POST /event` endpoint — writes to `events` table in Supabase
  - Hook scripts: `pre-tool-use.sh`, `post-tool-use.sh` installed to `~/.claude/hooks/`
  - JSONL fallback writer: on connection failure, append to `.sensei/events.jsonl`; drain on reconnect

- **apps/dashboard:**
  - Repo list view: show indexed repos with symbol count, file count, last indexed timestamp
  - Symbol browser: searchable table of symbols for a repo (name, kind, file, line)
  - Basic Traceability view: placeholder with "0 links" status, ready for Phase 6

### Test Strategy

- **Unit:**
  - `shared`: config parsing, Supabase client construction, type guards
  - `engine/scan`: given a temp directory with known files + a prior scan_state, Scanner returns correct `changed` and `deleted` arrays
  - `engine/parse`: TypeScriptAdapter.extractSymbols on fixture `.ts` files returns expected symbol list; extractEdges returns correct call edge pairs
  - `engine/index`: Indexer.indexRepo writes expected rows to Supabase test instance; re-run on unchanged files is idempotent
  - `collector`: daemon starts, POST /event writes to Supabase, JSONL fallback on refused connection, drain empties JSONL on reconnect
  - `server`: `search` tool returns matching symbol given a query that matches a fixture symbol name

- **Integration:**
  - `engine` full pipeline: Scan → Parse → Index on the `packages/shared/src` directory; assert expected symbol count in Supabase
  - `server` + `engine`: MCP client calls `search("createClient")`, gets expected symbol back

- **E2E:**
  - Playwright: run `sensei init` in a temp clone of a small TS repo → assert `.sensei/config.yaml` created, Supabase `symbols` table non-empty, dashboard at `localhost:3000` shows the repo name and symbol count > 0

### Done When
`sensei init` runs on the sensei repo itself → dashboard at `localhost:3000` shows the repo with file count, symbol count, and "Traceability: 0 links" status → `search` MCP tool returns results for the query "createClient".

---

## Phase 2: Smart Context Delivery

### Goal
The `context_pack` MCP tool returns a ranked, token-budgeted set of code slices relevant to a given task description. The result is visible in the dashboard Context Pack inspector showing which symbols are included and the token breakdown.

### Packages Involved
`shared`, `engine`, `server`, `apps/dashboard`

### What Gets Built

- **packages/shared:**
  - `TokenCounter` interface
  - Implementations: `AnthropicTokenCounter` (via `@anthropic-ai/tokenizer`), `OpenAITokenCounter` (via `tiktoken`), `EstimateTokenCounter` (character/4 heuristic)
  - `TokenBudget` type: `{ max: number, used: number, remaining: number }`
  - Supabase migration: `context_packs` table (session_id, task, slices JSONB, total_tokens, created_at)

- **packages/engine:**
  - Rank stage: `RankingStrategyChain` — executes strategies in configured order, merges scores
    - `DiffFirstBFS`: load changed files from last scan_state, BFS through call_edges in Supabase, score by BFS depth
    - `BM25Strategy`: lexical match between task string and symbol names + file paths
    - `SemanticStrategy`: embed task string via ModelBackend, cosine similarity via pgvector `<=>` operator
  - Slice stage:
    - `ASTSlicer`: given a Symbol (file path + line range), read file, extract the symbol's text plus its immediate callers/callees up to `depth` hops; returns `CodeSlice { path, startLine, endLine, content, tokens }`
    - `SectionSlicer`: given a `DocSection`, return `DocSlice { source, heading, content, tokens }`
  - Assemble stage: `Assembler` — iterate ranked slices, accumulate token count, stop at budget, deduplicate symbols already in session context, return `ContextPack`

- **packages/server:**
  - `context_pack(task, maxTokens?, sessionId?)` tool: calls Ranker → Slicer → Assembler, persists result to `context_packs`, returns ContextPack
  - `recommend_next(task)` tool: returns top-3 ranked symbols and recommended token budget for the task
  - `token_stats(sessionId)` tool: returns token usage breakdown for a session

- **apps/dashboard:**
  - Context Pack inspector view: given a session, show the list of `context_packs`, for each pack show slices with file path, line range, token count, and ranking score; total token gauge

### Test Strategy

- **Unit:**
  - `shared`: TokenCounter implementations — count known strings and assert exact or near-exact token counts against reference values
  - `engine/rank`: DiffFirstBFS given a fixture call graph and changed_files set returns correct ordering; BM25Strategy scores exact-match symbol higher than unrelated symbol; SemanticStrategy returns positive score for semantically related task
  - `engine/slice`: ASTSlicer on a fixture TS file extracts correct line range for a named function
  - `engine/assemble`: Assembler stops at budget, deduplicates correctly, returns slices in rank order

- **Integration:**
  - Full rank → slice → assemble pipeline on sensei's own indexed symbols: given task "fix Supabase connection", assert that symbols related to Supabase client appear in the pack

- **E2E:**
  - Playwright: MCP client calls `context_pack({ task: "fix auth middleware", maxTokens: 8000 })` → assert response has slices, totalTokens <= 8000, at least one slice path contains "auth" or "middleware" → open dashboard Context Pack inspector, assert pack appears with token breakdown

### Done When
`context_pack({ task: "fix auth middleware" })` returns a ContextPack with at least 3 slices, total tokens ≤ 8000, visible in the dashboard Context Pack inspector with per-slice token counts.

---

## Phase 3: Session Continuity

### Goal
An agent starts a task, the terminal is closed (session interrupted), the agent is reopened, and `get_session_context` returns a recovery context showing the last snapshot, decisions made, and where the task was left off.

### Packages Involved
`collector`, `server`, `apps/dashboard`

### What Gets Built

- **packages/collector:**
  - Session tracking: on first hook fire for a new session, create a `task_sessions` row (session_id, repo, agent, start_time, status: "active")
  - Turn tracking: on each PostToolUse event, append a `task_turns` row (session_id, turn_index, tool_name, input_summary, output_summary, timestamp)
  - Snapshot writer: `POST /snapshot` endpoint accepts snapshot payload (decisions[], patterns[], open_questions[], current_focus), writes to `session_snapshots` table; triggered by agent calling MCP `snapshot` tool
  - Crash detection: heartbeat endpoint `POST /heartbeat` called every 30s by daemon; on `process.on('exit')`, mark session as "interrupted" in Supabase; on daemon restart, scan for sessions with last heartbeat > 60s ago and mark them "interrupted"
  - Automatic snapshot: if N turns pass without an explicit snapshot, write an automatic snapshot from the last N turns

- **packages/server:**
  - `snapshot(decisions?, patterns?, focus?)` tool: POST to collector `/snapshot` with current context
  - `add_decision(text, context?)` tool: appends a decision to the active session's decisions array in Supabase
  - `add_pattern(name, description, example?)` tool: appends a pattern to the active session
  - `get_session_context` (update): if a prior session for this repo is in "interrupted" status, include recovery context (last snapshot, open questions, current focus) in the response

- **apps/dashboard:**
  - Session timeline view: list sessions for a repo with status (active, completed, interrupted), duration, turn count, and link to snapshot
  - Recovery indicator: if the most recent session is "interrupted", show a banner on the Repo view: "Previous session interrupted — recovery context available"
  - Session detail: show turns timeline, decisions log, patterns captured, snapshots list

- **packages/shared:**
  - Types: `TaskSession`, `TaskTurn`, `SessionSnapshot`, `Decision`, `Pattern`
  - Supabase migration: `task_sessions`, `task_turns`, `session_snapshots` tables

### Test Strategy

- **Unit:**
  - `collector`: session creation on first event, turn appending idempotent on duplicate event_id, heartbeat updates last_seen, crash detection marks session interrupted when heartbeat lapses
  - `server`: `snapshot` tool formats payload correctly and calls collector endpoint; `get_session_context` includes recovery block when interrupted session exists

- **Integration:**
  - Collector daemon + Supabase: simulate 5 tool events → assert 1 session row and 5 turn rows; simulate process exit → assert session status = "interrupted"; restart daemon → assert previous interrupted session detected

- **E2E:**
  - Playwright: start `sensei init` → run MCP `snapshot` → kill collector process → start fresh MCP session → call `get_session_context` → assert response contains "recovery" key with last snapshot content → open dashboard, assert recovery banner visible on Repo view

### Done When
Start a task → call `snapshot()` → kill terminal → reopen → `get_session_context()` returns recovery context containing the snapshot content (decisions, focus) → dashboard shows the session as "interrupted" with a recovery banner.

---

## Phase 4: Multi-Agent Support

### Goal
`sensei setup` generates agent-specific skill files and hook configurations for Claude Code and opencode. The generated skills contain project-specific content derived from the indexed codebase (not generic templates).

### Packages Involved
`engine`, `cli`, `server`, `apps/dashboard`

### What Gets Built

- **packages/engine:**
  - `ProjectProfile` type: extracted from Supabase index — dominant language, framework, key symbols, import patterns, test patterns, doc coverage
  - Skill generation pipeline: given `ProjectProfile` + `.sensei/config.yaml`, prompt a local model via `ModelBackend.generate()` to produce skill markdown; template variables filled with project-specific content (actual package names, actual CLI commands, actual patterns)
  - `AgentAdapter` interface + implementations:
    - `ClaudeAdapter`: skills dir `~/.claude/skills/`, hooks dir `~/.claude/hooks/`, CLAUDE.md format
    - `OpenCodeAdapter`: skills dir `~/.opencode/skills/`, hooks dir `~/.opencode/hooks/`, AGENTS.md format
    - `GenericAdapter`: plain AGENTS.md only, no skill files
  - Skill categories generated: `orientation` (repo overview, key paths), `workflow` (task protocol, snapshot discipline), `context` (when to call context_pack, token guidance), `patterns` (project-specific conventions from patterns.md)

- **packages/cli:**
  - `sensei setup --agent <claude|opencode|generic|all>` command: run skill generation, write skill files to agent skills dir, install hooks, write AGENTS.md
  - `sensei setup` without flag: auto-detect installed agents (check for `.claude/`, `.opencode/` dirs), prompt user
  - Progress output: spinner per skill file generated, summary of files written

- **packages/server:**
  - `install_skills(agent?)` tool: triggers skill generation and installation for specified or all detected agents; returns list of files written

- **apps/dashboard:**
  - Agent configuration view: show detected agents, skill files status (present/stale/missing), last generated timestamp, "Regenerate" button per agent

- **packages/shared:**
  - `AgentConfig` type in config schema: `agents: { claude?: { enabled: boolean }, opencode?: { enabled: boolean } }`

### Test Strategy

- **Unit:**
  - `engine/skill-generation`: given a fixture `ProjectProfile`, skill generator returns markdown containing the project's actual package names; template renders without errors for all three adapter types
  - `engine/agent-adapters`: ClaudeAdapter.skillsDir returns correct path; installHooks writes correct hook file content; GenericAdapter generates valid AGENTS.md

- **Integration:**
  - Skill generation end-to-end: index sensei's own packages → generate Claude skills → assert generated skill mentions "supabase" and "engine" (actual project content, not generic)

- **E2E:**
  - Playwright: run `sensei setup --agent claude` in a temp repo → assert `~/.claude/skills/sensei-orientation.md` exists and contains the repo name → open dashboard Agent configuration view → assert skill files shown as "present"

### Done When
`sensei setup --agent claude` runs on the sensei repo → `~/.claude/skills/` contains at least 4 skill files → each skill file contains sensei-specific content (e.g., references to `packages/engine`, `bun run test`) → dashboard shows all skill files as "present".

---

## Phase 5: Library Intelligence

### Goal
External libraries used by the project (e.g., Rokkit, SvelteKit) are indexed so `get_lib_docs` returns relevant doc chunks for a specific component. Indexed library docs and generated lib skills are visible in the dashboard Libraries view.

### Packages Involved
`engine`, `cli`, `server`, `apps/dashboard`

### What Gets Built

- **packages/engine:**
  - External doc adapters:
    - `HttpFetcher`: fetches a URL, strips HTML to markdown (using `@mozilla/readability` or `turndown`), caches in Supabase
    - `LlmsTxtParser`: fetches `<base-url>/llms.txt`, parses the structured format, discovers doc pages
    - `LocalSourceAdapter`: runs the full Scan → Parse → Index pipeline on a local directory path (for monorepo sibling packages)
  - `LibIndexer`: orchestrates fetching + parsing + indexing for a library entry; writes to `lib_doc_sections` table with `lib_name`, `component`, `heading`, `content`, `embedding`
  - `LibSkillGenerator`: given indexed lib doc sections + lib usage patterns from the repo's symbol index, generate a skill file teaching the agent how to use the library in this project's context
  - `update-registry` pipeline: reads `custom_libs` from `.sensei/config.yaml`, runs LibIndexer for each, marks stale entries (last_fetched > N days)

- **packages/cli:**
  - `sensei update-registry` command: iterate `custom_libs` config, run LibIndexer per lib, show progress, report freshness status

- **packages/server:**
  - `get_lib_docs(lib, component?, query?)` tool: semantic + BM25 search against `lib_doc_sections` filtered by `lib_name` and optionally `component`; return ranked doc chunk list

- **apps/dashboard:**
  - Libraries view: table of indexed libs with name, doc page count, last fetched timestamp, freshness status (fresh/stale/missing), generated skill file status
  - Library detail: list doc sections with headings; link to skill file

- **packages/shared:**
  - `LibEntry` type: `{ name, source_type: "llms.txt" | "http" | "local", base_url?, local_path?, last_fetched, section_count }`
  - `custom_libs` config schema
  - Supabase migration: `lib_doc_sections` table

### Test Strategy

- **Unit:**
  - `engine/http-fetcher`: given a mock HTTP server returning HTML, fetcher returns stripped markdown
  - `engine/llms-txt-parser`: given a fixture llms.txt file, parser returns correct page list
  - `engine/lib-indexer`: given mock fetcher output, LibIndexer writes expected rows to Supabase test instance
  - `server`: `get_lib_docs("rokkit", "Button")` against test Supabase returns rows with correct lib_name filter

- **Integration:**
  - Index Rokkit's llms.txt (or a fixture thereof) → assert doc section count > 0 in Supabase → `get_lib_docs("rokkit")` returns relevant chunks

- **E2E:**
  - Playwright: add rokkit to `custom_libs` in `.sensei/config.yaml` → run `sensei update-registry` → assert `lib_doc_sections` rows present → call `get_lib_docs({ lib: "rokkit", component: "Button" })` → assert response contains "Button" in at least one chunk → open dashboard Libraries view → assert rokkit shown with section count > 0

### Done When
`get_lib_docs({ lib: "rokkit", component: "Button" })` returns at least 2 relevant doc chunks → Rokkit skill file is visible in the dashboard Libraries view with freshness status "fresh".

---

## Phase 6: Documentation & Traceability

### Goal
`sensei doctor` runs on the sensei repo's own docs and reports coverage. The traceability dashboard shows all feature modules with status. Drift is detected when a symbol changes but its linked doc section is unchanged.

### Packages Involved
`engine`, `cli`, `server`, `apps/dashboard`

### What Gets Built

- **packages/engine:**
  - Doc section indexing: `LocalMarkdownAdapter` — parse `docs/**/*.md`, extract sections by heading, compute content hash, write to `doc_sections` table with `file_path`, `heading_path`, `content`, `hash`, `embedding`
  - `ConfluenceAdapter` (optional, configured via `doc_systems` in config): fetch Confluence pages via API, index to `doc_sections`
  - Drift detection: `DriftDetector` — for each `TraceabilityLink` (symbol_id → doc_section_id), compare symbol's `updated_at` with doc section's `updated_at`; emit drift events where symbol is newer than linked doc
  - Traceability link engine: `TraceabilityLinker` — given a symbol and a doc section (manual or inferred by embedding proximity), write a `TraceabilityLink` row; confidence score for inferred links
  - Doc generation: `DocGenerator` — given a feature's linked symbols, generate a feature doc section from the call graph using `ModelBackend.generate()`; write to `docs/` and index the result
  - Quality analysis: `QualityAnalyser` — cyclomatic complexity per symbol (from AST), test coverage (match `*.spec.ts` imports to symbols), dead code detection (symbols with no callers and not exported), pattern consistency (compare against patterns in `patterns.md`)

- **packages/cli:**
  - `sensei doctor` command: run DriftDetector + QualityAnalyser, output report to stdout (colour-coded: green/amber/red), exit code 1 if any red issues
  - `sensei doc new <title>` command: scaffold a new design doc from template with frontmatter
  - `sensei doc generate --feature <id>` command: run DocGenerator for a feature's symbols
  - `sensei traceability link <symbol> <doc>` command: manually create a TraceabilityLink
  - `sensei traceability export` command: export traceability matrix as CSV or Markdown table
  - `sensei quality report` command: output quality metrics (coverage %, dead code count, complexity histogram)

- **packages/server:**
  - `check_drift(path?)` tool: run DriftDetector for a file or entire repo, return list of drifted links
  - `find_doc(query)` tool: semantic search against `doc_sections`, return ranked doc section list with file path and heading

- **apps/dashboard:**
  - Full Traceability view: table of features × design docs × code symbols; cell shows link status (linked, inferred, missing); click cell to see linked symbols/docs
  - Drift alerts: banner on repo view if any drift detected; Drift detail view showing symbol vs doc section with diff of last-change timestamps
  - Quality metrics panel: test coverage %, dead code count, complexity distribution chart
  - Doc generation UI: "Generate doc" button for a feature that triggers `sensei doc generate` via a Supabase Edge Function or direct engine call

- **packages/shared:**
  - Types: `DocSection`, `TraceabilityLink`, `DriftEvent`, `QualityMetrics`
  - Supabase migration: `doc_sections`, `traceability_links`, `drift_events` tables

### Test Strategy

- **Unit:**
  - `engine/doc-indexer`: LocalMarkdownAdapter on fixture markdown files returns correct section list with heading paths
  - `engine/drift`: DriftDetector on fixture where symbol.updated_at > doc_section.updated_at returns drift event; no event when doc is newer
  - `engine/quality`: QualityAnalyser.testCoverage returns correct ratio given fixture symbols and spec files; dead code detection identifies symbol with no callers
  - `server`: `check_drift` tool returns empty list on freshly indexed and linked repo; returns 1 event after simulated symbol update

- **Integration:**
  - Index sensei's own `docs/design/` → assert doc_sections count matches file/heading count → link a symbol to a doc section → update symbol → run DriftDetector → assert drift event created

- **E2E:**
  - Playwright: run `sensei doctor docs/` on sensei repo → assert exit code and stdout contain coverage percentage → open dashboard Traceability view → assert at least 5 feature rows visible with status indicators → assert Quality metrics panel shows test coverage %

### Done When
`sensei doctor docs/` runs on sensei's own docs and exits with a coverage report → traceability dashboard shows all 10 feature modules with link status → quality report shows test coverage % and dead code count → `check_drift` returns at least one drift item after a symbol is modified.

---

## Phase 7: Analytics & Quality

### Goal
After running 5 agent sessions, the dashboard shows FTR scores per session, model comparison data, and at least one personalised coaching recommendation.

### Packages Involved
`collector`, `engine`, `cli`, `apps/dashboard`

### What Gets Built

- **packages/collector:**
  - FTR calculator: after a session completes, for each requirement touched in the session, check whether the implementing code was accepted in the first attempt (no revert, no follow-up fix turn) → write `task_outcomes` row (session_id, requirement_id, ftr: boolean, rework_count)
  - Requirement quality scorer: analyse the task description text for clarity signals (length, specificity, presence of acceptance criteria) → score 0–1, write to `requirement_quality` in `task_sessions`

- **packages/engine:**
  - Coaching engine: given a developer's session history (FTR per requirement type, rework patterns, model used), compare against aggregate benchmarks → generate a list of coaching recommendations (e.g., "Requirements involving auth have 40% lower FTR — consider adding more context upfront")
  - Aggregate benchmark pipeline: compute per-model FTR averages, per-task-type FTR averages, across all non-private sessions (if telemetry consented)
  - Benchmark runner: `BenchmarkRunner` — given a task corpus YAML, run each task through the MCP server, record FTR results, write to `benchmark_runs` table

- **packages/cli:**
  - `sensei stats` command: show session count, average FTR, top 3 requirement types by FTR, last 7-day trend
  - `sensei benchmark run --corpus <file>` command: run BenchmarkRunner against a task corpus
  - `sensei benchmark compare --run-a <id> --run-b <id>` command: compare two benchmark runs side-by-side

- **apps/dashboard:**
  - Analytics view: FTR trend chart (line chart, per-session), model comparison bar chart (FTR by model used), requirement quality distribution
  - Coaching view: list of personalised recommendations with supporting data (which requirement types, FTR delta vs aggregate), "dismiss" and "mark done" actions
  - Requirement Quality indicator: on session detail view, show the requirement quality score with explanation

- **packages/shared:**
  - Types: `TaskOutcome`, `CoachingRecommendation`, `BenchmarkRun`, `BenchmarkResult`
  - Supabase migration: `task_outcomes`, `coaching_recommendations`, `benchmark_runs`, `benchmark_results` tables

### Test Strategy

- **Unit:**
  - `collector/ftr`: given a session with 3 turns, one of which is a revert, FTR calculator marks that requirement as ftr=false
  - `collector/requirement-quality`: short, vague task description scores < 0.4; detailed description with acceptance criteria scores > 0.7
  - `engine/coaching`: given fixture session history where auth tasks have 2x rework rate, coaching engine generates a recommendation mentioning "auth"

- **Integration:**
  - Run 3 fixture sessions through collector → assert task_outcomes rows created → run coaching engine → assert at least 1 coaching_recommendation row created

- **E2E:**
  - Playwright: run 5 sessions (using fixture events injected via POST /event) → open dashboard Analytics view → assert FTR trend chart shows 5 data points → open Coaching view → assert at least 1 recommendation visible

### Done When
5 sessions recorded → dashboard Analytics view shows FTR trend with 5 data points → Coaching view shows at least 1 personalised recommendation with supporting data (requirement type + FTR delta).

---

## Phase 8: System Intelligence

### Goal
Create a workspace containing sensei + kavach + one other repo. The dashboard renders a service map as a Mermaid diagram. Cross-repo drift is detected when a shared TypeScript type changes in one repo.

### Packages Involved
`engine`, `cli`, `server`, `apps/dashboard`

### What Gets Built

- **packages/engine:**
  - Workspace indexer: `WorkspaceIndexer` — given a workspace config (list of repo paths), runs per-repo `Indexer` in parallel, waits for all to complete, then runs the cross-repo pipeline
  - Cross-repo graph builder: `CrossRepoGraphBuilder` — queries all `symbols` and `imports` across workspace repos, identifies cross-repo import edges (where an import's resolved path matches a symbol in another repo), writes `cross_repo_edges` to Supabase
  - Contract adapters: `OpenAPIAdapter` (parse `openapi.yaml`/`swagger.json` → extract endpoints as symbols), `ProtobufAdapter` (parse `.proto` files → extract messages and RPCs as symbols)
  - Service map generator: `ServiceMapGenerator` — given `cross_repo_edges` + contract symbols, generate a Mermaid `graph LR` diagram showing services (repos) and their dependencies (import edges + API calls)
  - Conformance checker: `ConformanceChecker` — given workspace-level rules (e.g., "no circular dependencies between services", "all API endpoints must have OpenAPI spec"), check all repos, write `conformance_results`
  - System gap analyser: `SystemGapAnalyser` — identify symbols imported across repo boundaries that have no corresponding doc section or traceability link; report as system gaps

- **packages/cli:**
  - `sensei workspace init` command: create `.sensei/workspace.yaml` in a parent directory, prompt for repo paths to include
  - `sensei workspace add <path>` command: add a repo to the workspace config
  - `sensei workspace index` command: run WorkspaceIndexer for all repos in workspace
  - `sensei workspace analyse` command: run ConformanceChecker + SystemGapAnalyser, output report

- **packages/server:**
  - `workspace_status` tool: return workspace name, repo list with per-repo index freshness
  - `workspace_graph` tool: return the Mermaid service map diagram string
  - `find_service(query)` tool: find a service (repo) by name or capability description

- **apps/dashboard:**
  - Service Map view: render the Mermaid diagram from `workspace_graph`; click a service node to navigate to that repo's detail view
  - Workspace Health view: table of repos × conformance rules with pass/fail status; link to failing symbol
  - Cross-repo Drift view: list of cross-repo edges where the source symbol has changed but the consuming repo has not re-indexed; "stale dependency" indicator

- **packages/shared:**
  - Types: `Workspace`, `ServiceNode`, `CrossRepoEdge`, `ConformanceResult`, `SystemGap`
  - Supabase migration: `workspaces`, `workspace_repos`, `cross_repo_edges`, `conformance_results`, `system_gaps` tables

### Test Strategy

- **Unit:**
  - `engine/workspace-indexer`: given 2 fixture repos already indexed, WorkspaceIndexer completes without re-running Scan/Parse (idempotent); parallel execution verified via timing
  - `engine/cross-repo-graph`: given fixture symbols from 2 repos where repo-B imports a symbol from repo-A, CrossRepoGraphBuilder writes 1 cross_repo_edge
  - `engine/service-map`: given 3 repos with 2 cross-repo edges, ServiceMapGenerator returns a Mermaid diagram with 3 nodes and 2 arrows
  - `engine/conformance`: circular dependency rule catches fixture where repo-A imports repo-B which imports repo-A

- **Integration:**
  - Index 2 fixture repos → run CrossRepoGraphBuilder → assert cross_repo_edges count matches expected import count → run ServiceMapGenerator → assert Mermaid output is valid and contains both repo names

- **E2E:**
  - Playwright: `sensei workspace init` with sensei + kavach → `sensei workspace index` → `sensei workspace analyse` → open dashboard Service Map view → assert Mermaid diagram renders with at least 2 nodes → modify a shared type in sensei → assert Cross-repo Drift view shows kavach as having a stale dependency

### Done When
Workspace created with sensei + kavach + one other repo → service map renders in dashboard with all three nodes → `sensei workspace analyse` reports conformance results → modifying a shared type in sensei causes a "stale dependency" alert for the consuming repo in the Cross-repo Drift view.

---

## Phase 9: Identity, Access & Pricing

### Goal
Two developers log in to the dashboard with GitHub OAuth. Each sees only their own repos. A private repo is not visible to a non-member. Pricing tiers restrict which features are accessible.

### Packages Involved
`shared`, `server`, `apps/dashboard`

### What Gets Built

- **packages/shared:**
  - Auth types: `User`, `Team`, `TeamMember`, `Subscription`, `PricingTier`
  - RLS policy helpers: TypeScript wrappers that add `user_id = auth.uid()` or `team_id IN (SELECT team_id FROM team_members WHERE user_id = auth.uid())` filters to Supabase queries
  - Supabase migration: add `user_id` and `team_id` columns to `repos`, `workspaces`, `task_sessions`; create `teams`, `team_members`, `subscriptions` tables; write RLS policies for all tables

- **packages/server:**
  - Auth middleware: validate `Authorization: Bearer <supabase-jwt>` header on every MCP tool call; reject unauthenticated calls with MCP error response
  - Workspace-scoped access enforcement: tool handlers check that the requesting user is a member of the workspace's team before returning data
  - Tier enforcement: feature flags per pricing tier (e.g., workspace tools require "pro" tier); return structured error for tier-restricted tools

- **apps/dashboard:**
  - Auth UI: GitHub OAuth login button (Supabase Auth); magic link email login; session persistence via Supabase session cookie
  - Team management view: create team, invite members by email, set role (admin/member), remove member
  - Settings → Subscription view: current plan, usage metrics (sessions this month, repos indexed), upgrade CTA
  - Telemetry consent UI: first-login modal asking whether to share anonymous aggregate benchmark data; stored in user profile

- **packages/shared:**
  - `TelemetryConsent` type; config schema update to include `telemetry.enabled: boolean`

### Test Strategy

- **Unit:**
  - `shared/rls`: RLS helper wraps a Supabase query with correct user_id filter; workspace query filters by team membership
  - `server/auth-middleware`: valid JWT passes; expired JWT returns MCP error; missing header returns MCP error
  - `server/tier-enforcement`: `workspace_graph` tool returns tier error for user with "free" subscription

- **Integration:**
  - Create 2 test users in Supabase test instance; create a repo owned by user-A; assert user-B's Supabase client cannot read it due to RLS policy; assert user-A's client can read it

- **E2E:**
  - Playwright: open dashboard → assert redirect to login → log in as user-A via magic link → assert user-A's repos visible → log out → log in as user-B → assert user-A's repos not visible → navigate directly to user-A's repo URL → assert 404 or empty state

### Done When
Two users log in with GitHub OAuth → each user sees only their own repos → navigating directly to the other user's repo URL shows empty state or 403 → a user on the free tier calling `workspace_graph` receives a clear "upgrade required" error message.
