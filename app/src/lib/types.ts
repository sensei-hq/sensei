// ─── Stage pattern ─────────────────────────────────────────────────────────────

import type { EventManager } from './events.js';
import type { Component } from 'svelte';

export interface Stage {
  id: string;
  title: string;
  icon: string;
  description: string;
  watermark?: boolean;
  component: Component;
  canAdvance: () => boolean;
  load?: () => Promise<void>;
  source?: EventManager<any>;
}

export interface StateEvent<T extends { id: string }> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;
  data: T | T[];
}

// ─── Scan ──────────────────────────────────────────────────────────────────────

export type FolderStatus = 'discovered' | 'queued' | 'indexing' | 'indexed' | 'failed';
export type ProjectStatus = 'scanning' | 'indexing' | 'active' | 'failed';
export type ActivityLevel = 'discover' | 'queue' | 'process' | 'info' | 'success' | 'error';

export interface ScanProjectFolder {
  id: string;
  name: string;
  path: string;
  stack: string[];
  filesTotal: number;
  filesCompleted: number;
  status: FolderStatus;
  /**
   * Wall-clock ms timestamp of the last add/update event the client received
   * for this folder. Drives the task-list panel's reverse-chronological
   * ordering — in-flight folders bubble up as their progress events arrive,
   * terminal-status folders drift down once their lastUpdated freezes.
   * Optional because hydration from /api/projects has no event timestamp.
   */
  lastUpdated?: number;
}

/** Folder-level SSE event from daemon — includes projectId for routing. */
export interface ScanFolderEvent extends ScanProjectFolder {
  projectId: string;
}

export interface ScanProject {
  id: string;
  name: string;
  status: ProjectStatus;
  folders: ScanProjectFolder[];
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
}

export interface ActivityEvent {
  id: string;
  level: ActivityLevel;
  message: string;
  elapsed: number;
  timestamp: number;
}

// ─── Assistants (AI coding tools) ──────────────────────────────────────────────

export interface AssistantStatus {
  id: string;
  name: string;
  family: string;
  installed: boolean;
  /** True when sensei is integrated with this assistant. The integration
   *  mechanism varies — Claude Code installs a plugin (skills, commands,
   *  agents, hooks, MCP all bundled); others register sensei as an MCP
   *  server in their config file. This flag is the unified signal. */
  configured: boolean;
  config_path: string;
}

/** Capability area an Assistant configures — drives the per-part chips
 *  on each AssistantCard. Stable across detect() so chip order doesn't
 *  flicker when variants come and go. */
export interface AssistantPart {
  id: string;
  label: string;
}

export interface AssistantFamily {
  family: string;
  name: string;
  members: AssistantStatus[];
  installed: boolean;
  config_path: string;
  /** Union of parts across every variant in the family, in canonical order. */
  parts: AssistantPart[];
}

/** Per-part status emitted by the daemon over SSE. Matches the wizard's
 *  AssistantCard chip vocabulary. `idle` is the resting / unconfigured
 *  state — the daemon emits it after a successful remove(), and the app
 *  defaults to it when no event has arrived for a part yet. */
export type AssistantPartStatus = 'idle' | 'configuring' | 'done' | 'error';

/** Payload of StateEvent { entity: "assistant" } — one event per
 *  family×part transition. The aggregated family-level error shown under
 *  the chip strip is derived on the client by collecting messages from
 *  parts whose status is `error`. */
export interface AssistantPartEvent {
  family: string;
  part: string;
  status: AssistantPartStatus;
  error?: string;
}

export interface AssistantConfigureResult {
  configured: string[];
  skipped: string[];
  errors: string[];
}

// ─── Installer ────────────────────────────────────────────────────────────────

export interface InstallResult {
  hooks_installed: number;
  skills_installed: number;
  commands_installed: number;
  stale_commands_removed: number;
  stale_skills_removed: number;
  assistants_configured: string[];
  errors: string[];
  marketplace_version: string;
}

export interface RemoveResult {
  assistants_removed: string[];
  plugin_removed: boolean;
  commands_removed: number;
  skills_removed: number;
  agents_removed: number;
  hooks_removed: boolean;
  cache_cleared: boolean;
  projects_cleaned: string[];
  errors: string[];
}

export interface AssistantRemoveResult {
  assistants_removed: string[];
  errors: string[];
}

export interface InstalledItem {
  name: string;
  kind: string;
  path: string;
  /** Live in `~/.claude/<kind>s/` (true) or moved to sibling `disabled/`
   *  (false). Toggled via `PUT /api/install/installed/{name}/enabled`. */
  enabled: boolean;
}

export interface MarketplaceCatalogItem {
  name: string;
  kind: string;
  description: string;
  scope: string;
  path: string;
  recommended_for: string[];
  stage: string[];
}

export interface MarketplaceCatalog {
  version: string | null;
  items: MarketplaceCatalogItem[];
}

// ─── Scanned Repo (from Tauri analyze_folder) ─────────────────────────────────

export type RepoStatus = 'active' | 'recent' | 'stale' | 'archived' | 'abandoned' | 'unknown';

export interface ScannedRepo {
  name: string;
  path: string;
  repoId?: string;
  remote?: string | null;
  description?: string | null;
  categories: string[];
  status: RepoStatus;
  last_commit_days?: number | null;
  tech_stack: string[];
  commit_count: number;
  duplicate_of?: string | null;
  variant_group?: string | null;
  client?: string;
}

// ─── Indexing ─────────────────────────────────────────────────────────────────

export interface IndexQueueStatus {
  current: { repo_id: string; repo_path: string; status: string } | null;
  queued: Array<{ repo_id: string; repo_path: string; status: string }>;
  recent: Array<{ repo_id: string; repo_path: string; status: string }>;
}

export interface IndexProgressEvent {
  type: 'queued' | 'started' | 'progress' | 'completed' | 'failed';
  repo_id?: string;
  position?: number;
  files_total?: number;
  current_file?: string;
  files_processed?: number;
  files_indexed?: number;
  functions_indexed?: number;
  types_indexed?: number;
  edges_created?: number;
  duration_ms?: number;
  error?: string;
}

