import type {
  ServerProject, ProjectSummary, GraphData, GraphNode, GraphEdge,
  GraphSymbolNode, GraphCallEdge,
  SolutionGraphResponse, SolutionAnalysis, InferredRole,
  IndexQueueStatus, DirtyStatus, IndexError,
  FunctionDetail, TypeDetail, CommunityInfo, DocDrift,
  LibEntry, LibDoc, DepVersion, SessionData, SessionsDigest,
  ProjectMemory, DriftItem, PatternEntry, Recommendation,
  ProjectSession, CallFlowModule, CallFlowCall,
  ProjectListItem,
  KnowledgeSource, NewKnowledgeSourceBody, SyncStats,
  DojoMembership, ConnectDojoBody, DojoBindingSuggestion, DojoUpgradesResponse, CollectivePreferences,
  ShareReviewResponse, PublishBatchOutcome,
  McpToolManifest, SessionToolTimeline, MemoryShareBatch, ImpactVerdictEntry,
  ProjectMcpToolStat, ToolSignal, ProjectService, ToolInsight, ToolsHealth,
  SessionReplayResponse, McpServerRow, McpServerToolsManifest,
  ObservatoryToday, ObservatoryFtr, ProjectOverview,
  InsightsBoard, LogRow, ScheduledTask,
  IntakeGuide, PlaybookRecommendation,
  ProvisionModel, ProvisionPhase,
} from './types.js';
import type {
  MemoryListResponse, MemoryDetail, ContextResponse,
  ProposalCreateBody, MemoryCreateBody, OutcomeBody, OutcomesBatchResponse,
} from './setup/contracts.js';
import type {
  ConsolidatedRuleset, ConsolidateResult,
} from '../routes/(observatory)/consolidation/consolidation-view.js';
import type {
  ProjectMetricRow, RegistryMetric, MetricSeriesPoint, MetricsNarrative,
  DaySessions, DrilldownSession, ToolUsage,
} from './metrics/metric-view.js';

export type { DaySessions, DrilldownSession, ToolUsage };

export type ApiError = { status: number; message: string } | { status: 0; message: string };

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** The daemon's origin for a given port. The single source of truth for the
 *  loopback base URL, shared by `senseiApi` and any non-fetch consumer (e.g.
 *  an `<img src>` that streams bytes from the daemon, like a project icon). */
export const apiBase = (port: number): string => `http://127.0.0.1:${port}`;

