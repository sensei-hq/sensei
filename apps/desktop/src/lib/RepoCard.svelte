<script lang="ts">
  import type { SolutionRepo, ServerProject, GraphData, RepoRole, IndexProgress } from './types.js';
  import { senseiApi } from './api.js';
  import GraphCanvas from './GraphCanvas.svelte';

  let {
    repo,
    serverInfo,
    port,
    expanded = false,
    onToggle,
    onRoleChange,
  }: {
    repo: SolutionRepo;
    serverInfo?: ServerProject;
    port: number;
    expanded: boolean;
    onToggle: () => void;
    onRoleChange?: (role: RepoRole) => void;
  } = $props();

  type TabId = 'index' | 'graph' | 'deps';
  let activeTab = $state<TabId>('index');

  // Index state
  let indexing = $state(false);
  let indexError = $state<string | null>(null);
  let progress = $state<IndexProgress | null>(null);
  let indexLog = $state<string[]>([]);

  // Graph state (lazy loaded)
  let graphData = $state<GraphData | null>(null);
  let graphNodes = $state<Array<{ id: string; name: string; kind: string; file: string; line: number; complexity?: number }>>([]);
  let graphEdges = $state<Array<{ source: string; target: string; type: string }>>([]);
  let graphLoading = $state(false);
  let selectedGraphNode = $state<any>(null);

  const ROLE_CLS: Record<string, string> = {
    backend: 'bg-info-z2 text-info-z7',
    frontend: 'bg-primary-z2 text-primary-z7',
    mobile: 'bg-secondary-z2 text-secondary-z7',
    library: 'bg-warning-z2 text-warning-z7',
    infra: 'bg-surface-z3 text-surface-z6',
    docs: 'bg-surface-z3 text-surface-z5',
    shared: 'bg-accent-z2 text-accent-z7',
    middleware: 'bg-info-z2 text-info-z6',
    unknown: 'bg-surface-z3 text-surface-z5',
  };

  const ROLES: RepoRole[] = ['backend', 'frontend', 'mobile', 'middleware', 'infra', 'docs', 'library', 'shared', 'unknown'];

  let repoName = $derived(repo.label ?? repo.path.split('/').at(-1) ?? repo.repoId);
  let progressPct = $derived(progress && progress.filesTotal > 0
    ? Math.round((progress.filesProcessed / (progress.filesTotal - (progress.filesUnchanged ?? 0))) * 100)
    : 0);

  async function startIndex(force = false) {
    indexing = true;
    indexError = null;
    progress = null;
    indexLog = [];

    try {
      const api = senseiApi(port);
      await api.registerProject(repo.repoId, repoName, repo.path);

      // Queue indexing — progress comes via SSE, not streaming
      const res = await api.indexRepo(repo.repoId, repo.path, force);

      if (!res.ok) {
        indexError = 'Failed to queue index';
        indexing = false;
        return;
      }

      indexLog = [...indexLog, `Queued at position ${res.position}`];
    } catch (e) {
      indexError = String(e);
    } finally {
      indexing = false;
    }
  }

  async function pollProgress() {
    try {
      const health = await senseiApi(port).getHealth();
      const progressMap = (health as any).progress as Record<string, IndexProgress> | undefined;
      if (progressMap?.[repo.repoId]) {
        progress = progressMap[repo.repoId];
      }
    } catch { /* ignore */ }
  }

  // Poll progress while indexing is active
  let pollInterval: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (indexing && expanded && activeTab === 'index') {
      pollInterval = setInterval(pollProgress, 2000);
      return () => { if (pollInterval) clearInterval(pollInterval); };
    } else if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  });

  async function loadGraph() {
    if (graphData || graphLoading) return;
    graphLoading = true;
    try {
      const [data, nodesData] = await Promise.all([
        senseiApi(port).getGraph(repo.repoId, repo.path),
        fetch(`http://127.0.0.1:${port}/api/graph/nodes?repoId=${encodeURIComponent(repo.repoId)}`)
          .then(r => r.ok ? r.json() as Promise<{ nodes: typeof graphNodes; edges: typeof graphEdges }> : { nodes: [], edges: [] })
          .catch(() => ({ nodes: [] as typeof graphNodes, edges: [] as typeof graphEdges })),
      ]);
      graphData = data;
      graphNodes = nodesData.nodes;
      graphEdges = nodesData.edges;
    } catch { graphData = null; }
    finally { graphLoading = false; }
  }

  function onTabChange(tab: TabId) {
    activeTab = tab;
    if (tab === 'graph') loadGraph();
  }
