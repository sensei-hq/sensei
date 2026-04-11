<script lang="ts">
  import type { SolutionRepo, ServerProject, GraphData, RepoRole } from './types.js';
  import { senseiApi } from './api.js';

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

  // Graph state (lazy loaded)
  let graphData = $state<GraphData | null>(null);
  let graphLoading = $state(false);

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

  async function startIndex(force = false) {
    indexing = true;
    indexError = null;
    try {
      const api = senseiApi(port);
      await api.registerProject(repo.repoId, repoName, repo.path);
      const res = await api.indexRepo(repo.repoId, repo.path, force);
      if (!res.ok) indexError = `Server returned ${res.status}`;
    } catch (e) {
      indexError = String(e);
    } finally {
      indexing = false;
    }
  }

  async function loadGraph() {
    if (graphData || graphLoading) return;
    graphLoading = true;
    try {
      graphData = await senseiApi(port).getGraph(repo.repoId, repo.path);
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
    {#if serverInfo?.indexedAt}
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
              <p class="text-xs text-surface-z4">Loading graph data…</p>
            {:else if graphData}
              <div class="flex gap-4 text-xs">
                <span class="text-surface-z4">{graphData.summary.totalSymbols} symbols</span>
                <span class="text-surface-z4">{graphData.summary.totalEdges} edges</span>
                <span class="text-surface-z4">{graphData.summary.communities} communities</span>
              </div>

              {#if graphData.godNodes.length > 0}
                <div>
                  <p class="text-[10px] font-semibold text-surface-z5 uppercase tracking-wide mb-1">God Nodes (high coupling)</p>
                  <div class="space-y-1">
                    {#each graphData.godNodes.slice(0, 5) as node}
                      <div class="flex items-center gap-2 text-xs">
                        <span class="h-2 w-2 rounded-full bg-error-z5 shrink-0"></span>
                        <span class="font-mono text-surface-z7 truncate">{node.name}</span>
                        <span class="text-surface-z4 ml-auto">degree {node.degree}</span>
                        <span class="text-surface-z3 truncate max-w-32">{node.file}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if graphData.communities.length > 0}
                <div>
                  <p class="text-[10px] font-semibold text-surface-z5 uppercase tracking-wide mb-1">Communities</p>
                  <div class="space-y-1">
                    {#each graphData.communities.slice(0, 8) as community}
                      <div class="flex items-center gap-2 text-xs">
                        <span class="rounded px-1.5 py-0.5 text-[10px] {community.color}">{community.symbolCount}</span>
                        <span class="text-surface-z6 truncate">{community.label}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              {#if graphData.rationale.length > 0}
                <div>
                  <p class="text-[10px] font-semibold text-surface-z5 uppercase tracking-wide mb-1">Rationale Comments</p>
                  <div class="space-y-1">
                    {#each graphData.rationale.slice(0, 5) as r}
                      <div class="flex items-start gap-2 text-xs">
                        <span class="rounded px-1 py-0.5 text-[10px] font-medium shrink-0
                          {r.tag === 'WHY' ? 'bg-info-z2 text-info-z6' :
                           r.tag === 'DECISION' ? 'bg-primary-z2 text-primary-z6' :
                           r.tag === 'HACK' ? 'bg-warning-z2 text-warning-z6' :
                           'bg-surface-z3 text-surface-z5'}">{r.tag}</span>
                        <span class="text-surface-z6 truncate">{r.text}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            {:else}
              <p class="text-xs text-surface-z4">No graph data. Index this repo first.</p>
            {/if}
          </div>

        <!-- Dependencies tab -->
        {:else if activeTab === 'deps'}
          <p class="text-xs text-surface-z4">Dependency detection coming in Phase 2.</p>
        {/if}
      </div>
    </div>
  {/if}
</div>
