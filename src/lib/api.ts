import type {
  ServerProject, ProjectSummary, GraphData, GraphNode, GraphEdge,
  SolutionGraphResponse, SolutionAnalysis, InferredRole,
  IndexQueueStatus, DirtyStatus, IndexError,
  FunctionDetail, TypeDetail, CommunityInfo, DocDrift,
  LibEntry, LibDoc, DepVersion, SessionData,
} from './types.js';

/** Create a typed API client for the sensei Rust daemon. */
export function senseiApi(port: number) {
  const base = `http://127.0.0.1:${port}`;

  async function get<T>(path: string, fallback: T): Promise<T> {
    try {
      const res = await fetch(`${base}${path}`);
      return res.ok ? await res.json() as T : fallback;
    } catch { return fallback; }
  }

  async function post<T>(path: string, body: unknown, fallback: T): Promise<T> {
    try {
      const res = await fetch(`${base}${path}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      return res.ok ? await res.json() as T : fallback;
    } catch { return fallback; }
  }

  async function put(path: string, body: unknown) {
    try {
      await fetch(`${base}${path}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
    } catch { /* non-fatal */ }
  }

  async function del(path: string) {
    try { await fetch(`${base}${path}`, { method: 'DELETE' }); } catch { /* non-fatal */ }
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
      get<ProjectSummary>(`/api/repos/${enc(repoId)}/summary`, null as any),

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
    listProjects: () => get<any[]>('/api/projects', []),

    createProject: (project: any) =>
      post<{ ok: boolean; id?: string }>('/api/projects', project, { ok: false }),

    updateProject: (id: string, patch: any) =>
      put(`/api/projects/${enc(id)}`, patch),

    deleteProject: (id: string) => del(`/api/projects/${enc(id)}`),

    addProjectRepo: (projectId: string, repo: { repoId: string; role?: string }) =>
      post(`/api/projects/${enc(projectId)}/repos`, repo, { ok: false }),

    removeProjectRepo: (projectId: string, repoId: string) =>
      del(`/api/projects/${enc(projectId)}/repos/${enc(repoId)}`),

    getProjectGraph: (id: string) =>
      get<SolutionGraphResponse>(`/api/projects/${enc(id)}/graph`, null as any),

    getProjectRoles: (id: string) =>
      get<InferredRole[]>(`/api/projects/${enc(id)}/roles`, []),

    analyzeProject: (id: string) =>
      post<SolutionAnalysis>(`/api/projects/${enc(id)}/analyze`, {}, null as any),

    // ── Project detail (new multi-window endpoints) ───────────────────
    getProjectFtr: (id: string) =>
      get<{ ftr14d: number; ftr14dPrev: number; ftrTrend: number[]; sessions7d: number }>(
        `/api/projects/${enc(id)}/ftr`,
        { ftr14d: 0, ftr14dPrev: 0, ftrTrend: [], sessions7d: 0 }
      ),

    getProjectRepos: (id: string) =>
      get<{ repos: Array<{ id: string; name: string; path: string; kind: string }> }>(
        `/api/projects/${enc(id)}/repos`, { repos: [] }
      ),

    getProjectLibraries: (id: string) =>
      get<{ libraries: Array<{ id: string; name: string; ecosystem: string; scope: 'global' | 'project'; enabled: boolean }> }>(
        `/api/projects/${enc(id)}/libraries`, { libraries: [] }
      ),

    getProjectInstruments: (id: string) =>
      get<{ tools: Array<{ id: string; name: string; kind: string; scope: 'global' | 'project'; enabled: boolean }> }>(
        `/api/projects/${enc(id)}/instruments`, { tools: [] }
      ),

    getProjectMemories: (id: string) =>
      get<{ active: any[]; total: number; pendingShare: number }>(
        `/api/projects/${enc(id)}/memories`, { active: [], total: 0, pendingShare: 0 }
      ),

    getProjectDrift: (id: string) =>
      get<{ items: any[]; total: number; drifted: number; broken: number }>(
        `/api/projects/${enc(id)}/drift`, { items: [], total: 0, drifted: 0, broken: 0 }
      ),

    getProjectPatterns: (id: string) =>
      get<{ followed: any[]; antiPatterns: any[] }>(
        `/api/projects/${enc(id)}/patterns`, { followed: [], antiPatterns: [] }
      ),

    getProjectRecommendations: (id: string, status?: string) =>
      get<any[]>(
        `/api/projects/${enc(id)}/recommendations${status ? `?status=${status}` : ''}`, []
      ),

    getProjectSessions: (id: string, limit = 50) =>
      get<{ sessions: any[] }>(
        `/api/projects/${enc(id)}/sessions?limit=${limit}`, { sessions: [] }
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
      get<{ modules: any[]; calls: any[]; moduleCount: number; exportCount: number; callCount: number }>(
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
    scanFolder: (root: string, maxDepth = 3) =>
      post<{ ok: boolean; scanning: boolean }>(
        '/api/scan', { root, max_depth: maxDepth }, { ok: false, scanning: false },
      ),

    // ── MCP Tool Proxy ────────────────────────────────────────────────
    mcpListTools: () => get<{ tools: Array<{ name: string; description: string; params: string[] }> }>('/api/mcp/tools', { tools: [] }),

    mcpCallTool: (tool: string, params: Record<string, string>) =>
      post<Record<string, unknown>>('/api/mcp/call', { tool, params }, {}),

    // ── Marketplace ──────────────────────────────────────────────────
    marketplaceInstall: (target: string, marketplacePath: string, item?: string, scope?: string) =>
      post('/api/marketplace/install', { target, marketplacePath, item, scope }, { ok: false }),

    // ── Config (user preferences) ──────────────────────────────────────
    getConfig: () => get<Record<string, string>>('/api/config', {}),

    getConfigKey: (key: string) =>
      get<{ key: string; value: string | null }>(`/api/config/${enc(key)}`, { key, value: null }),

    setConfig: (config: Record<string, string>) =>
      put('/api/config', config),

    deleteConfig: (key: string) => del(`/api/config/${enc(key)}`),

    // ── Assistants ────────────────────────────────────────────────────────
    detectAssistants: () => get<import('./types').AssistantStatus[]>('/api/assistants/detect', []),

    detectAssistantFamilies: () => get<import('./types').AssistantFamily[]>('/api/assistants/families', []),

    configureAssistants: (assistants: string[]) =>
      post<import('./types').AssistantConfigureResult>('/api/assistants/configure', { acps: assistants }, { configured: [], skipped: [], errors: [] }),

    removeAssistants: (assistants: string[] = []) =>
      post<import('./types').AssistantRemoveResult>('/api/assistants/remove', { acps: assistants }, { assistants_removed: [], errors: [] }),

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

function enc(s: string): string {
  return encodeURIComponent(s);
}