/** Create a typed API client for the sensei Rust daemon. */
export function senseiApi(port: number) {
  const base = apiBase(port);

  // Legacy fallback-returning helpers — kept for callers that pre-date
  // the Result-based `tryGet`/`tryPost` etc. New code should use the
  // try-prefixed variants so errors flow through ApiResult instead of
  // being absorbed into a sentinel value. The console.warn here keeps
  // failures observable until the migration finishes.
  async function get<T>(path: string, fallback: T): Promise<T> {
    try {
      const res = await fetch(`${base}${path}`);
      if (res.ok) return await res.json() as T;
      console.warn('[api] GET non-ok; using fallback', { path, status: res.status, statusText: res.statusText });
      return fallback;
    } catch (e) {
      console.warn('[api] GET failed; using fallback', { path }, e);
      return fallback;
    }
  }

  async function post<T>(path: string, body: unknown, fallback: T): Promise<T> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) return await res.json() as T;
      console.warn('[api] POST non-ok; using fallback', { path, status: res.status, statusText: res.statusText });
      return fallback;
    } catch (e) {
      console.warn('[api] POST failed; using fallback', { path }, e);
      return fallback;
    }
  }

  async function put(path: string, body: unknown) {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!res.ok) console.warn('[api] PUT non-ok', { path, status: res.status, statusText: res.statusText });
    } catch (e) {
      console.warn('[api] PUT failed', { path }, e);
    }
  }

  async function tryPut(path: string, body: unknown): Promise<ApiResult<void>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) return { ok: true, data: undefined };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  async function del(path: string) {
    try { await fetch(`${base}${path}`, { method: 'DELETE' }); } catch { /* non-fatal */ }
  }

  async function tryDelete(path: string): Promise<ApiResult<void>> {
    try {
      const res = await fetch(`${base}${path}`, { method: 'DELETE' });
      if (res.ok) return { ok: true, data: undefined };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  async function tryGet<T>(path: string): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`);
      if (res.ok) return { ok: true, data: await res.json() as T };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  async function tryPost<T>(path: string, body: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) return { ok: true, data: await res.json() as T };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  // Error-propagating PUT that returns the parsed response body — the
  // whole-object full-replace endpoints (e.g. collective preferences) echo the
  // saved object back and the caller adopts it. On a non-ok it surfaces the
  // daemon's `{ "error": "..." }` message when present, else the status text.
  async function tryPutJson<T>(path: string, body: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) return { ok: true, data: await res.json() as T };
      let message = res.statusText;
      try {
        const j = await res.json() as { error?: string };
        if (j && typeof j.error === 'string' && j.error) message = j.error;
      } catch { /* non-JSON error body — keep the status text */ }
      return { ok: false, error: { status: res.status, message } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  return {
    // ── Health ────────────────────────────────────────────────────────────
    getHealth: () => get<Record<string, unknown>>('/health', {}),

    getComponents: () =>
      get<{ components: Array<{ id: string; name: string; version: string | null; status: string; icon: string }> }>(
        '/api/health/components', { components: [] },
      ),

    // ── Scan suggestions ─────────────────────────────────────────────────
    getScanSuggestions: () =>
      get<Array<{ name: string; strategy: string; repo_ids: string[] }>>(
        '/api/scan/suggestions', [],
      ),

    getScanRoots: () =>
      get<Array<{ path: string; created_at: string | null; repos_found: number; scanned: boolean }>>(
        '/api/scan/roots', [],
      ),

    // ── Repos (individual git repos) ────────────────────────────────────
    getRepos: async () => {
      const repos = await get<ServerProject[]>('/api/repos', []);
      return repos.map(p => ({
        ...p,
        repoId: p.repo_id,
        indexedAt: p.indexed_at,
        lastError: p.last_error,
        partiallyIndexed: false,
      }));
    },

    getRepoSummary: (repoId: string) =>
      get<ProjectSummary | null>(`/api/repos/${enc(repoId)}/summary`, null),

    registerRepo: (repoId: string, name: string, path: string) =>
      post('/api/repos', { repoId, name, path }, { ok: false }),

    updateRepo: (repoId: string, patch: Partial<ServerProject>) =>
      put(`/api/repos/${enc(repoId)}`, patch),

    deleteRepo: (repoId: string) => del(`/api/repos/${enc(repoId)}`),

    excludeRepo: (repoId: string) =>
      post(`/api/repos/${enc(repoId)}/exclude`, {}, { ok: false }),

    addRepoTag: (repoId: string, tag: string) =>
      post(`/api/repos/${enc(repoId)}/tags`, { tag }, { ok: false }),

    removeRepoTag: (repoId: string, tag: string) =>
      del(`/api/repos/${enc(repoId)}/tags/${enc(tag)}`),

    // ── Projects (groups of 1+ repos) ───────────────────────────────────
    listProjects: () => get<ProjectListItem[]>('/api/projects', []),

    createProject: (project: object) =>
      post<{ ok: boolean; id?: string }>('/api/projects', project, { ok: false }),

    updateProject: (id: string, patch: object) =>
      put(`/api/projects/${enc(id)}`, patch),

    /** Error-propagating variant of updateProject — the About-tab metadata
     *  form drives a saveStatus (idle/saving/saved/error) lifecycle off the
     *  Result, mirroring trySetConfig on the settings form. The daemon's
     *  PUT /api/projects/{id} does a partial (COALESCE) update of the editable
     *  identity subset (name/description/client/goal/maturity/…). */
    tryUpdateProject: (id: string, patch: object): Promise<ApiResult<void>> =>
      tryPut(`/api/projects/${enc(id)}`, patch),

    deleteProject: (id: string) => del(`/api/projects/${enc(id)}`),

    /** Update a single folder. Currently only `role` is honored daemon-side. */
    updateFolder: (id: string, patch: { role?: string | null }) =>
      put(`/api/folders/${enc(id)}`, patch),

    addProjectRepo: (projectId: string, repo: { repoId: string; role?: string }) =>
      post(`/api/projects/${enc(projectId)}/repos`, repo, { ok: false }),

    removeProjectRepo: (projectId: string, repoId: string) =>
      del(`/api/projects/${enc(projectId)}/repos/${enc(repoId)}`),

    getProjectGraph: (id: string) =>
      get<SolutionGraphResponse | null>(`/api/projects/${enc(id)}/graph`, null),

    getProjectRoles: (id: string) =>
      get<InferredRole[]>(`/api/projects/${enc(id)}/roles`, []),

    analyzeProject: (id: string) =>
      post<SolutionAnalysis | null>(`/api/projects/${enc(id)}/analyze`, {}, null),

    // ── Project detail (new multi-window endpoints) ───────────────────
    getProjectFtr: (id: string) =>
      get<{ ftr14d: number | null; ftr14dPrev: number | null; ftrTrend: number[]; sessions7d: number }>(
        `/api/projects/${enc(id)}/ftr`,
        // Fallback on a fetch error is honest no-data (null), never a fabricated 0%.
        { ftr14d: null, ftr14dPrev: null, ftrTrend: [], sessions7d: 0 }
      ),

    // ── Project metrics (registry-driven) ─────────────────────────────
    // Error-propagating (tryGet): a fetch failure must be distinguishable from a
    // project with no computed metrics yet, so the screen shows an error state
    // rather than an empty grid that hides a broken daemon (no-fabrication).
    getProjectMetrics: (id: string) =>
      tryGet<{ metrics: ProjectMetricRow[]; count: number; narrative?: MetricsNarrative | null }>(
        `/api/projects/${enc(id)}/metrics`,
      ),

    // The catalog — the only surface that carries each metric's `family`, joined
    // client-side to group the per-project values into sections.
    getMetricsRegistry: () =>
      tryGet<{ metrics: RegistryMetric[]; count: number }>(`/api/metrics/registry`),

    // Per-metric time series for the card sparklines / trend view.
    getProjectMetricSeries: (
      id: string,
      key: string,
      grain: 'daily' | 'weekly' | 'monthly' | 'quarterly' = 'daily',
    ) =>
      tryGet<{
        metric: string;
        grain: string;
        // The metric's `formula` (registry "how it's calculated") — travels with the
        // series so the detail screen renders it. Honest-null for an unknown key.
        formula: string | null;
        series: MetricSeriesPoint[];
        count: number;
      }>(`/api/projects/${enc(id)}/metrics/${enc(key)}?grain=${grain}`),

    // The measurable sessions behind ONE daily datapoint (the datapoint→sessions
    // drill-down). Error-propagating (tryGet): a fetch failure — including a 404
    // on a daemon that predates the endpoint — surfaces as `{ ok: false, error }`
    // so the drill-down renders an explicit "not available"/error STATE, never a
    // fabricated session list. `day` is `YYYY-MM-DD`.
    getProjectMetricDaySessions: (id: string, key: string, day: string) =>
      tryGet<DaySessions>(
        `/api/projects/${enc(id)}/metrics/${enc(key)}/sessions?day=${enc(day)}`,
      ),

    // Per-tool usage breakdown (which tools the ACPs invoked) — the tool-usage
    // bubble view. Honest-empty ({tools:[]}) on a fetch error, never fabricated.
    getProjectTools: (id: string) =>
      get<{ tools: ToolUsage[]; count: number }>(`/api/projects/${enc(id)}/tools`, {
        tools: [],
        count: 0,
      }),

    getProjectRepos: (id: string) =>
      get<{ repos: Array<{ id: string; name: string; path: string; kind: string; role?: string }> }>(
        `/api/projects/${enc(id)}/repos`, { repos: [] }
      ),

    // Error-propagating (mockup-drift-audit F8): a fetch failure must be
    // distinguishable from a project that genuinely has no libraries. tryGet
    // surfaces {ok:false,error} so the screen shows error-with-Retry instead of
    // an empty list that hides a broken daemon.
    tryGetProjectLibraries: (id: string) =>
      tryGet<{ libraries: Array<{
        id: string; name: string; ecosystem: string;
        scope: 'global' | 'project'; enabled: boolean;
        description?: string | null;
        hasDocs?: boolean; pageCount?: number;
        localSource?: string | null;
      }> }>(
        `/api/projects/${enc(id)}/libraries`,
      ),

    // T1a version-conflict view — surfaces libraries pinned to multiple
    // versions across the project's folders. One row per (project, library)
    // pair with the distinct versions + folders where each was seen.
    // Error-propagating (F8) — see tryGetProjectLibraries.
    tryGetProjectLibraryVersionConflicts: (id: string) =>
      tryGet<{ conflicts: Array<{ library_id: string; library_name: string; ecosystem: string; versions: string[]; folders: string[] }> }>(
        `/api/projects/${enc(id)}/library-version-conflicts`,
      ),

    getProjectInstruments: (id: string) =>
      get<{ tools: Array<{ id: string; name: string; kind: string; scope: 'global' | 'project'; enabled: boolean }> }>(
        `/api/projects/${enc(id)}/instruments`, { tools: [] }
      ),

    // Per-project MCP tool aggregation — calls / errors / avg duration / FTR
    // scoped to the project. Returns EVERY manifest tool with usage overlaid
    // (zero-call rows included) so the UI can render the full catalogue.
    getProjectMcpToolStats: (id: string) =>
      get<{ tools: ProjectMcpToolStat[] }>(
        `/api/projects/${enc(id)}/mcp-tool-stats`, { tools: [] }
      ),

    // Services (MCP servers, inference providers) with per-project scope
    // resolved (scoped > global > default true).
    listProjectServices: (id: string) =>
      get<{ services: ProjectService[] }>(
        `/api/projects/${enc(id)}/services`, { services: [] }
      ),

    setProjectServiceScope: (id: string, serviceId: string, enabled: boolean) =>
      put(`/api/projects/${enc(id)}/services/${enc(serviceId)}/scope`, { enabled }),

    getProjectMemories: (id: string) =>
      get<{ active: ProjectMemory[]; total: number }>(
        `/api/projects/${enc(id)}/memories`, { active: [], total: 0 }
      ),

    // Memory share batches — the proposal / review / verdict lifecycle for
    // grouping memories before federating them out to a hive-mind.
    listMemoryShareBatches: (id: string, status?: string) =>
      get<{ batches: MemoryShareBatch[] }>(
        `/api/projects/${enc(id)}/memory-batches${status ? `?status=${enc(status)}` : ''}`,
        { batches: [] },
      ),

    createMemoryShareBatch: (id: string, memoryIds: string[], note?: string) =>
      post<{ id: string }>(
        `/api/projects/${enc(id)}/memory-batches`,
        { memory_ids: memoryIds, note },
        { id: '' },
      ),

    decideMemoryShareBatch: (id: string, batchId: string, status: 'approved' | 'rejected' | 'withdrawn', note?: string) =>
      put(
        `/api/projects/${enc(id)}/memory-batches/${enc(batchId)}`,
        { status, note },
      ),

    // Impact verdicts (manual log) — user-logged retrospectives about
    // shipped changes. Independent of the automatic recommendation
    // verdicts on /api/projects/{id}/impact.
    listImpactVerdicts: (id: string, verdict?: string) =>
      get<{ verdicts: ImpactVerdictEntry[] }>(
        `/api/projects/${enc(id)}/impact-verdicts${verdict ? `?verdict=${enc(verdict)}` : ''}`,
        { verdicts: [] },
      ),

    createImpactVerdict: (id: string, title: string, note?: string, sessionId?: string) =>
      post<{ id: string }>(
        `/api/projects/${enc(id)}/impact-verdicts`,
        { title, note, session_id: sessionId },
        { id: '' },
      ),

    decideImpactVerdict: (id: string, verdictId: string, verdict: 'success' | 'mixed' | 'failure', note?: string) =>
      put(
        `/api/projects/${enc(id)}/impact-verdicts/${enc(verdictId)}`,
        { verdict, note },
      ),

    // Measured impact reports — the analyzer's before/after FTR + MOE-style
    // consensus for recommendations that carry a reasoning trace or a
    // non-pending verdict. Distinct from the manual `listImpactVerdicts`
    // retrospectives above.
    getProjectImpact: (id: string) =>
      get<Array<{
        id: string; title: string; actionType: string; status: string;
        verdict: string; baselineFtr: number | null; currentFtr: number | null;
        ftrDelta: number | null;
        props: Record<string, unknown>;
        // Honest reasoning JSON — `null` when the rec has no trace, otherwise
        // `{headline, body, modelsUsed: string[], suggestedRevision}`. One
        // FTR-delta verdict, so no fabricated consensus tally or per-model
        // roles/notes; `modelsUsed` lists the models that actually ran. Legacy
        // `{conclusion}`-shape traces are shimmed on the daemon side.
        reasoning: {
          headline: string | null;
          body: string | null;
          modelsUsed: string[];
          suggestedRevision: string | null;
        } | null;
      }>>(`/api/projects/${enc(id)}/impact`, []),

    getProjectDrift: (id: string) =>
      get<{ items: DriftItem[]; total: number; drifted: number; broken: number }>(
        `/api/projects/${enc(id)}/drift`, { items: [], total: 0, drifted: 0, broken: 0 }
      ),

    getProjectPatterns: (id: string) =>
      get<{ followed: PatternEntry[]; antiPatterns: PatternEntry[] }>(
        `/api/projects/${enc(id)}/patterns`, { followed: [], antiPatterns: [] }
      ),

    getProjectRecommendations: (id: string, status?: string) =>
      get<Recommendation[]>(
        `/api/projects/${enc(id)}/recommendations${status ? `?status=${enc(status)}` : ''}`, []
      ),

    // Result-based variant: a recommendations-only fetch failure (the /metrics
    // leg can succeed independently) must SURFACE, not collapse to [] — otherwise
    // a 500/timeout is indistinguishable from a genuinely-empty list on a
    // governance-facing screen. Callers that tolerate the legacy swallow keep
    // getProjectRecommendations above.
    tryGetProjectRecommendations: (id: string, status?: string) =>
      tryGet<Recommendation[]>(
        `/api/projects/${enc(id)}/recommendations${status ? `?status=${enc(status)}` : ''}`,
      ),

    // Gap 1 fix — expose the accept/reject flow so MeasureVerdicts has
    // work to measure. Each returns { ok } on success or 409 CONFLICT if
    // the rec was already decided.
    acceptProjectRecommendation: (id: string, recId: string) =>
      post<{ ok: boolean }>(
        `/api/projects/${enc(id)}/recommendations/${enc(recId)}/accept`, {}, { ok: false },
      ),
    rejectProjectRecommendation: (id: string, recId: string) =>
      post<{ ok: boolean }>(
        `/api/projects/${enc(id)}/recommendations/${enc(recId)}/reject`, {}, { ok: false },
      ),

    // P-A: rule-class accept that MATERIALIZES a governance rule (spec 2026-08-20).
    // `preview` renders what would be written (no write); `materialize` accepts +
    // writes the rule at the chosen scope/enforcement (defaults: project / recommended).
    previewRecommendation: (id: string, recId: string) =>
      get<{
        materializable: boolean; kind?: string; action_type?: string; reason?: string;
        title?: string; body?: string; impact?: string | null; gov_scope?: string; enforcement?: string;
      }>(`/api/projects/${enc(id)}/recommendations/${enc(recId)}/preview`, { materializable: false }),
    materializeRecommendation: (
      id: string, recId: string,
      opts?: { gov_scope?: string; namespace_id?: string; enforcement?: string; title?: string; body?: string },
    ) =>
      post<{ ok: boolean; materialized?: unknown }>(
        `/api/projects/${enc(id)}/recommendations/${enc(recId)}/materialize`, opts ?? {}, { ok: false },
      ),

    // ── Observatory · Insights (Slot 5) ─────────────────────────────────
    // Server-side triage aggregator: bundles recs, memories, patterns and
    // corrections pre-bucketed into Now / Soon / Settled. `project` scopes
    // all three columns to one project (name-or-uuid). Fallback is the empty
    // board so a daemon hiccup renders the quiet-state, never a broken screen.
    getInsights: (project?: string) =>
      get<InsightsBoard>(
        `/api/insights${project ? `?project=${enc(project)}` : ''}`,
        {
          counts: { now: 0, soon: 0, settled: 0 },
          projects: [], recommendations: [], memories: [], patterns: [], corrections: [],
        },
      ),

    // ── Front door · Intake ─────────────────────────────────────────────
    // The guide (frame + axis prompts + catalog) for the intake screen.
    // Fallback is the empty guide so a daemon hiccup renders the quiet state.
    getIntakeGuide: () =>
      get<IntakeGuide>('/api/playbook/guide', { frame: '', axes: [], playbooks: [] }),

    // Classify + recommend a playbook. `{ chunk, preview: true }` previews
    // (no row written); `{ lifecycle, intent, risk, confirm: true }` records
    // the confirmed run. tryPost so the form can surface errors.
    recommendPlaybook: (body: Record<string, unknown>) =>
      tryPost<PlaybookRecommendation>('/api/playbook/recommend', body),

    getProjectSessions: (id: string, limit = 50) =>
      get<{ sessions: ProjectSession[] }>(
        `/api/projects/${enc(id)}/sessions?limit=${limit}`, { sessions: [] }
      ),

    // ── Project window · Overview (Slot 4) ──────────────────────────────
    // Server-assembled landing pane: header + top rec + stats + recent
    // sessions in one call. Fallback is the all-quiet shape so a daemon
    // hiccup renders the calm empty pane, never a broken screen.
    getProjectOverview: (id: string) =>
      get<ProjectOverview>(`/api/projects/${enc(id)}/overview`, {
        project: {
          id, name: '', kanji: '場', client: null, goal: null,
          ftr: 0, warn: false, sessions7d: 0, folders: [],
        },
        top_recommendation: null,
        stats: {
          sessions7d: 0, sessions7dCorrected: 0,
          memories: { total: 0, readyToShare: 0, toMerge: 0 },
          docDrift: { open: 0, referencedDocs: 0 },
        },
        recentSessions: [],
      }),

    // ── Observatory · Today (home screen) ───────────────────────────────
    // The daemon assembles the whole screen — greeting, maturity gate, hero
    // koan, insights, adopted lane, recent sessions. The screen renders it;
    // it does not decide early-vs-mature. Fallback is the early state so a
    // daemon hiccup degrades to "still listening", never a broken screen.
    getObservatoryToday: () =>
      get<ObservatoryToday>('/api/observatory/today', {
        greeting: '',
        today: '',
        dataMaturity: 'early',
        hero: { kanji: '観', koan: '', body: '', impact: null, action: null, source: '', noticed: '' },
        insights: [],
        adopted: [],
        recentSessions: [],
      }),

    // Holistic FTR headline (14d / prior / trend / 7d) → Today header chip.
    getObservatoryFtr: () =>
      get<ObservatoryFtr>('/api/observatory/ftr', { ftr14d: null, ftr14dPrev: null, ftrTrend: [], sessions7d: 0 }),

    // ── Observatory chart data ──────────────────────────────────────────

    getHolisticFtrDaily: (days = 14) =>
      get<{ ftr_daily: Array<{ day: string; ftr_rate: number; session_count: number }> }>(
        `/api/observatory/ftr-daily?days=${days}`, { ftr_daily: [] }
      ),

    getProjectFtrDaily: (id: string, days = 14) =>
      get<{ ftr_daily: Array<{ day: string; ftr_rate: number; session_count: number }> }>(
        `/api/projects/${enc(id)}/ftr-daily?days=${days}`, { ftr_daily: [] }
      ),

    getProjectHotspots: (id: string, days = 7) =>
      get<{ hotspots: Array<{ folder: string; file_path: string; edit_count: number; correction_count: number }> }>(
        `/api/projects/${enc(id)}/hotspots?days=${days}`, { hotspots: [] }
      ),

    getProjectQualitySignals: (id: string) =>
      get<{ ftr_7d: number; pattern_compliance: number | null; open_drift_count: number; test_pass_rate: number | null }>(
        `/api/projects/${enc(id)}/quality-signals`,
        { ftr_7d: 0, pattern_compliance: null, open_drift_count: 0, test_pass_rate: null }
      ),

    getProjectTeachings: (id: string, limit = 10) =>
      get<{ teachings: Array<{ id: string; name: string; family: string | null; instance_count: number; modified_at: string }> }>(
        `/api/projects/${enc(id)}/teachings?limit=${limit}`, { teachings: [] }
      ),

    getToolUsage: () =>
      get<{ tools: Array<{ tool_name: string; call_count: number; error_count: number; avg_duration_ms: number | null; last_used_at: string }> }>(
        '/api/observatory/tool-usage', { tools: [] }
      ),

    getToolSignals: () =>
      get<{ signals: ToolSignal[]; source?: 'cache' | 'derived' }>(
        '/api/observatory/tool-signals', { signals: [] }
      ),

    // Cached per-tool snapshots (T2 Slice D) — full metrics + optional
    // signal card per tool. Populated by the AggregateToolInsights task.
    getToolInsights: () =>
      get<{ insights: ToolInsight[] }>(
        '/api/observatory/tool-insights', { insights: [] }
      ),

    // L1 source grid for Instruments · Health — one card per registered
    // source (builtin tool set or probed MCP server) with the share of
    // registered tools actually invoked in the last 14 days.
    getToolsHealth: () =>
      get<ToolsHealth>('/api/instruments/tools-health', { sources: [] }),

    getLibraryUsage: (id: string) =>
      get<{ usage: Array<{ library_name: string; folder: string; version_used: string | null; import_count: number }> }>(
        `/api/libs/${enc(id)}/usage`, { usage: [] }
      ),

    // ── Indexing ─────────────────────────────────────────────────────────
    indexRepo: (repoId: string, repoPath: string, force = false) =>
      post<{ ok: boolean; queued: boolean; taskId: number }>(
        '/api/index', { repoId, repoPath, force }, { ok: false, queued: false, taskId: -1 },
      ),

    getIndexStatus: () =>
      get<{ queue: { pending: number; blocked: number; running: number; completed: number }; repos: Record<string, { total: number; pending: number; running: number }> }>(
        '/api/index/status', { queue: { pending: 0, blocked: 0, running: 0, completed: 0 }, repos: {} },
      ),

    getIndexDirty: () => get<DirtyStatus[]>('/api/index/dirty', []),

    getIndexErrors: (repoId?: string) =>
      get<IndexError[]>(repoId ? `/api/index/errors/${enc(repoId)}` : '/api/index/errors', []),

    /** Subscribe to index progress via SSE. Returns an EventSource. */
    subscribeIndexProgress: (): EventSource =>
      new EventSource(`${base}/api/index/progress`),

    // ── Graph ────────────────────────────────────────────────────────────
    /** Legacy — returns graph data in old format for existing UI. */
    getGraph: async (repoId: string, _repoPath?: string) => {
      const { nodes, edges } = await get<{ nodes: GraphNode[]; edges: GraphEdge[] }>(
        `/api/graph/nodes?repoId=${enc(repoId)}`, { nodes: [], edges: [] },
      );
      return {
        summary: { totalSymbols: nodes.length, totalEdges: edges.length, communities: 0 },
        communities: [] as GraphData['communities'],
        godNodes: [] as GraphData['godNodes'],
        rationale: [] as GraphData['rationale'],
      } satisfies GraphData;
    },

    getGraphNodes: (repoId: string) =>
      get<{ nodes: GraphSymbolNode[]; edges: GraphCallEdge[] }>(
        `/api/graph/nodes?repoId=${enc(repoId)}`, { nodes: [], edges: [] },
      ),

    searchFunctions: (repoId: string, q: string) =>
      get<FunctionDetail[]>(`/api/graph/functions?repoId=${enc(repoId)}&q=${enc(q)}`, []),

    searchTypes: (repoId: string, q: string) =>
      get<TypeDetail[]>(`/api/graph/types?repoId=${enc(repoId)}&q=${enc(q)}`, []),

    getCallers: (repoId: string, name: string) =>
      get<FunctionDetail[]>(`/api/graph/callers?repoId=${enc(repoId)}&name=${enc(name)}`, []),

    getCallees: (repoId: string, name: string) =>
      get<FunctionDetail[]>(`/api/graph/callees?repoId=${enc(repoId)}&name=${enc(name)}`, []),

    getFilesByTag: (repoId: string, tag: string) =>
      get<Array<{ id: string; path: string; tags: string }>>(
        `/api/graph/files?repoId=${enc(repoId)}&tag=${enc(tag)}`, [],
      ),

    getCommunities: (repoId: string) =>
      get<CommunityInfo[]>(`/api/graph/communities/info?repoId=${enc(repoId)}`, []),

    detectCommunities: (repoId: string) =>
      post<{ ok: boolean; communities: number }>('/api/graph/communities', { repoId }, { ok: false, communities: 0 }),

    getCallFlow: (repoId: string) =>
      get<{ modules: CallFlowModule[]; calls: CallFlowCall[]; moduleCount: number; exportCount: number; callCount: number }>(
        `/api/graph/call-flow?repoId=${enc(repoId)}`, { modules: [], calls: [], moduleCount: 0, exportCount: 0, callCount: 0 },
      ),

    getDocDrift: (repoId: string) =>
      get<DocDrift[]>(`/api/graph/doc-drift?repoId=${enc(repoId)}`, []),

    // ── Libraries ────────────────────────────────────────────────────────
    getLibs: (params?: { repoId?: string; projectId?: string; shared?: boolean }) => {
      const qs = new URLSearchParams();
      if (params?.repoId) qs.set('repoId', params.repoId);
      if (params?.projectId) qs.set('projectId', params.projectId);
      if (params?.shared) qs.set('shared', 'true');
      return get<{ total: number; libs: LibEntry[] }>(`/api/libs?${qs}`, { total: 0, libs: [] });
    },

    indexLib: (libName: string, url: string, version?: string) =>
      post('/api/libs/index', { libName, url, version }, { ok: false }),

    getLibDocs: (name: string) =>
      get<LibDoc[]>(`/api/libs/${enc(name)}/docs`, []),

    searchLibDocs: (q: string) =>
      get<LibDoc[]>(`/api/libs/docs?q=${enc(q)}`, []),

    getDepVersions: (repoId: string) =>
      get<DepVersion[]>(`/api/libs/versions?repoId=${enc(repoId)}`, []),

    // ── Unified Query ────────────────────────────────────────────────────
    query: (q: string, repoId?: string, projectId?: string) =>
      post<Record<string, unknown>>('/api/query', { q, repoId, projectId }, {}),

    // ── Sessions ─────────────────────────────────────────────────────────
    getSessions: () =>
      get<SessionData>('/api/sessions', { stats: null, sessions: [], toolUsage: [], benchmarkPairs: [] }),

    // Observatory · Sessions digest (Slot 6). Same `/api/sessions` handler,
    // now scoped by an optional `range` (7d|30d|90d) and/or `project`
    // (name-or-uuid). Both are additive server-side. Fallback is the empty
    // digest so a daemon hiccup renders the quiet state, never a broken screen.
    getSessionsDigest: (range?: string, project?: string) => {
      const p = new URLSearchParams();
      if (range) p.set('range', range);
      if (project) p.set('project', project);
      const qs = p.toString() ? `?${p.toString()}` : '';
      return get<SessionsDigest>(`/api/sessions${qs}`, { sessions: [] });
    },

    getMetrics: (project: string) =>
      get<Record<string, unknown>>(`/api/metrics/${encodeURIComponent(project)}`, {}),

    // ── Observatory · Logs (activity logs) ──────────────────────────────
    // Structured daemon/cli/mcp/app log rows for the Logs screen. All filters
    // are optional server query params: exact `level` / `source` / `module`
    // match, `since` (relative `15m|1h|24h|7d`, RFC-3339, or `all`), and a row
    // `limit` (clamped daemon-side to 1000). Newest-first. Fallback is the
    // empty array so a daemon hiccup renders the quiet state, never a broken
    // screen. An unparseable `since` is a 400 daemon-side → empty via fallback.
    getLogs: (q: { level?: string; source?: string; module?: string; since?: string; limit?: number } = {}) => {
      const p = new URLSearchParams();
      if (q.level) p.set('level', q.level);
      if (q.source) p.set('source', q.source);
      if (q.module) p.set('module', q.module);
      if (q.since) p.set('since', q.since);
      if (q.limit !== undefined) p.set('limit', String(q.limit));
      const qs = p.toString() ? `?${p.toString()}` : '';
      return get<LogRow[]>(`/api/logs${qs}`, []);
    },

    // Background-worker registry + last-run times (#96). Fallback empty so a
    // daemon hiccup renders the empty state, never a broken panel.
    getScheduledTasks: () =>
      get<{ tasks: ScheduledTask[] }>('/api/tasks/scheduled', { tasks: [] }),

    // ── Scan ─────────────────────────────────────────────────────────────
    /** Add a root to the DB immediately (synchronous). Does not start scanning. */
    addWatchRoot: (path: string) =>
      post<{ ok: boolean; id: string; path: string }>(
        '/api/scan/roots', { path }, { ok: false, id: '', path },
      ),

    /** Remove a watch root from the DB by its UUID. */
    removeWatchRoot: (id: string) =>
      del(`/api/scan/roots/${enc(id)}`),

    scanFolder: (root: string, maxDepth = 3) =>
      post<{ ok: boolean; scanning: boolean }>(
        '/api/scan', { root, max_depth: maxDepth }, { ok: false, scanning: false },
      ),

    // ── MCP Tool Proxy ────────────────────────────────────────────────
    mcpListTools: () => get<{ tools: McpToolManifest[] }>('/api/mcp/tools', { tools: [] }),

    mcpCallTool: (tool: string, params: Record<string, string>) =>
      post<Record<string, unknown>>('/api/mcp/call', { tool, params }, {}),

    // Session tool-call timeline for the Instruments Replay tab.
    getSessionToolTimeline: (sessionId: string, limit = 200) =>
      get<SessionToolTimeline>(
        `/api/sessions/${enc(sessionId)}/tool-timeline?limit=${limit}`,
        { sessionId, calls: [], count: 0 },
      ),

    // #84 T2 Slice C — Replay timeline with #90 verdicts joined + session
    // summary. Set `classify: true` to run the classifier before the read
    // (idempotent — safe on every open) so verdicts populate on first view.
    getSessionReplay: (sessionId: string, opts: { limit?: number; classify?: boolean } = {}) => {
      const p = new URLSearchParams();
      p.set('limit', String(opts.limit ?? 200));
      if (opts.classify) p.set('classify', 'true');
      return get<SessionReplayResponse>(
        `/api/sessions/${enc(sessionId)}/replay?${p.toString()}`,
        { sessionId, calls: [], count: 0, summary: { used: 0, partial: 0, ignored: 0, total: 0 }, classified: 0 },
      );
    },

    // #84 T2 Slice A — discovered MCP servers (Claude / Cursor / Zed configs).
    getMcpServers: (projectId?: string) => {
      const q = projectId ? `?project_id=${enc(projectId)}` : '';
      return get<{ servers: McpServerRow[] }>(
        `/api/instruments/mcp-servers${q}`, { servers: [] },
      );
    },
    setMcpServerEnabled: (id: string, enabled: boolean) =>
      put(`/api/instruments/mcp-servers/${enc(id)}/enabled`, { enabled }),
    refreshMcpServers: () =>
      post<{ discovered: number; pruned: number }>(
        '/api/instruments/mcp-servers/refresh', {}, { discovered: 0, pruned: 0 },
      ),

    // #84 T2 Slice B — cached tool manifest per server. `?refresh=true`
    // forces a re-probe; `?probe=false` returns stale-badged cache without
    // spawning.
    getMcpServerTools: (
      serverId: string,
      opts: { refresh?: boolean; probe?: boolean } = {},
    ) => {
      const p = new URLSearchParams();
      if (opts.refresh) p.set('refresh', 'true');
      if (opts.probe === false) p.set('probe', 'false');
      const qs = p.toString() ? `?${p.toString()}` : '';
      return get<McpServerToolsManifest>(
        `/api/instruments/mcp-servers/${enc(serverId)}/tools${qs}`,
        {
          id: '', server_id: serverId, tools: [], tool_count: 0,
          probed_at: '', ttl_seconds: 900, error: null,
          protocol_version: null, server_name: null, server_version: null,
          age_seconds: 0,
        },
      );
    },

    // ── Marketplace ──────────────────────────────────────────────────
    marketplaceInstall: (target: string, marketplacePath: string, item?: string, scope?: string) =>
      post('/api/marketplace/install', { target, marketplacePath, item, scope }, { ok: false }),

    // ── Config (user preferences) ──────────────────────────────────────
    getConfig: () => get<Record<string, string>>('/api/config', {}),

    getConfigKey: (key: string) =>
      get<{ key: string; value: string | null }>(`/api/config/${enc(key)}`, { key, value: null }),

    setConfig: (config: Record<string, string>) =>
      put('/api/config', config),

    /** Error-propagating variant of setConfig — required where drift between
     *  daemon and localStorage caches would corrupt downstream state. */
    trySetConfig: (config: Record<string, string>) =>
      tryPut('/api/config', config),

    /** Error-propagating variant of getConfig — used by appState.load() to
     *  distinguish "daemon unreachable" (don't touch cache) from "daemon says
     *  empty config" (clear cache). */
    tryGetConfig: () => tryGet<Record<string, string>>('/api/config'),

    deleteConfig: (key: string) => del(`/api/config/${enc(key)}`),

    // ── Assistants ────────────────────────────────────────────────────────
    detectAssistants: () => get<import('./types').AssistantStatus[]>('/api/assistants/detect', []),

    detectAssistantFamilies: () => get<import('./types').AssistantFamily[]>('/api/assistants/families', []),

    configureAssistants: (assistants: string[]) =>
      post<import('./types').AssistantConfigureResult>('/api/assistants/configure', { acps: assistants }, { configured: [], skipped: [], errors: [] }),

    removeAssistants: (assistants: string[] = []) =>
      post<import('./types').AssistantRemoveResult>('/api/assistants/remove', { acps: assistants }, { assistants_removed: [], errors: [] }),

    // ── Instruments (MCP registry — setup wizard Instruments stage) ───────
    listInstruments: () =>
      get<{ total: number; mcps: import('./setup/contracts').DaemonMcpEntry[]; stack: string[] }>(
        '/api/instruments',
        { total: 0, mcps: [], stack: [] },
      ),

    // ── Installer ───────────────────────────────────────────────────────
    installAll: (assistants: string[], scope = 'global') =>
      post<import('./types').InstallResult>('/api/install', { acps: assistants, scope }, {
        hooks_installed: 0, skills_installed: 0, commands_installed: 0,
        stale_commands_removed: 0, stale_skills_removed: 0,
        assistants_configured: [], errors: [], marketplace_version: '',
      }),

    installHooks: () =>
      post<{ ok: boolean; count: number }>('/api/install/hooks', {}, { ok: false, count: 0 }),

    installItem: (name: string, kind: string) =>
      post<{ ok: boolean; path?: string; error?: string }>('/api/install/item', { name, kind }, { ok: false }),

    removeItem: (name: string, kind: string) =>
      post<{ ok: boolean }>('/api/install/item/remove', { name, kind }, { ok: false }),

    getCatalog: () =>
      get<import('./types').MarketplaceCatalog>('/api/install/catalog', { version: null, items: [] }),

    getInstalledItems: () =>
      get<import('./types').InstalledItem[]>('/api/install/installed', []),

    /** Toggle a skill or command. The daemon moves the .md file between
     *  `~/.claude/<kind>s/` and its `disabled/` sibling — Claude Code
     *  scans only the live folder. Idempotent server-side (the daemon
     *  returns `changed: false` when the item was already in the target
     *  state, but we discard that here — re-fetch the list to reflect
     *  new state).
     *  Error codes: 400 unknown kind, 404 unknown item or ambiguous
     *  state (item present in both live + disabled folders). */
    setInstalledItemEnabled: (name: string, kind: string, enabled: boolean) =>
      tryPut(
        `/api/install/installed/${enc(name)}/enabled`,
        { kind, enabled },
      ),

    removeAll: (purge = false) =>
      post<import('./types').RemoveResult>('/api/remove', { purge }, {
        assistants_removed: [], plugin_removed: false, commands_removed: 0,
        skills_removed: 0, agents_removed: 0, hooks_removed: false,
        cache_cleared: false, projects_cleaned: [], errors: [],
      }),

    // ── Gateway routers ──────────────────────────────────────────────────
    listGatewayRouters: () =>
      get<{ routers: import('./setup/contracts').DaemonRouter[] }>(
        '/api/gateway/routers',
        { routers: [] },
      ),

    setGatewayRouterKey: (id: string, key: string) =>
      tryPost<{ ok: boolean; configured: boolean }>(
        `/api/gateway/routers/${enc(id)}/key`,
        { key },
      ),

    clearGatewayRouterKey: (id: string) =>
      tryDelete(`/api/gateway/routers/${enc(id)}/key`),

    // Model Assignments — chains carry an optional `role` column; a
    // chain-with-a-role IS the role assignment. Utility chains
    // (consensus-* / classify) stay null and never surface in the picker.
    listGatewayChains: () =>
      get<{ chains: import('./setup/contracts').DaemonChain[] }>(
        '/api/gateway/chains',
        { chains: [] },
      ),

    /** Assign or clear a chain's sensei inference role. Pass `null` to
     *  unassign. Returns error status codes: 400 unknown role, 404
     *  unknown chain, 409 role already owned by another chain. */
    setGatewayChainRole: (id: string, role: import('./setup/contracts').SenseiRole | null) =>
      tryPut(
        `/api/gateway/chains/${enc(id)}/role`,
        { role },
      ),

    /** Chain-model editing — available picker source. Returns models
     *  with matching capability, reachable via `models_in_router`,
     *  minus the ones already in this chain. */
    listAvailableChainModels: (chainId: string) =>
      get<{ models: Array<{
        modelId: string; modelName: string; fullName: string;
        routerId: string; routerName: string;
      }>}>(`/api/gateway/chains/${enc(chainId)}/available-models`, { models: [] }),

    /** Append a (model, router) pair to the end of a chain's ordered
     *  list. Returns the new member id + assigned sequence order. */
    addGatewayChainModel: (chainId: string, modelId: string, routerId: string) =>
      tryPost<{ ok: boolean; memberId: string; sequenceOrder: number }>(
        `/api/gateway/chains/${enc(chainId)}/models`,
        { model_id: modelId, router_id: routerId },
      ),

    /** Remove a chain member row. Compacts remaining sequence orders
     *  server-side so the list stays contiguous. */
    removeGatewayChainModel: (chainId: string, memberId: string) =>
      tryDelete(`/api/gateway/chains/${enc(chainId)}/models/${enc(memberId)}`),

    /** Swap a chain member with its neighbour. direction = -1 (up) /
     *  +1 (down). `moved: false` in the response means at a boundary,
     *  not an error — the UI dims the arrow. */
    moveGatewayChainModel: (chainId: string, memberId: string, direction: -1 | 1) =>
      tryPut(
        `/api/gateway/chains/${enc(chainId)}/models/${enc(memberId)}/move`,
        { direction },
      ),

    generateImage: (body: {
      prompt: string;
      output_path?: string;
      model?: string;
      router?: string;
      size?: string;
      quality?: string;
      style?: string;
      n?: number;
    }) =>
      post<{ ok: boolean; paths: string[]; model?: string; router?: string }>(
        '/api/gateway/image/generate',
        body,
        { ok: false, paths: [] },
      ),

    // ── On-demand local-model provisioning ───────────────────────────────
    // The provisionable catalog with each model's current phase. Fallback is
    // the empty list so a daemon hiccup (or a non-embedded build) renders the
    // quiet state, never a broken panel; the UI can poll unconditionally.
    provisionStatus: () =>
      get<{ models: ProvisionModel[] }>(
        '/api/gateway/models/provision/status', { models: [] },
      ),

    // Begin (or join) an on-demand pull of a local model, returning its initial
    // phase. `tryPost` so failures propagate: a 501 (embedded provisioning not
    // available in this build) surfaces as `{ ok: false, error: { status: 501,
    // … } }` and the UI shows the not-available notice rather than crashing.
    provisionModel: (id: string) =>
      tryPost<{ model: string; phase: ProvisionPhase }>(
        '/api/gateway/models/' + enc(id) + '/provision', {},
      ),

    // ── Knowledge plane ──────────────────────────────────────────────────
    listMemories: (q: { status?: string; scope?: string; project_id?: string; limit?: number } = {}) => {
      const p = new URLSearchParams();
      for (const [k, v] of Object.entries(q)) if (v !== undefined) p.set(k, String(v));
      return get<MemoryListResponse>(`/api/knowledge/memories?${p.toString()}`, { memories: [] });
    },

    getMemoryDetail: (id: string) =>
      tryGet<MemoryDetail>(`/api/knowledge/memories/${encodeURIComponent(id)}`),

    getLayeredContext: (project_id: string, opts: { limit?: number; tags?: string[] } = {}) => {
      const p = new URLSearchParams({ project_id });
      if (opts.limit !== undefined) p.set('limit', String(opts.limit));
      if (opts.tags?.length) p.set('tags', opts.tags.join(','));
      return get<ContextResponse>(`/api/knowledge/context?${p.toString()}`,
                                   { version: '', memories: [], cache_until: '' });
    },

    proposeMemory: (body: ProposalCreateBody) =>
      tryPost<{ id: string; status: 'proposed' }>('/api/knowledge/proposals', body),

    saveMemory: (body: MemoryCreateBody) =>
      tryPost<{ id: string; status: 'active' }>('/api/knowledge/memories', body),

    // Ready-to-share lane — rewrite a project-scoped memory into a portable,
    // project-agnostic rule. `tryPost` so the 503 honest-degrade path (model
    // unavailable / returned nothing usable) surfaces as `{ ok: false, error }`
    // instead of a fabricated rewrite — the UI shows the error, keeps the
    // original. On success the daemon flips `generalised = true` and stores the
    // rewrite, so the caller re-fetches to reflect the new chip state.
    generaliseMemory: (id: string) =>
      tryPost<{ id: string; original: string; generalised: string }>(
        `/api/knowledge/memories/${enc(id)}/generalise`, {},
      ),

    // Widen a proven memory up the scope ladder (project → user → org →
    // collective). Reuses the EXISTING promote route — it creates a `proposed`
    // copy on the target namespace (origin=promoted); accepting it through the
    // normal proposal flow is the governance gate, so it never auto-applies.
    // `gov_scope` names the target scope ("user" | "org" | "global"); the other
    // fields stay optional for namespace/folder-resolved promotions.
    promoteMemory: (
      id: string,
      body: { gov_scope?: string; namespace_id?: string; folder?: string; enforcement?: string } = {},
    ) =>
      tryPost<{ id: string; status: string; origin: string }>(
        `/api/knowledge/memories/${enc(id)}/promote`, body,
      ),

    // ── Memory lifecycle (triage → active → archive) ─────────────────────
    // Deterministic status transitions for an existing memory. All POST, no
    // body (except merge). `tryPost` so the daemon's `{ "error": "..." }` /
    // 409-terminal path surfaces as `{ ok: false, error }` and the UI can
    // degrade gracefully instead of crashing — mirrors promote/generalise.

    // Retire a memory. → `archived`.
    archiveMemory: (id: string) =>
      tryPost<{ id: string; status: 'archived' }>(
        `/api/knowledge/memories/${enc(id)}/archive`, {},
      ),

    // Strengthen a memory that proved out. → still active, `reinforced: true`.
    reinforceMemory: (id: string) =>
      tryPost<{ id: string; reinforced: boolean }>(
        `/api/knowledge/memories/${enc(id)}/reinforce`, {},
      ),

    // Contest a memory. → `challenged` (stays in the active set). 409 when the
    // memory is already in a terminal state (archived / rejected).
    challengeMemory: (id: string) =>
      tryPost<{ id: string; status: 'challenged' }>(
        `/api/knowledge/memories/${enc(id)}/challenge`, {},
      ),

    // Reject a memory outright. → `rejected`. 409 when already terminal.
    dismissMemory: (id: string) =>
      tryPost<{ id: string; status: 'rejected' }>(
        `/api/knowledge/memories/${enc(id)}/dismiss`, {},
      ),

    // Fold this memory into a surviving one (`into`). The folded memory is
    // archived. 400 on a missing/self merge, 404 when the survivor is unknown.
    // No merge-target picker exists on the triage screen yet, so this is not
    // wired to a button — kept here for when that affordance lands.
    mergeMemory: (id: string, into: string) =>
      tryPost<{ id: string; into: string; status: 'archived' }>(
        `/api/knowledge/memories/${enc(id)}/merge`, { into },
      ),

    acceptProposal: (id: string) =>
      tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/accept`, {}),

    rejectProposal: (id: string, reason?: string) =>
      tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/reject`, { reason }),

    recordOutcomes: (outcomes: OutcomeBody[]) =>
      tryPost<OutcomesBatchResponse>('/api/knowledge/outcomes', { outcomes }),

    // ── Governance Tier-2 · ruleset consolidation ────────────────────────
    // Tier-1 gathers a scope's raw rules; Tier-2 asks a model to merge them into
    // one coherent, deduped ruleset; a human approves. The current merged
    // ruleset (approved if present, else latest proposed) or `null` when a
    // consolidation has never run. Fallback is `null` so a daemon hiccup renders
    // the empty state, never a broken screen.
    // Error-propagating (mockup-drift-audit F8): `null` means "never consolidated"
    // (honest-empty). A fetch FAILURE must not collapse to that same null — tryGet
    // surfaces {ok:false,error} so the screen shows error-with-Retry instead.
    tryGetConsolidatedRuleset: () =>
      tryGet<ConsolidatedRuleset | null>('/api/knowledge/rules/consolidated'),

    // Run the merge now → a fresh `proposed` version, or `{ skipped, reason }`
    // when there's nothing to merge / the Tier-1 input is unchanged. `tryPost` so
    // a 502 (merge model unavailable) surfaces as `{ ok: false, error }` for the
    // screen to show, rather than being absorbed into a fallback.
    consolidateRules: () =>
      tryPost<ConsolidateResult>('/api/knowledge/rules/consolidate', {}),

    // Approve a proposed consolidated ruleset (the approval gate). Supersedes the
    // prior approved version and re-materializes the global rules.md. `tryPost` so
    // a 404 (unknown ruleset) surfaces to the screen. On success the page
    // re-fetches to reflect the approved state.
    approveConsolidatedRuleset: (id: string) =>
      tryPost<{ id: string; status: 'approved'; scope: string }>(
        `/api/knowledge/rules/consolidate/${enc(id)}/approve`, {},
      ),

    // ── Knowledge plane — federation sources ─────────────────────────────
    listKnowledgeSources: () =>
      get<{ sources: KnowledgeSource[] }>('/api/knowledge/sources', { sources: [] }),

    createKnowledgeSource: (body: NewKnowledgeSourceBody) =>
      tryPost<KnowledgeSource>('/api/knowledge/sources', body),

    deleteKnowledgeSource: (id: string) =>
      tryDelete(`/api/knowledge/sources/${encodeURIComponent(id)}`),

    syncKnowledgeSource: (id: string) =>
      tryPost<SyncStats>(`/api/knowledge/sources/${encodeURIComponent(id)}/sync`, {}),

    // ── Dōjō connections (memberships) ───────────────────────────────────
    // Mirrors the federation knowledge-sources surface. GET returns a
    // top-level array (empty when no Dōjō is connected — the honest empty
    // state). `credential_ref` is never exposed; the device token on connect
    // is write-only. Fallback is the empty array so a daemon hiccup renders
    // the empty state, never a broken screen.
    getDojoMemberships: () =>
      get<DojoMembership[]>('/api/dojo/memberships', []),

    // Register a Dōjō connection. `tryPost` so the validation/registration
    // errors (bad membership uuid, insecure registry url, unknown project to
    // bind) surface as `{ ok: false, error }` for the connect form to show.
    connectDojo: (body: ConnectDojoBody) =>
      tryPost<{ id: string }>('/api/dojo/memberships', body),

    // Replace the git-remote owner slugs a membership covers (org-tagging) —
    // drives the R3 infer-at-detect auto-bind. `tryPut` so a 404 (unknown
    // membership) surfaces to the form.
    setMembershipOrgs: (membershipId: string, orgSlugs: string[]) =>
      tryPut(`/api/dojo/memberships/${enc(membershipId)}/orgs`, { org_slugs: orgSlugs }),

    // R3 infer-at-detect: the inferred (confirm-inferred) project→Dōjō binding
    // for the About-panel chip. `{ suggestion: null }` when already bound / no
    // git owner / no covering membership. Read-only — suggests, never binds.
    getDojoSuggestion: (projectId: string) =>
      get<{ suggestion: DojoBindingSuggestion | null }>(
        `/api/projects/${enc(projectId)}/dojo-suggestion`,
        { suggestion: null },
      ),

    // Confirm a project→Dōjō binding (the user accepting the inferred chip, or
    // an explicit bind). Fails closed server-side if the membership is unknown.
    bindProjectDojo: (projectId: string, membershipId: string) =>
      tryPost<{ ok: boolean; dojo_id: string }>(
        `/api/projects/${enc(projectId)}/dojo-binding`,
        { membership_id: membershipId },
      ),

    // ── Dōjō downstream inbox (Observatory · Upgrades · C7) ──────────────
    // The daemon's inbox of approved Dōjō / Collective artifacts pulled back
    // for the user to review. Muted are hidden unless `includeMuted`; pinned
    // float to the top; `unread_count` counts the still-pending items. Fallback
    // is the empty inbox so a daemon that predates the route (404) degrades to
    // the empty state, never a broken screen.
    getUpgrades: (includeMuted = false) =>
      get<DojoUpgradesResponse>(
        `/api/upgrades${includeMuted ? '?include_muted=1' : ''}`,
        { artifacts: [], unread_count: 0 },
      ),

    // Apply / Mute / Pin one artifact. `tryPost` so a failure surfaces as
    // `{ ok: false, error }` for the row to show and retry, rather than being
    // absorbed into a fallback — the screen re-loads on success via
    // `invalidateAll`. Apply lands a principle/pattern as a dojo memory
    // (skill/agent/prompt/guard defer); Mute hides it locally; Pin floats it to
    // the top and lets it outrank ambiguous local alternatives.
    applyUpgrade: (id: string) =>
      tryPost<Record<string, unknown>>(`/api/upgrades/${enc(id)}/apply`, {}),
    muteUpgrade: (id: string) =>
      tryPost<{ id: string; state: string }>(`/api/upgrades/${enc(id)}/mute`, {}),
    pinUpgrade: (id: string) =>
      tryPost<{ id: string; state: string }>(`/api/upgrades/${enc(id)}/pin`, {}),

    // ── Share review (Observatory · Share review · C11) ──────────────────
    // The upstream publish-gate. GET returns the next approved-but-unsent batch
    // with its per-item dereference PREVIEW, or `{ batch: null }` when nothing is
    // pending; the `get` fallback is that empty state so a daemon that predates
    // the route (404) degrades to the empty screen, never a broken one. Publish
    // runs the confidentiality-gated contribute path (client-work dereference is
    // mandatory and cannot be overridden); `tryPost` so a missing (404) or
    // not-approved (409) batch surfaces as `{ ok: false, error }` for the screen
    // to show, rather than being absorbed into a fallback. On success the screen
    // re-loads via `invalidateAll` and the returned outcome is shown.
    // Error-propagating (mockup-drift-audit F8): `{ batch: null }` means nothing
    // is pending (honest-empty). A daemon hiccup must NOT masquerade as that —
    // tryGet surfaces the error so the screen shows error-with-Retry.
    tryGetShareReviewBatch: () =>
      tryGet<ShareReviewResponse>('/api/share-review/next-batch'),

    publishBatch: (batchId: string) =>
      tryPost<PublishBatchOutcome>(`/api/share-review/${enc(batchId)}/publish`, {}),

    // ── Collective sharing preferences (Observatory · Collective · C9) ───────
    // GET always returns a FULL object on success (the stored row, or the daemon's
    // defaults when unset) — so a FAILURE here is a real error, never "empty".
    // Error-propagating (mockup-drift-audit F8): tryGet surfaces {ok:false,error}
    // so the screen shows error-with-Retry rather than fabricating the defaults
    // and hiding a broken daemon. PUT is a whole-object full-replace that echoes
    // the saved object back (fresh updated_at); `tryPutJson` so a 400 (bad enum /
    // unknown category / non-boolean) surfaces with the daemon's message.
    tryGetCollectivePreferences: () =>
      tryGet<CollectivePreferences>('/api/preferences/collective'),

    putCollectivePreferences: (body: CollectivePreferences) =>
      tryPutJson<CollectivePreferences>('/api/preferences/collective', body),

    // ── Lifecycle ────────────────────────────────────────────────────────
    stop: () => post('/stop', {}, {}),

    // ── Deprecated aliases (migration: solution→project, project→repo) ──
    /** @deprecated Use getRepos */
    get getProjects() { return this.getRepos; },
    /** @deprecated Use getRepoSummary */
    get getProjectSummary() { return this.getRepoSummary; },
    /** @deprecated Use registerRepo */
    get registerProject() { return this.registerRepo; },
    /** @deprecated Use addRepoTag */
    get addProjectTag() { return this.addRepoTag; },
    /** @deprecated Use removeRepoTag */
    get removeProjectTag() { return this.removeRepoTag; },
    /** @deprecated Use listProjects */
    get listSolutions() { return this.listProjects; },
    /** @deprecated Use createProject */
    get createSolution() { return this.createProject; },
    /** @deprecated Use updateProject */
    get updateSolution() { return this.updateProject; },
    /** @deprecated Use deleteProject */
    get deleteSolution() { return this.deleteProject; },
    /** @deprecated Use addProjectRepo */
    get addSolutionRepo() { return this.addProjectRepo; },
    /** @deprecated Use removeProjectRepo */
    get removeSolutionRepo() { return this.removeProjectRepo; },
    /** @deprecated Use getProjectGraph */
    get getSolutionGraph() { return this.getProjectGraph; },
    /** @deprecated Use getProjectRoles */
    get getSolutionRoles() { return this.getProjectRoles; },
    /** @deprecated Use analyzeProject */
    get analyzeSolution() { return this.analyzeProject; },
  };
}

export type SenseiApi = ReturnType<typeof senseiApi>;

function enc(s: string): string {
  return encodeURIComponent(s);
}