export interface DirtyStatus {
  repoId: string;
  codeFiles: number;
  docFiles: number;
  total: number;
}

// ─── Project Model (group of 1+ repos) ──────────────────────────────────────

export type RepoRole =
  | 'backend' | 'frontend' | 'mobile' | 'middleware'
  | 'infra' | 'docs' | 'library' | 'shared' | 'unknown';

export type ProjectCategory = 'active' | 'side' | 'idea';

export interface ProjectRepo {
  repoId: string;
  path: string;
  role: RepoRole;
  label?: string;
}

export interface Project {
  id: string;
  name: string;
  description?: string;
  client?: string;
  category: ProjectCategory;
  repos: ProjectRepo[];
  createdAt: string;
  updatedAt: string;
}

/** @deprecated Use Project instead */
export type Solution = Project;
/** @deprecated Use ProjectRepo instead */
export type SolutionRepo = ProjectRepo;
/** @deprecated Use ProjectCategory instead */
export type SolutionCategory = ProjectCategory;

// ─── Server API response shapes ──────────────────────────────────────────────

export interface ServerProject {
  repo_id: string;
  name: string;
  path: string;
  stack: string[];
  libs: string[];
  tags: string[];
  status: string;
  indexed_at?: string;
  last_error?: string;
  // Aliases for existing UI code (populated by API client)
  repoId: string;
  indexedAt?: string;
  lastError?: string;
  partiallyIndexed?: boolean;
}

/** Shape returned by GET /api/projects (project groups, not individual repos). */
export interface ProjectListItem {
  id: string;
  name: string;
  description?: string | null;
  client?: string | null;
  goal?: string | null;
  maturity: string;
  stack: { languages?: string[]; frameworks?: string[]; runtimes?: string[]; services?: string[] };
  /** Small icon union. `kind:"kanji"` → `value` is a single glyph;
   *  `kind:"image"` → `value` is a URL/path from the project-icon pipeline.
   *  The wire may send an empty `{}` (nothing inferred yet) or `null`. */
  icon?: { kind?: string; value?: string } | null;
  preferred_acp?: string;
  /** Project "purpose" text — sourced from `sensei.projects.goal` and
   *  exposed by the list endpoint as `vision`. Null when absent. */
  vision?: string | null;
  /** The Dōjō membership this project is bound to (`projects.dojo_id`), or null
   *  when unbound. Drives the About-panel binding chip's confirmed state. */
  dojo_id?: string | null;
  /** COUNT of the project's repo-root folders (not nested subfolders). */
  repos_count?: number;
  /** COUNT of the project's linked libraries. */
  libs_count?: number;
  /** MAX(sessions.started_at) for the project, ISO. Null when never run. */
  last_session_at?: string | null;
  /** COUNT of sessions in the last 7 days. Also available via the /ftr fanout. */
  sessions7d?: number;
  [key: string]: unknown;
}

export interface ProjectSummary {
  repoId: string;
  name: string;
  path: string;
  stack: string[];
  libs: string[];
  tags: string[];
  status: string;
  indexedAt?: string;
  functions: number;
  types: number;
  packages: number;
  modules: number;
  edges: number;
  solutions: Array<{ solutionId: string; solutionName: string; role: string }>;
}

export interface GraphNode {
  id: string;
  name: string;
  kind: string;
  file: string;
  line: number;
  complexity?: number;
  doc_type?: string;
  level?: string;
  parent_id?: string;
  tags?: string;
  repoId?: string;
  role?: string;
}

export interface GraphEdge {
  source: string;
  target: string;
  type: string;
  repoId?: string;
}

/**
 * Raw symbol node as returned by `GET /api/graph/nodes?repoId=…`
 * (the daemon's `get_nodes_scoped` shape). Distinct from `GraphNode`,
 * which is the *mapped* solution-graph shape (`file`/`line`). Here the
 * wire carries `file_path`/`line_start` and the owning `folder_id`.
 */
export interface GraphSymbolNode {
  id: string;
  kind: string;
  name: string;
  file_path: string;
  folder_id?: string;
  parent_id?: string | null;
  line_start?: number | null;
  line_end?: number | null;
  /** True when the node belongs to a test file (daemon `is_test_path`). Lets the
   *  Atlas filter tests out when focusing on production code. */
  is_test?: boolean;
}

/**
 * Raw call edge from `GET /api/graph/nodes` / `GET /api/graph/call-flow`.
 * `target_id` is null for calls that resolve to an out-of-scope / stdlib
 * symbol (only `target_name` is known); those are dropped from the graph.
 */
export interface GraphCallEdge {
  id: string;
  source_id: string;
  target_id: string | null;
  target_name?: string | null;
}

export interface GraphData {
  summary: { totalSymbols: number; totalEdges: number; communities: number };
  communities: Array<{
    id: string; label: string; project: string; color: string;
    symbolCount: number; godNodes: string[];
  }>;
  godNodes: Array<{
    name: string; project: string; degree: number; community: string; file: string;
  }>;
  rationale: Array<{
    file: string; tag: string; project: string; text: string;
  }>;
}

export interface ProjectGraphResponse {
  projectId: string;
  name: string;
  nodes: number;
  edges: number;
  repos: number;
  graph: { nodes: GraphNode[]; edges: GraphEdge[] };
}

export interface ProjectAnalysis {
  project_id: string;
  links: Array<{
    from_repo: string; to_repo: string; link_type: string;
    details: string[]; strength: number;
  }>;
  inferred_roles: InferredRole[];
  shared_libs: Array<{ name: string; repos: string[] }>;
}

/** @deprecated Use ProjectGraphResponse instead */
export type SolutionGraphResponse = ProjectGraphResponse;
/** @deprecated Use ProjectAnalysis instead */
export type SolutionAnalysis = ProjectAnalysis;

export interface InferredRole {
  repo_id: string;
  role: string;
  confidence: number;
  reasons: string[];
}

