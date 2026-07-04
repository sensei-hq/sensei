import type {
  ServerProject, ProjectSummary, GraphData, GraphNode, GraphEdge,
  SolutionGraphResponse, SolutionAnalysis, InferredRole,
  IndexQueueStatus, DirtyStatus, IndexError,
  FunctionDetail, TypeDetail, CommunityInfo, DocDrift,
  LibEntry, LibDoc, DepVersion, SessionData,
  ProjectMemory, DriftItem, PatternEntry, Recommendation,
  ProjectSession, CallFlowModule, CallFlowCall,
  ProjectListItem,
  KnowledgeSource, NewKnowledgeSourceBody, SyncStats,
  McpToolManifest, SessionToolTimeline, MemoryShareBatch, ImpactVerdictEntry,
  ProjectMcpToolStat, ToolSignal, ProjectService, ToolInsight,
} from './types.js';
import type {
  MemoryListResponse, MemoryDetail, ContextResponse,
  ProposalCreateBody, MemoryCreateBody, OutcomeBody, OutcomesBatchResponse,
} from './setup/contracts.js';

export type ApiError = { status: number; message: string } | { status: 0; message: string };

export type ApiResult<T> = { ok: true; data: T } | { ok: false; error: ApiError };

/** Create a typed API client for the sensei Rust daemon. */
export function senseiApi(port: number) {
  const base = `http://127.0.0.1:${port}`;

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
      get<{ ftr14d: number; ftr14dPrev: number; ftrTrend: number[]; sessions7d: number }>(
        `/api/projects/${enc(id)}/ftr`,
        { ftr14d: 0, ftr14dPrev: 0, ftrTrend: [], sessions7d: 0 }
      ),

    getProjectRepos: (id: string) =>
      get<{ repos: Array<{ id: string; name: string; path: string; kind: string; role?: string }> }>(
        `/api/projects/${enc(id)}/repos`, { repos: [] }
      ),

    getProjectLibraries: (id: string) =>
      get<{ libraries: Array<{
        id: string; name: string; ecosystem: string;
        scope: 'global' | 'project'; enabled: boolean;
        description?: string | null;
        hasDocs?: boolean; pageCount?: number;
        localSource?: string | null;
      }> }>(
        `/api/projects/${enc(id)}/libraries`, { libraries: [] }
      ),

    // T1a version-conflict view — surfaces libraries pinned to multiple
    // versions across the project's folders. One row per (project, library)
    // pair with the distinct versions + folders where each was seen.
    getProjectLibraryVersionConflicts: (id: string) =>
      get<{ conflicts: Array<{ library_id: string; library_name: string; ecosystem: string; versions: string[]; folders: string[] }> }>(
        `/api/projects/${enc(id)}/library-version-conflicts`, { conflicts: [] },
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
      get<{ active: ProjectMemory[]; total: number; pendingShare: number }>(
        `/api/projects/${enc(id)}/memories`, { active: [], total: 0, pendingShare: 0 }
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
        // Rich MOE reasoning JSON — `null` when the rec has no trace,
        // otherwise `{headline, body, consensus, models: [{name, role,
        // note}], suggestedRevision}`. Legacy `{conclusion}`-shape
        // traces are shimmed on the daemon side so the reader always
        // gets the panel shape when reasoning is present at all.
        reasoning: {
          headline: string | null;
          body: string | null;
          consensus: string | null;
          models: Array<{ name: string; role: string; note: string }>;
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

    getProjectSessions: (id: string, limit = 50) =>
      get<{ sessions: ProjectSession[] }>(
        `/api/projects/${enc(id)}/sessions?limit=${limit}`, { sessions: [] }
      ),

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
      get<{ nodes: GraphNode[]; edges: GraphEdge[] }>(
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

    getMetrics: (project: string) =>
      get<Record<string, unknown>>(`/api/metrics/${encodeURIComponent(project)}`, {}),

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

    acceptProposal: (id: string) =>
      tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/accept`, {}),

    rejectProposal: (id: string, reason?: string) =>
      tryPost<{ id: string; status: string }>(`/api/knowledge/proposals/${encodeURIComponent(id)}/reject`, { reason }),

    recordOutcomes: (outcomes: OutcomeBody[]) =>
      tryPost<OutcomesBatchResponse>('/api/knowledge/outcomes', { outcomes }),

    // ── Knowledge plane — federation sources ─────────────────────────────
    listKnowledgeSources: () =>
      get<{ sources: KnowledgeSource[] }>('/api/knowledge/sources', { sources: [] }),

    createKnowledgeSource: (body: NewKnowledgeSourceBody) =>
      tryPost<KnowledgeSource>('/api/knowledge/sources', body),

    deleteKnowledgeSource: (id: string) =>
      tryDelete(`/api/knowledge/sources/${encodeURIComponent(id)}`),

    syncKnowledgeSource: (id: string) =>
      tryPost<SyncStats>(`/api/knowledge/sources/${encodeURIComponent(id)}/sync`, {}),

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