</script>

<div class="rounded-lg border border-surface-z0/50 bg-surface-z2/50 overflow-hidden">
  <!-- Collapsed row -->
  <button
    onclick={onToggle}
    class="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-surface-z2"
  >
    <span class="rounded px-1.5 py-0.5 text-[10px] font-medium shrink-0 {ROLE_CLS[repo.role] ?? ROLE_CLS.unknown}">
      {repo.role}
    </span>
    <span class="text-sm text-surface-z7 flex-1 truncate font-medium">{repoName}</span>
    {#if indexing}
      <span class="i-solar-refresh-bold-duotone animate-spin text-xs text-primary-z5 shrink-0"></span>
    {:else if serverInfo?.indexedAt}
      <span class="h-1.5 w-1.5 rounded-full bg-success-z5 shrink-0"></span>
    {:else if serverInfo?.lastError}
      <span class="h-1.5 w-1.5 rounded-full bg-error-z5 shrink-0"></span>
    {:else}
      <span class="h-1.5 w-1.5 rounded-full bg-surface-z4 shrink-0"></span>
    {/if}
    <span class="i-solar-alt-arrow-{expanded ? 'up' : 'down'}-bold-duotone text-xs text-surface-z4"></span>
  </button>

  <!-- Expanded panel -->
  {#if expanded}
    <div class="border-t border-surface-z0/30">
      <!-- Tab bar -->
      <div class="flex items-center gap-1 px-3 py-1.5 bg-surface-z1/50">
        {#each [['index', 'Index'], ['graph', 'Graph'], ['deps', 'Dependencies']] as [id, label]}
          <button
            onclick={() => onTabChange(id as TabId)}
            class="rounded px-2.5 py-1 text-xs font-medium transition-colors
                   {activeTab === id ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z4 hover:text-surface-z6'}"
          >
            {label}
          </button>
        {/each}

        <!-- Role selector (right-aligned) -->
        <div class="ml-auto flex items-center gap-1.5">
          <span class="text-[10px] text-surface-z4">Role:</span>
          <select
            value={repo.role}
            onchange={(e) => onRoleChange?.((e.target as HTMLSelectElement).value as RepoRole)}
            class="rounded border border-surface-z3 bg-surface-z1 px-1.5 py-0.5 text-[10px] text-surface-z6"
          >
            {#each ROLES as r}
              <option value={r}>{r}</option>
            {/each}
          </select>
        </div>
      </div>

      <div class="px-3 py-3">
        <!-- Index tab -->
        {#if activeTab === 'index'}
          <div class="space-y-2">
            <div class="flex items-center gap-2 text-xs">
              <span class="text-surface-z4">Path:</span>
              <span class="font-mono text-surface-z6 truncate">{repo.path}</span>
            </div>
            {#if serverInfo?.indexedAt}
              <div class="flex items-center gap-2 text-xs">
                <span class="text-success-z6">Indexed</span>
                <span class="text-surface-z4">{new Date(serverInfo.indexedAt).toLocaleString()}</span>
              </div>
            {/if}

            <!-- Progress bar -->
            {#if indexing && progress}
              <div class="space-y-1">
                <div class="flex items-center justify-between text-[10px]">
                  <span class="text-surface-z5 truncate">{progress.currentFile}</span>
                  <span class="text-surface-z4 shrink-0">{progress.filesProcessed}/{progress.filesTotal - (progress.filesUnchanged ?? 0)}</span>
                </div>
                <div class="h-1.5 rounded-full bg-surface-z3 overflow-hidden">
                  <div class="h-full rounded-full bg-primary-z5 transition-all" style="width: {Math.min(progressPct, 100)}%"></div>
                </div>
              </div>
            {:else if indexing}
              <div class="flex items-center gap-2 text-xs text-primary-z5">
                <span class="i-solar-refresh-bold-duotone animate-spin text-xs"></span>
                Indexing…
              </div>
            {/if}

            <!-- Index log -->
            {#if indexLog.length > 0}
              <div class="max-h-32 overflow-y-auto rounded bg-surface-z1 px-2 py-1.5 text-[10px] font-mono text-surface-z5 space-y-0.5">
                {#each indexLog as line}
                  <div class="truncate">{line}</div>
                {/each}
              </div>
            {/if}

            {#if serverInfo?.lastError}
              <div class="rounded bg-error-z1 px-2 py-1.5 text-xs text-error-z6">
                {serverInfo.lastError}
              </div>
            {/if}
            {#if indexError}
              <div class="rounded bg-error-z1 px-2 py-1.5 text-xs text-error-z6">{indexError}</div>
            {/if}
            <div class="flex gap-2">
              <button
                onclick={() => startIndex(false)}
                disabled={indexing}
                class="rounded-md bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3 transition-colors disabled:opacity-50"
              >
                {indexing ? 'Indexing…' : serverInfo?.indexedAt ? 'Re-index' : 'Index'}
              </button>
              {#if serverInfo?.lastError || serverInfo?.partiallyIndexed}
                <button
                  onclick={() => startIndex(true)}
                  disabled={indexing}
                  class="rounded-md bg-warning-z2 px-3 py-1.5 text-xs font-medium text-warning-z7 hover:bg-warning-z3 transition-colors disabled:opacity-50"
                >
                  Force retry
                </button>
              {/if}
            </div>
          </div>

        <!-- Graph tab -->
        {:else if activeTab === 'graph'}
          <div class="space-y-2">
            {#if graphLoading}
              <p class="text-xs text-surface-z4">Loading graph…</p>
            {:else if graphNodes.length > 0}
              <!-- Interactive D3 graph -->
              <div class="h-64 rounded-lg border border-surface-z0/30">
                <GraphCanvas
                  nodes={graphNodes}
                  edges={graphEdges}
                  onSelectNode={(n) => { selectedGraphNode = n; }}
                />
              </div>

              <!-- Stats bar -->
              <div class="flex gap-4 text-xs text-surface-z4">
                <span>{graphNodes.length} nodes</span>
                <span>{graphEdges.length} edges</span>
                {#if graphData}
                  <span>{graphData.summary.communities} communities</span>
                {/if}
                {#if selectedGraphNode}
                  <span class="text-primary-z6">
                    {selectedGraphNode.name} ({selectedGraphNode.kind}) — {selectedGraphNode.file}:{selectedGraphNode.line}
                  </span>
                {/if}
              </div>

              <!-- God nodes + communities (collapsed) -->
              {#if graphData?.godNodes?.length}
                <details class="text-xs">
                  <summary class="text-[10px] font-semibold text-surface-z5 uppercase tracking-wide cursor-pointer">
                    God Nodes ({graphData.godNodes.length})
                  </summary>
                  <div class="mt-1 space-y-0.5 pl-2">
                    {#each graphData.godNodes.slice(0, 8) as node}
                      <div class="flex items-center gap-2">
                        <span class="h-1.5 w-1.5 rounded-full bg-error-z5 shrink-0"></span>
                        <span class="font-mono text-surface-z7">{node.name}</span>
                        <span class="text-surface-z4 ml-auto">degree {node.degree}</span>
                      </div>
                    {/each}
                  </div>
                </details>
              {/if}
            {:else if graphData}
              <!-- Fallback: text only (no node data) -->
              <div class="flex gap-4 text-xs text-surface-z4">
                <span>{graphData.summary.totalSymbols} symbols</span>
                <span>{graphData.summary.totalEdges} edges</span>
              </div>
            {:else}
              <p class="text-xs text-surface-z4">No graph data. Index this repo first.</p>
            {/if}
          </div>

        <!-- Dependencies tab -->
        {:else if activeTab === 'deps'}
          {#await senseiApi(port).getDepVersions(repo.repoId)}
            <p class="text-xs text-surface-z4">Loading dependencies...</p>
          {:then deps}
            {#if deps.length === 0}
              <p class="text-xs text-surface-z4">No dependency manifest found.</p>
            {:else}
              <div class="space-y-0.5 max-h-48 overflow-y-auto">
                {#each deps as dep}
                  <div class="flex items-center gap-2 text-xs px-1 py-0.5 rounded hover:bg-surface-z3/50">
                    <span class="flex-1 text-surface-z7 font-mono truncate">{dep.lib_name}</span>
                    <span class="text-surface-z5 font-mono">{dep.version}</span>
                    <span class="text-[10px] text-surface-z4">{dep.source}</span>
                    {#if dep.dev}
                      <span class="text-[10px] text-surface-z3">dev</span>
                    {/if}
                  </div>
                {/each}
              </div>
              <p class="text-[10px] text-surface-z3 mt-1">{deps.length} dependencies</p>
            {/if}
          {/await}
        {/if}
      </div>
    </div>
  {/if}
</div>