// ─── Graph Queries ───────────────────────────────────────────────────────────

export interface FunctionDetail {
  id: string;
  name: string;
  file: string;
  line: number;
  signature?: string;
  docstring?: string;
  complexity: number;
  tags?: string;
}

export interface TypeDetail {
  id: string;
  name: string;
  file: string;
  line: number;
  kind: string;
}

/**
 * A detected code community as returned by
 * `GET /api/graph/communities/info?repoId=…`. `label` is a representative
 * `"{kind} ({directory})"` summary; `node_count` is the community size.
 * There is no per-node membership on the wire — communities are graph-
 * clustered, so the atlas uses this only for the collapsed overview.
 */
export interface CommunityInfo {
  id: string;
  label: string;
  node_count: number;
}

export interface DocDrift {
  doc_id: string;
  doc_path: string;
  edge_type: string;
  changed_target: string;
}

// ─── Libraries ───────────────────────────────────────────────────────────────

export interface LibEntry {
  id: string;
  name: string;
  ecosystem?: string;
  version?: string | null;
  description?: string | null;
  pageCount?: number;
  repos: string[];
  repoCount: number;
}

export interface LibDoc {
  id: string;
  title: string;
  url: string;
  summary: string;
  content?: string;
  source_type: string;
  component: string;
  indexed_at: string;
}

export interface DepVersion {
  lib_name: string;
  version: string;
  raw_version: string;
  source: string;
  dev: boolean;
}

// ─── Sessions ────────────────────────────────────────────────────────────────

export interface SessionData {
  stats: Record<string, unknown> | null;
  sessions: Array<{
    // project is LEFT JOINed from the session's project_id, so it can be null
    // for a session whose folder isn't linked to a project yet.
    id: string; task: string; project: string | null; startedAt: string;
    completedAt?: string; outcome?: string; summary?: string;
    cost?: number; tokensIn?: number; tokensOut?: number; ftr?: number | null;
    // First-try-right feed carries a rework count (0 ⇒ first-try, N ⇒ N× rework).
    // Optional on the wire so pre-fix daemons still parse.
    corrections?: number;
  }>;
  toolUsage: unknown[];
  benchmarkPairs: unknown[];
}

// ── Observatory · Sessions digest (Slot 6) ────────────────────────────────
// Wire shape for the range/project-scoped `GET /api/sessions?range=&project=`.
// `outcome` is the real `sensei.session_outcome` enum; `agent` is the captured
// assistant family ("claude" / "zed"). `ftr` is a boolean on this endpoint
// (true = first-try-right) or null when the session isn't scored yet.
export type SessionOutcome =
  | 'completed' | 'corrected' | 'blocked' | 'partial' | 'abandoned';

export interface SessionRow {
  id: string;
  project: string | null;
  task: string;
  summary: string | null;
  // Kept string-tolerant so an unrecognized daemon enum value renders neutral
  // rather than throwing — the quality mapper handles the known five.
  outcome: SessionOutcome | string;
  ftr: boolean | null;
  turns: number;
  corrections: number;
  startedAt: string;
  completedAt: string | null;
  agent: string | null;
  // Populated only for multi-repo projects once the daemon LEFT JOINs
  // sensei.folders for `role`. Absent on the current endpoint, so the
  // session row's folder-role chip stays dormant until then.
  folderRole?: string | null;
}

export interface SessionsDigest {
  sessions: SessionRow[];
}

// ── Observatory · Today (home screen) ──────────────────────────────────────
// Wire shapes for GET /api/observatory/today and /api/observatory/ftr. The
// daemon owns the early/mature decision and the koan/insights/adopted
// assembly — the screen renders these fields, it does not derive them.
export interface ObservatoryTodayHero {
  kanji: string;
  koan: string;
  body: string;
  /** Projected effect, e.g. "Projected FTR +14%". Null when not quantified. */
  impact: string | null;
  /** Next-step CTA label. Null in the early state (no confident action yet). */
  action: string | null;
  /** Provenance, e.g. "from s-2891 · s-2889". May be empty. */
  source: string;
  /** Relative recency, e.g. "noticed 2 days ago". May be empty. */
  noticed: string;
}

export interface ObservatoryInsight {
  kanji: string;
  label: string;
  text: string;
  tag: string;
  tone: 'warn' | 'good' | 'mute';
}

export interface ObservatoryAdopted {
  when: string;
  what: string;
  scope: string;
  source: string;
}

export interface ObservatoryToday {
  greeting: string;
  today: string;
  dataMaturity: 'early' | 'mature';
  hero: ObservatoryTodayHero;
  insights: ObservatoryInsight[];
  adopted: ObservatoryAdopted[];
  // Daemon-selected recent sessions in the standard wire-session shape; the
  // page re-shapes them via toRecentSessions for the RecentSessions template.
  recentSessions: SessionData['sessions'];
}

export interface ObservatoryFtr {
  ftr14d: number;
  ftr14dPrev: number;
  ftrTrend: number[];
  sessions7d: number;
}

export interface IndexError {
  repo_id: string;
  file_path: string;
  error: string;
  adapter?: string;
  timestamp: string;
}

// ── Project detail types ──────────────────────────────────────────────────
export interface ProjectMemory {
  id: string;
  name: string;
  title: string;
  kind: string;
  type: string;
  strength: number;
  scope: string;
  // Anatomy fields — surfaced by the Memory Anatomy detail drawer.
  // `content` is the What/Because body the analyzer stored, `impact`
  // is the Consequence, and the two counts are the evidence tally.
  content?: string;
  impact?: string | null;
  reinforcedCount?: number;
  violatedCount?: number;
  scopeFilter?: string | null;
  // Ready-to-share lane — `generalised` is sensei's assessment that this memory
  // has been rewritten stack-agnostic and is safe to widen up the scope ladder;
  // `generalisedContent` carries that portable rewrite (null until generalised).
  // Both are served by GET /api/projects/{id}/memories.
  generalised?: boolean;
  generalisedContent?: string | null;
  /** MAX(reinforced_at, violated_at) — null for freshly minted memories. */
  lastRelevantAt?: string | null;
}

