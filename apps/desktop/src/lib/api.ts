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

    // ── Projects ─────────────────────────────────────────────────────────
    getProjects: async () => {
      const projects = await get<ServerProject[]>('/api/projects', []);
      return projects.map(p => ({
        ...p,
        repoId: p.repo_id,
        indexedAt: p.indexed_at,
        lastError: p.last_error,
        partiallyIndexed: false,
      }));
    },

    getProjectSummary: (repoId: string) =>
      get<ProjectSummary>(`/api/projects/${enc(repoId)}/summary`, null as any),

    registerProject: (repoId: string, name: string, path: string) =>
      post('/api/projects', { repoId, name, path }, { ok: false }),

    updateProject: (repoId: string, patch: Partial<ServerProject>) =>
      put(`/api/projects/${enc(repoId)}`, patch),

    deleteProject: (repoId: string) => del(`/api/projects/${enc(repoId)}`),

    addProjectTag: (repoId: string, tag: string) =>
      post(`/api/projects/${enc(repoId)}/tags`, { tag }, { ok: false }),

    removeProjectTag: (repoId: string, tag: string) =>
      del(`/api/projects/${enc(repoId)}/tags/${enc(tag)}`),

    // ── Solutions ────────────────────────────────────────────────────────
    listSolutions: () => get<any[]>('/api/solutions', []),

    createSolution: (solution: any) =>
      post<{ ok: boolean; id?: string }>('/api/solutions', solution, { ok: false }),

    updateSolution: (id: string, patch: any) =>
      put(`/api/solutions/${enc(id)}`, patch),

    deleteSolution: (id: string) => del(`/api/solutions/${enc(id)}`),

    addSolutionRepo: (solutionId: string, repo: { repoId: string; role?: string }) =>
      post(`/api/solutions/${enc(solutionId)}/repos`, repo, { ok: false }),

    removeSolutionRepo: (solutionId: string, repoId: string) =>
      del(`/api/solutions/${enc(solutionId)}/repos/${enc(repoId)}`),

    getSolutionGraph: (id: string) =>
      get<SolutionGraphResponse>(`/api/solutions/${enc(id)}/graph`, null as any),

    getSolutionRoles: (id: string) =>
      get<InferredRole[]>(`/api/solutions/${enc(id)}/roles`, []),

    analyzeSolution: (id: string) =>
      post<SolutionAnalysis>(`/api/solutions/${enc(id)}/analyze`, {}, null as any),

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
    getLibs: (params?: { repoId?: string; solutionId?: string; shared?: boolean }) => {
      const qs = new URLSearchParams();
      if (params?.repoId) qs.set('repoId', params.repoId);
      if (params?.solutionId) qs.set('solutionId', params.solutionId);
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
    query: (q: string, repoId?: string, solutionId?: string) =>
      post<Record<string, unknown>>('/api/query', { q, repoId, solutionId }, {}),

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

    // ── ACP (AI Coding Platform) ────────────────────────────────────────
    detectAcps: () => get<import('./types').AcpStatus[]>('/api/acp/detect', []),

    configureAcps: (acps: string[]) =>
      post<import('./types').AcpConfigureResult>('/api/acp/configure', { acps }, { configured: [], skipped: [], errors: [] }),

    unconfigureAcps: () =>
      post<{ ok: boolean; removed: string[] }>('/api/acp/unconfigure', {}, { ok: false, removed: [] }),

    // ── Installer ───────────────────────────────────────────────────────
    installAll: (acps: string[], scope = 'global') =>
      post<import('./types').InstallResult>('/api/install', { acps, scope }, {
        hooks_installed: 0, skills_installed: 0, commands_installed: 0,
        acps_configured: [], errors: [], marketplace_version: '',
      }),

    installHooks: () =>
      post<{ ok: boolean; count: number }>('/api/install/hooks', {}, { ok: false, count: 0 }),

    installItem: (name: string, kind: string) =>
      post<{ ok: boolean; path?: string; error?: string }>('/api/install/item', { name, kind }, { ok: false }),

    uninstallItem: (name: string, kind: string) =>
      post<{ ok: boolean }>('/api/install/item/remove', { name, kind }, { ok: false }),

    getCatalog: () =>
      get<import('./types').MarketplaceCatalog>('/api/install/catalog', { version: null, items: [] }),

    getInstalledItems: () =>
      get<import('./types').InstalledItem[]>('/api/install/installed', []),

    uninstallAll: () =>
      post<import('./types').UninstallResult>('/api/uninstall', {}, {
        acps_removed: [], hooks_removed: false, skills_removed: 0,
        plugin_removed: false, cache_cleared: false,
      }),

    // ── Lifecycle ────────────────────────────────────────────────────────
    stop: () => post('/stop', {}, {}),
  };
}

function enc(s: string): string {
  return encodeURIComponent(s);
}