export interface DriftItem {
  id: string;
  file: string;
  status: string;
  confidence: number;
  detail?: string;
  /** What the doc claims the referenced symbol looks like. */
  expectedSignature?: string | null;
  /** What the code actually has. Null when `status = broken` (the target
   *  was deleted or renamed). */
  actualSignature?: string | null;
}

export interface PatternEntry {
  id: string;
  name: string;
  kind: string;
  members: string[];
  confidence: number;
  is_anti_pattern: boolean;
  lifecycle?: string;
  /** Human-readable description of the pattern and why it matters. */
  description?: string | null;
  /** Code example showing the pattern (or the anti-pattern to avoid). */
  example?: string | null;
  /** Free-form enforcement guidance surfaced when lifecycle = rule. */
  enforcement?: string | null;
  /** Signed FTR delta between the pattern's folder and the project baseline.
   *  Positive → the pattern's locus performs above the project average;
   *  negative → below. Null when either baseline can't be computed. */
  ftrDelta?: number | null;
  /** How many code sites carry this pattern (or the anti-pattern). */
  instanceCount?: number;
}

export interface Recommendation {
  id: string;
  title: string;
  description?: string;
  why?: string;
  impact?: string;
  status: string;
  priority?: number;
  verdict?: string;
  urgency?: string;
  baseline_ftr?: number | null;
  current_ftr?: number | null;
  acted_at?: string | null;
  measured_at?: string | null;
}

// ── Observatory · Insights (Learnings triage) ──────────────────────────────
// Wire shapes for `GET /api/insights` — the server-side triage aggregator.
// Every item carries a server-assigned `column` ("now" | "soon" | "settled");
// the client trusts that label and never re-buckets.

export type InsightColumn = 'now' | 'soon' | 'settled';

/** Project reference for the filter-chip strip. */
export interface InsightProjectRef {
  id: string;
  name: string;
  kanji: string;
  /** MAX(sessions.started_at), ISO — drives the Insights filter's "recent 3"
   *  chips. Null when the project has never run a session. */
  last_session_at?: string | null;
}

/** A pending recommendation, pre-bucketed. `name` is the project name;
 *  `impact` is a plain-language sentence (may be null). */
export interface InsightRecommendation {
  id: string;
  urgency: string;
  title: string;
  why: string | null;
  impact: string | null;
  evidence: string[];
  project_id: string;
  name: string;
  column: InsightColumn;
}

/** A memory row. Violated (`violated_count > 0`) memories land in Now;
 *  in-force memories in Settled sorted by strength. */
export interface InsightMemory {
  id: string;
  status: string;
  title: string;
  content: string;
  violated_count: number;
  strength: number;
  scope: string;
  project_id: string;
  column: InsightColumn;
}

/** A detected pattern. `lifecycle` = "suggested" (Soon) | "rule" (Settled). */
export interface InsightPattern {
  id: string;
  name: string;
  family: string | null;
  lifecycle: string;
  instance_count: number;
  project_id: string;
  column: InsightColumn;
}

/** A recurring correction cluster (top-N by count → Now). */
export interface InsightCorrection {
  id: string;
  text: string;
  suggestion: string | null;
  count: number;
  column: InsightColumn;
}

/** The whole triage board in one payload. */
export interface InsightsBoard {
  counts: { now: number; soon: number; settled: number };
  projects: InsightProjectRef[];
  recommendations: InsightRecommendation[];
  memories: InsightMemory[];
  patterns: InsightPattern[];
  corrections: InsightCorrection[];
}

// ─── Front door · Intake ──────────────────────────────────────────────────

/** One playbook in the front-door catalog (from GET /api/playbook/guide). */
export interface IntakePlaybook {
  name: string;
  title: string;
  when_to_use: string;
  opening_tone: string;
  method_ref: string | null;
}

/** One per-axis prompt in the intake guide (kind === "axis"). */
export interface IntakeAxisGuide {
  kind: string;
  axis: string | null;
  prompt: string;
  help: string | null;
}

/** GET /api/playbook/guide — the front-door frame + axis prompts + catalog. */
export interface IntakeGuide {
  frame: string;
  axes: IntakeAxisGuide[];
  playbooks: IntakePlaybook[];
}

/** Proven FTR history for the recommended combo (drives the auto-select badge). */
export interface PlaybookTrust {
  n: number;
  ftr: number;
}

/** POST /api/playbook/recommend — the classified axes + the chosen playbook. */
export interface PlaybookRecommendation {
  playbook: string;
  rationale: string;
  lifecycle: string;
  intent: string;
  risk: string;
  rule: string;
  defaulted: boolean;
  opening_tone: string;
  when_to_use: string;
  auto_select: boolean;
  trust: PlaybookTrust;
}

export interface ProjectSession {
  id: string;
  task: string;
  outcome: string | null;
  ftr: boolean | null;
  turns: number;
  corrections: number;
  provider: string | null;
  model: string | null;
  startedAt: string;
  completedAt: string | null;
}

export interface CallFlowModule {
  path: string;
  exports: string[];
}

export interface CallFlowCall {
  source: string;
  target: string;
  kind: string;
}

// ── Diagnostic Log Types ──────────────────────────────────────────────────

export interface SystemInfo {
    os:        string;
    arch:      string;
    ram_gb:    number;
    cpu_cores: number;
}

export interface LogEntry {
    id:     string;
    ts:     string;
    level:  'info' | 'warn' | 'error';
    layer:  'ui' | 'api' | 'sidecar' | 'data_load';
    step:   string;
    msg:    string;
    data?:  Record<string, unknown>;
    err?:   string;
    stack?: string;
}

export interface BootstrapTrace {
    id:            string;
    ts:            string;
    action_type:   'check' | 'resolve' | 'instruct';
    step:          string;
    desc:          string;
    cmd:           string;
    exit:          number | null;
    out:           string;
    err:           string;
    ms:            number;
    ok:            boolean;
    fix_attempted: boolean;
    fix_approach:  string | null;
    fix_ok:        boolean | null;
}

export interface LogSession {
    id:          string;
    module:      string;
    started_at:  string;
    app_version: string;
    system_info: SystemInfo;
    // 'blocked' is a legacy bootstrap-engine (≤v0.2.2) outcome still present in
    // old session files — kept so the viewer can label it instead of falling
    // through to the generic "fail" styling.
    outcome:     'success' | 'partial' | 'failed' | 'blocked';
    duration_ms: number;
    traces:      (BootstrapTrace | LogEntry)[];
}

// ─── Knowledge plane — federation sources ────────────────────────────────────

export interface KnowledgeSource {
  id: string;
  kind: string;
  name: string;
  url: string;
  namespace_id: string | null;
  direction: string;
  last_seq: number | null;
  enabled: boolean;
}

export interface NewKnowledgeSourceBody {
  url: string;
  key?: string;
  name?: string;
  namespace_id?: string;
}

export interface SyncStats {
  [k: string]: unknown;
}

// ─── Dōjō connections (memberships) ──────────────────────────────────────────
//
// Wire shape returned by `GET /api/dojo/memberships` — a top-level array whose
// items mirror `ConnectionView` in `crates/senseid/src/dojo/memberships.rs`.
// `credential_ref` is deliberately NOT part of the shape: the Keychain
// reference is never exposed over the API, and the device token is write-only
// (sent on connect, never read back).

/** A local project bound to a membership via `projects.dojo_id`. */
export interface DojoBoundProject {
  id: string;
  name: string;
}

/** One Dōjō connection as surfaced by `GET /api/dojo/memberships`. */
export interface DojoMembership {
  id: string;
  registry_url: string;
  tenant_key: string;
  dojo_url: string;
  /** employer | client | community | personal. */
  kind: string;
  /** contributor | maintainer | lead | admin. */
  role: string;
  /** sso | github_oauth | device_code. */
  authenticated_via: string;
  /** named | anonymous | dereferenced. */
  attribution_default: string;
  /** Git-remote owner slugs this membership covers (lowercased) — the
   *  org-tagging that drives infer-at-detect auto-bind. */
  org_slugs: string[];
  /** healthy | stale | error | authenticating. */
  sync_status: string;
  last_seq: number;
  last_heartbeat_at: string | null;
  enabled: boolean;
  bound_projects: DojoBoundProject[];
}

/** The inferred (confirm-inferred) project→Dōjō binding surfaced in the project
 *  About panel — `GET /api/projects/{id}/dojo-suggestion`. The user confirms it
 *  (`POST …/dojo-binding`); it is never auto-committed. */
export interface DojoBindingSuggestion {
  membership_id: string;
  /** employer | client | community | personal. */
  kind: string;
  /** The project's git-remote owner slug that matched the membership. */
  matched_slug: string;
  tenant_key: string;
  dojo_url: string;
}

/** Body for `POST /api/dojo/memberships` — register a connection. `credential`
 *  is the write-only device token (stored in the OS Keychain, never returned).
 *  `registry_url` defaults server-side when omitted. */
export interface ConnectDojoBody {
  /** Service membership id (`dojo.memberships.id`, a uuid) — becomes the PK. */
  membership_id: string;
  registry_url?: string;
  tenant_key: string;
  kind: string;
  /** Git-remote owner slugs this membership covers (org-tagging). Normalised
   *  (lowercased/deduped) server-side. */
  org_slugs?: string[];
  role?: string;
  authenticated_via?: string;
  attribution_default?: string;
  /** The device token (Bearer). Write-only — never read back. */
  credential: string;
  /** Optional project to bind to this membership on connect. */
  project_id?: string;
}

// ─── MCP tool manifests (playground / instruments) ───────────────────────────
//
// Wire shape returned by `GET /api/mcp/tools`. One entry per tool the daemon
// dispatches on. Fields mirror `crates/senseid/src/api/handlers/mcp_manifests.rs`
// — the daemon is the source of truth; keep in lockstep.

export type McpToolKind = 'query' | 'action';
export type McpInputKind = 'text' | 'enum' | 'number';

export interface McpToolInput {
  key: string;
  kind: McpInputKind;
  required: boolean;
  label: string;
  placeholder?: string;
  default?: string;
  options?: string[];
}

export interface McpToolExample {
  response: string;
}

export interface McpToolManifest {
  mcp: string;
  id: string;
  name: string;
  kind: McpToolKind;
  summary: string;
  inputs: McpToolInput[];
  example: McpToolExample;
}

// ─── Session tool timeline (Instruments Replay tab) ──────────────────────────
//
// Wire shape returned by `GET /api/sessions/{id}/tool-timeline`. One row per
// tool call, paired PreToolUse ↔ PostToolUse. Mirrors the JSON keys the
// daemon emits from `pg_store::get_session_tool_calls`.

export interface SessionToolCall {
  callId: number;
  toolName: string;
  family: string;
  request: unknown;
  response: unknown | null;
  success: boolean | null;
  startedAtMs: number;
  completedAtMs: number | null;
  durationMs: number | null;
  startedAt: string;
  completedAt: string | null;
  /** True when the PostToolUse hasn't landed yet (call in-flight or aborted). */
  inFlight: boolean;
}

export interface SessionToolTimeline {
  sessionId: string;
  calls: SessionToolCall[];
  count: number;
}

// ─── #84 T2 Slice C — Replay tab session timeline (with #90 verdicts) ────────
// Response shape of GET /api/sessions/{id}/replay. Each call row is the
// paired PreToolUse↔PostToolUse pair plus the tool_call_verdicts join
// (verdict / confidence / verdictReason are null when unclassified).

export interface SessionReplayCall extends SessionToolCall {
  /** PostToolUse event id — the join key for the verdict. */
  postEventId: number | null;
  /** #90 classification: 'used' | 'partial' | 'ignored' | null. */
  verdict: 'used' | 'partial' | 'ignored' | null;
  /** Classifier confidence in [0, 1], or null when unclassified. */
  confidence: number | null;
  /** Short human-readable reason string from the classifier. */
  verdictReason: string | null;
}

export interface SessionReplayResponse {
  sessionId: string;
  calls: SessionReplayCall[];
  count: number;
  /** #90 session-level verdict summary. */
  summary: { used: number; partial: number; ignored: number; total: number };
  /** Number of verdict rows written by the classify pass (0 unless
   *  `?classify=true` was set on the request). */
  classified: number;
}

// ─── #84 T2 Slice A — discovered MCP servers ─────────────────────────────────

export interface McpServerRow {
  id: string;
  acp_family: string;
  mcp_key: string;
  scope: 'user' | 'project';
  project_id: string | null;
  config_source: string;
  command: string;
  args: unknown[];
  env: Record<string, unknown>;
  enabled: boolean;
  connection_state: 'unknown' | 'connected' | 'error' | 'disabled';
  last_error: string | null;
  last_seen_at: string;
  discovered_at: string;
}

// ─── #84 T2 Slice B — per-server tool manifest cache ─────────────────────────

export interface McpServerToolsManifest {
  id: string;
  server_id: string;
  tools: unknown[];
  tool_count: number;
  probed_at: string;
  ttl_seconds: number;
  error: string | null;
  protocol_version: string | null;
  server_name: string | null;
  server_version: string | null;
  age_seconds: number;
  /** Only present when the caller asked to skip probing and got stale cache. */
  stale?: boolean;
}

// ─── Memory share batches (T3 Slice 2.2) ─────────────────────────────────────

export type MemoryShareBatchStatus = 'proposed' | 'approved' | 'rejected' | 'withdrawn';

export interface MemoryShareBatch {
  id: string;
  status: MemoryShareBatchStatus;
  note: string | null;
  createdAt: string;
  decidedAt: string | null;
  memberCount: number;
}

// ─── Manual impact-verdict log (T3 Slice 3) ──────────────────────────────────

export type ImpactVerdict = 'pending' | 'success' | 'mixed' | 'failure';

export interface ImpactVerdictEntry {
  id: string;
  sessionId: string | null;
  title: string;
  note: string | null;
  verdict: ImpactVerdict;
  createdAt: string;
  decidedAt: string | null;
}

// ─── Project-scoped MCP tool stats (T2 Slice F) ──────────────────────────────

export interface ProjectMcpToolStat {
  id: string;
  name: string;
  mcp: string;
  kind: McpToolKind;
  summary: string;
  calls: number;
  errors: number;
  avgDurationMs: number | null;
  ftr: number | null;
  lastUsedAt: string | null;
}

// ─── Observatory tool signals (T2 Slice D-lite) ──────────────────────────────

export type SignalVariant = 'warn' | 'opportunity' | 'unused' | 'win';

export interface ToolSignal {
  tool_name: string;
  variant: SignalVariant;
  title: string;
  detail: string;
}

// ─── Service scoping (T2 Slice B) ────────────────────────────────────────────

export interface ProjectService {
  id: string;
  name: string;
  displayName: string;
  publisher: string | null;
  protocol: string;
  kind: string;
  summary: string | null;
  toolsCount: number;
  verified: boolean;
  installed: boolean;
  /** Effective enable state (scoped > global > default true). */
  enabledForProject: boolean;
  /** Raw scoped override for this project; null when no override row exists. */
  scopedEnabled: boolean | null;
  /** Raw global scope; null when no global row exists. */
  globalEnabled: boolean | null;
}

// ─── Instruments · Health L1 source grid ─────────────────────────────────────
// Shape of GET /api/instruments/tools-health — one row per registered source
// (a builtin assistant tool set, or a probed MCP server). Drives the L1
// "which sources earn their keep?" grid on the Instruments · Health surface.

export interface ToolHealthSource {
  assistant_family: string;
  source_type: 'mcp' | 'builtin';
  source_key: string;
  name: string;
  /** Normalised connection flag — the UI's primary discriminator. */
  connected: boolean;
  /** Raw enum from sensei.mcp_servers (unknown|connected|error|disabled), or
   *  null for builtin / never-probed sources. */
  connection_state: string | null;
  server_id: string | null;
  /** Null when the server was never probed — the card must then show
   *  `registered —` and omit the share bar (share_invoked is also null). */
  tools_registered: number | null;
  tools_invoked_14d: number;
  calls_14d: number;
  /** tools_invoked_14d / tools_registered for probed sources; null otherwise. */
  share_invoked: number | null;
}

export interface ToolsHealth {
  sources: ToolHealthSource[];
}

// ─── Cached tool insights (T2 Slice D) ───────────────────────────────────────

export interface ToolInsight {
  toolName: string;
  computedAt: string;
  metrics: {
    callCount?: number;
    errorCount?: number;
    errorRate?: number;
    avgDurationMs?: number | null;
    lastUsedAt?: string;
    // #84 T2 Slice D — 14d verdict split merged in by
    // aggregate_tool_insights. Present on every metrics row after the
    // Slice D commit, defaulting to 0 for tools with no classified
    // verdicts in the window.
    usedCount?: number;
    partialCount?: number;
    ignoredCount?: number;
    verdictTotal?: number;
    usedPct?: number;
    partialPct?: number;
    ignoredPct?: number;
    verdictWindowDays?: number;
  };
  variant: SignalVariant | null;
  title: string | null;
  detail: string | null;
}

// ─── Project window · Overview (Slot 4) ──────────────────────────────────────
// Shape of GET /api/projects/{id}/overview — a server-side assembler that
// composes the project window's landing pane from several sources. The wire is
// camelCase throughout, except the top-level `top_recommendation` key.

export interface ProjectOverviewFolder {
  id: string;
  name: string;
  /** Folder-project membership role (backend/frontend/library/…); null when
   *  the folder has no assigned role. */
  role: string | null;
  primary: boolean;
}

export interface ProjectOverviewHeader {
  id: string;
  name: string;
  /** Single glyph from the project's icon; 場 when nothing is set. */
  kanji: string;
  client: string | null;
  goal: string | null;
  /** 14-day rolling FTR, 0..1 — same project_ftr_metrics view the
   *  projects-index card reads, so the two numbers never disagree. */
  ftr: number;
  warn: boolean;
  sessions7d: number;
  folders: ProjectOverviewFolder[];
}

export interface ProjectOverviewTopRec {
  id: string;
  title: string;
  why: string;
  /** Session ids backing the recommendation; may be empty. */
  evidence: string[];
  /** Preferred ACP to route the rec to; null when none is set. */
  defaultAcp: string | null;
}

export interface ProjectOverviewStats {
  sessions7d: number;
  sessions7dCorrected: number;
  memories: { total: number; readyToShare: number; toMerge: number };
  docDrift: { open: number; referencedDocs: number };
}

export interface ProjectOverviewSession {
  id: string;
  title: string;
  startedAt: string;
  completedAt: string | null;
  corrections: number;
  ftr: boolean;
  /** Folder role for the repo this session touched; null for single-repo
   *  projects or unassigned folders. */
  role: string | null;
}

export interface ProjectOverview {
  project: ProjectOverviewHeader;
  top_recommendation: ProjectOverviewTopRec | null;
  stats: ProjectOverviewStats;
  recentSessions: ProjectOverviewSession[];
}

// ─── Dōjō downstream inbox (Observatory · Upgrades · C7) ──────────────────────
//
// Wire shape returned by `GET /api/upgrades[?include_muted=1]` — the daemon's
// downstream inbox of APPROVED Dōjō / Collective artifacts pulled back for the
// user to Apply / Mute / Pin. Mirrors `InboxItem` in
// `crates/senseid/src/collective/inbox.rs` (serde renames `kind` → `type`) plus
// the `ArtifactScope` / `Attribution` jsonb shapes from the `dojo-protocol`
// crate. Muted items are hidden unless `include_muted`; pinned float to the top;
// `unread_count` is the number of items still `pending`.

/** The six downstream artifact kinds — `dojo.artifact_kind`. */
export type DojoUpgradeType = 'principle' | 'pattern' | 'prompt' | 'guard' | 'skill' | 'agent';

/** Local override state of an inbox item — `dojo_inbox.state`. */
export type DojoUpgradeState = 'pending' | 'applied' | 'muted' | 'pinned';

/** Scope tag deciding where an artifact applies — the `dojo.artifacts.scope`
 *  jsonb. Every field is optional (an unscoped artifact applies everywhere). */
export interface DojoUpgradeScope {
  company?: string;
  team?: string;
  project?: string;
  stack?: string;
}

/** How the artifact is credited — the `dojo.artifacts.attribution` jsonb.
 *  `dereferenced: true` marks client-work whose source reference was stripped. */
export interface DojoUpgradeAttribution {
  /** named | anonymous | dereferenced (string-tolerant on the wire). */
  mode: DojoUpgradeAttributionMode | string;
  author?: string;
  org?: string;
  anonymous_id?: string;
  dereferenced: boolean;
}

export type DojoUpgradeAttributionMode = 'named' | 'anonymous' | 'dereferenced';

/** One downstream artifact as surfaced by `GET /api/upgrades`. `type` and
 *  `state` are kept string-tolerant so a new wire value renders neutral rather
 *  than throwing — wire-API-wins. */
export interface DojoUpgrade {
  id: string;
  membership_id: string;
  type: DojoUpgradeType | string;
  title: string;
  body: string;
  scope: DojoUpgradeScope;
  attribution: DojoUpgradeAttribution;
  state: DojoUpgradeState | string;
  /** Why an Apply didn't land (a deferred kind, or a scope mismatch). */
  note?: string;
  /** Set once an Applied principle/pattern lands as a `sensei.memories` row. */
  applied_memory_id?: string;
  received_at?: string;
}

/** Response shape of `GET /api/upgrades`. */
export interface DojoUpgradesResponse {
  artifacts: DojoUpgrade[];
  unread_count: number;
}

// ─── Share review (Observatory · Share review · C11) ─────────────────────────
//
// Wire shapes of `GET /api/share-review/next-batch` and
// `POST /api/share-review/{batch}/publish`. Derived from the Rust `BatchPreview`
// / `ItemPreview` / `ContributeOutcome` / `ItemOutcome` structs in
// `crates/senseid/src/dojo/contribute.rs`. The upstream publish-gate: the next
// approved-but-unsent batch with its per-item dereference PREVIEW. `type`,
// `state`, attribution.mode and `result` stay string-tolerant so a new wire
// value renders neutral rather than throwing — wire-API-wins. Attribution reuses
// `DojoUpgradeAttribution` (the same `dojo.artifacts.attribution` jsonb shape as
// the downstream Upgrades inbox).

/** One item as previewed on the share-review surface — the redacted text about
 *  to leave, plus whether it is held for residual identifier risk. `held` items
 *  never ship this batch; `queued` items ship on publish. `will_dereference`
 *  marks client-work whose source reference is stripped (mandatory — the user
 *  cannot override it). */
export interface ShareReviewItem {
  memory_id: string;
  type: DojoUpgradeType | string;
  title: string;
  body: string;
  attribution: DojoUpgradeAttribution;
  will_dereference: boolean;
  /** `queued` (ships next batch) or `held` (residual risk — won't ship). */
  state: 'queued' | 'held' | string;
}

/** The next approved-but-unsent batch preview surfaced by
 *  `GET /api/share-review/next-batch`. */
export interface ShareReviewBatch {
  batch_id: string;
  /** The routed destination tenant keys (`[]` when unbound / no memberships). */
  destination: string[];
  /** Cadence controls (Observatory · Collective / C9) are not modelled yet —
   *  currently always `manual`. */
  cadence: string;
  next_batch_at?: string;
  items: ShareReviewItem[];
}

/** Response of `GET /api/share-review/next-batch` — `batch: null` when nothing
 *  is pending. */
export interface ShareReviewResponse {
  batch: ShareReviewBatch | null;
}

/** Per-item outcome kind of a publish — the `result` tag of the Rust
 *  `ItemResult`. `published` landed (with `seq`/`remote_id`);
 *  `held_residual_risk` was refused by the confidentiality gate;
 *  `queued_retry` left queued after a transient failure; `already_sent` was
 *  deduped; `error` carries a `message`; `no_destination` had no routed Dōjō. */
export type PublishResultKind =
  | 'published'
  | 'staged'
  | 'held_residual_risk'
  | 'queued_retry'
  | 'already_sent'
  | 'error'
  | 'no_destination';

/** The outcome of contributing one memory to one destination — the Rust
 *  `ItemOutcome` (`result` flattened onto the row). */
export interface PublishItemOutcome {
  memory_id: string;
  membership_id?: string;
  tenant_key?: string;
  kind: DojoUpgradeType | string;
  result: PublishResultKind | string;
  /** Present when `result === 'published'`. */
  seq?: number;
  remote_id?: string;
  /** Present when `result === 'error'`. */
  message?: string;
}

/** Response of `POST /api/share-review/{batch}/publish` — the Rust
 *  `ContributeOutcome` (per-item outcomes plus roll-up counts). */
export interface PublishBatchOutcome {
  batch_id: string;
  published: number;
  held: number;
  queued: number;
  errored: number;
  already_sent: number;
  items: PublishItemOutcome[];
}

// ─── Collective sharing preferences (Observatory · Collective · C9) ──────────
//
// Wire shape of `GET/PUT /api/preferences/collective`. The daemon always
// returns a FULL object — the stored row, or the defaults when unset — and PUT
// is a whole-object full-replace (any omitted field takes its default), echoing
// back the saved object with a fresh `updated_at`. A bad enum / unknown category
// key / non-boolean category value is a 400 `{ "error": "..." }`. So the client
// holds the full object and does read-modify-write: mutate one field, PUT the
// whole thing, adopt the returned saved object as the new truth.

/** Where shared insight leaves the machine. `both` = global + dojo at once —
 *  a single enum here, not two independent toggles on the wire. */
export type CollectiveDestination = 'none' | 'global' | 'dojo' | 'both';

/** How often batches are prepared. */
export type CollectiveCadence = 'manual' | 'daily' | 'weekly';

/** Default authorship applied to what is shared. `dereferenced` (strip the
 *  source reference) is the safest and the wire default. */
export type CollectiveAttribution = 'named' | 'anonymous' | 'dereferenced';

/** The seven share categories — all keys always present, stable order. */
export type CollectiveCategoryKey =
  | 'memory'
  | 'pattern'
  | 'rule'
  | 'prompt'
  | 'guard'
  | 'skill'
  | 'agent';

/** Per-category share on/off. Every key is always present on the wire. */
export type CollectiveCategories = Record<CollectiveCategoryKey, boolean>;

/** The full collective-sharing preferences object. */
export interface CollectivePreferences {
  destination: CollectiveDestination;
  cadence: CollectiveCadence;
  categories: CollectiveCategories;
  attribution_default: CollectiveAttribution;
  /** RFC-3339 once saved; null while still on defaults. */
  updated_at: string | null;
}

// ── Observatory · Logs (activity logs) ─────────────────────────────────────
// Wire shape of a `GET /api/logs` row (see senseid logs handler `query_logs`).
// The daemon SELECTs `id, level, running_on, logged_at, message, context,
// data, error` and re-labels `running_on` → `source` in the JSON.

/** Ordered severity levels the daemon writes. `level` is kept as `string`
 *  on the row for forward-compatibility, but these are the known values. */
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error';

/** One structured log row from `GET /api/logs`. `context` is always an object
 *  (defaults to `{}`); `data`/`error` are null unless the writer set them.
 *  `source` (the daemon's `running_on`) is null until the writer populates it. */
export interface LogRow {
  id: string;
  /** One of `LogLevel` in practice; typed wide so an unknown level renders. */
  level: string;
  /** `running_on` — daemon / cli / mcp / app. Null when the writer omits it. */
  source: string | null;
  /** RFC-3339 timestamp. */
  logged_at: string;
  message: string;
  /** Free-form jsonb; carries `module`, `folder`, `path`, etc. */
  context: Record<string, unknown> | null;
  data: Record<string, unknown> | null;
  error: Record<string, unknown> | null;
}

/** One background worker from `GET /api/tasks/scheduled` (#96). Health fields
 *  (last_ok/last_error/next_run_at/interval_secs/avg_ms) are null until the
 *  daemon records run outcomes — render them as "—", not "healthy". */
export interface ScheduledTask {
  name: string;
  description: string;
  /** RFC-3339, or null when the worker persists no last-run watermark. */
  last_run_at: string | null;
  last_ok: boolean | null;
  last_error: string | null;
  next_run_at: string | null;
  interval_secs: number | null;
  avg_ms: number | null;
}

// ─── On-demand local-model provisioning ──────────────────────────────────────

/** The phase of an on-demand local-model pull — the serde shape of
 *  `kernel::ProvisionPhase` (internally tagged on `phase`, snake_case) as
 *  returned by `GET /api/gateway/models/provision/status`. Discriminated union
 *  on `phase` so callers narrow to the payload-carrying variants. */
export type ProvisionPhase =
  | { phase: 'absent' }
  | { phase: 'queued' }
  | { phase: 'downloading'; done: number; total: number | null }
  | { phase: 'verifying' }
  | { phase: 'loading' }
  | { phase: 'ready' }
  | { phase: 'failed'; error: string };

/** One provisionable local model — a catalog entry with its current phase. The
 *  status endpoint lists every pullable model (phase `absent` before any pull),
 *  overlaying the supervisor's live phase once a pull is in flight. */
export interface ProvisionModel {
  id: string;
  name: string;
  phase: ProvisionPhase;
}
