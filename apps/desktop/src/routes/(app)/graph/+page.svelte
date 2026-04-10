<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  type Project = typeof data.projects[0];
  type ViewMode = 'communities' | 'godNodes' | 'rationale';

  const SENSEI_API = `http://localhost:${typeof localStorage !== 'undefined' ? (localStorage.getItem('sensei:port') ?? '7744') : '7744'}`;

  let selectedProject = $state<Project | null>(null);
  let viewMode = $state<ViewMode>('communities');
  let selectedNode = $state<string | null>(null);
  let loading = $state(false);
  let graphData = $state<{ summary: typeof data.summary; communities: unknown[]; godNodes: unknown[]; rationale: unknown[] } | null>(null);

  const tagBg: Record<string, string> = {
    WHY:      'bg-info-z2 text-info-z7',
    DECISION: 'bg-primary-z2 text-primary-z7',
    NOTE:     'bg-surface-z3 text-surface-z6',
    HACK:     'bg-warning-z2 text-warning-z7',
  };

  async function loadGraph(p: Project) {
    selectedProject = p;
    selectedNode = null;
    viewMode = 'communities';
    loading = true;
    graphData = null;
    try {
      const res = await fetch(`${SENSEI_API}/api/graph?repoId=${encodeURIComponent(p.repoId)}&repoPath=${encodeURIComponent(p.path)}`);
      if (res.ok) graphData = await res.json();
    } catch { /* server offline */ }
    loading = false;
  }

  const communities = $derived((graphData?.communities ?? []) as { id: string; label: string; color: string; project: string; symbolCount: number; godNodes: string[] }[]);
  const godNodes    = $derived((graphData?.godNodes    ?? []) as { name: string; file: string; degree: number; community: string; project: string }[]);
  const rationale   = $derived((graphData?.rationale   ?? []) as { tag: string; text: string; file: string; project: string }[]);
  const summary     = $derived(graphData?.summary ?? data.summary);
</script>

<div class="flex h-full min-h-0">

  <!-- ── Project list ── -->
  <div class="flex w-56 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
    <div class="flex items-center justify-between border-b border-surface-z0/50 px-3 py-2 shrink-0">
      <h1 class="text-sm font-semibold text-surface-z8">Graph</h1>
      <span class="text-xs text-surface-z4">{data.projects.length}</span>
    </div>
    <div class="flex-1 overflow-y-auto py-1">
      {#if data.projects.length === 0}
        <div class="flex flex-col items-center justify-center h-full gap-2 px-4 py-10 text-center">
          <span class="i-solar-graph-up-bold-duotone text-2xl text-surface-z3"></span>
          <p class="text-xs text-surface-z4">No indexed projects.<br>Import repos and start the indexer.</p>
        </div>
      {:else}
        {#each data.projects as p}
          <button
            onclick={() => loadGraph(p)}
            class="flex w-full items-start gap-2.5 px-3 py-2.5 text-left transition-colors hover:bg-surface-z2
                   {selectedProject?.repoId === p.repoId ? 'bg-primary-z1 border-r-2 border-primary-z5' : ''}"
          >
            <span class="i-solar-graph-up-bold-duotone text-sm shrink-0 mt-0.5 {p.indexedAt ? 'text-primary-z5' : 'text-surface-z4'}"></span>
            <div class="min-w-0 flex-1">
              <p class="text-xs font-semibold text-surface-z8 truncate">{p.name}</p>
              <p class="text-[10px] text-surface-z4 mt-0.5">{p.indexedAt ? 'Indexed' : 'Not indexed'}</p>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <!-- ── Graph panel ── -->
  <div class="flex flex-1 min-w-0 flex-col overflow-hidden">

    {#if !selectedProject}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center text-surface-z4">
        <span class="i-solar-graph-up-bold-duotone text-3xl text-surface-z3"></span>
        <div>
          <p class="text-sm font-medium text-surface-z6">Select a project</p>
          <p class="text-xs mt-1">View communities, god nodes and rationale</p>
        </div>
      </div>

    {:else if loading}
      <div class="flex flex-1 items-center justify-center gap-3 text-surface-z4">
        <span class="i-solar-refresh-bold-duotone animate-spin text-xl text-primary-z5"></span>
        <span class="text-sm">Loading graph…</span>
      </div>

    {:else if !graphData || summary.totalSymbols === 0}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center text-surface-z4">
        <span class="i-solar-graph-up-bold-duotone text-3xl text-surface-z3"></span>
        <div>
          <p class="text-sm font-medium text-surface-z6">{selectedProject.name}</p>
          <p class="text-xs mt-1">{selectedProject.indexedAt ? 'No graph data yet — indexing may still be running.' : 'Not indexed yet. Start the indexer to index this project.'}</p>
        </div>
      </div>

    {:else}
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-surface-z0/50 px-4 py-2 shrink-0">
        <div class="flex items-center gap-2 text-xs text-surface-z4">
          <span class="font-semibold text-surface-z7">{selectedProject.name}</span>
          <span>·</span>
          <span>{summary.totalSymbols.toLocaleString()} symbols</span>
          <span>·</span>
          <span>{summary.totalEdges.toLocaleString()} edges</span>
          <span>·</span>
          <span>{summary.communities} communities</span>
        </div>
        <div class="flex items-center rounded-lg border border-surface-z3 bg-surface-z2 p-0.5">
          {#each (['communities', 'godNodes', 'rationale'] as const) as v}
            <button
              onclick={() => viewMode = v}
              class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors
                     {viewMode === v ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
            >{v === 'godNodes' ? 'God nodes' : v}</button>
          {/each}
        </div>
      </div>

      <!-- Content -->
      <div class="flex flex-1 min-h-0 overflow-hidden">
        {#if viewMode === 'communities'}
          <div class="flex-1 overflow-y-auto px-5 py-4">
            <div class="grid grid-cols-2 gap-3 xl:grid-cols-3">
              {#each communities as c (c.id)}
                <div class="rounded-2xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-4 transition-all hover:border-surface-z4 hover:bg-surface-z2">
                  <div class="flex items-center gap-2.5 mb-3">
                    <div class="h-3 w-3 rounded-full shrink-0 {c.color}"></div>
                    <span class="font-semibold text-surface-z8">{c.label}</span>
                    <span class="ml-auto text-[10px] text-surface-z4 rounded bg-surface-z3 px-1.5 py-0.5">{c.symbolCount} sym</span>
                  </div>
                  <div>
                    <p class="text-[9px] font-semibold uppercase tracking-wide text-surface-z4 mb-1.5">God nodes</p>
                    <div class="flex flex-wrap gap-1">
                      {#each c.godNodes as node}
                        <button
                          onclick={() => { selectedNode = node as string; viewMode = 'godNodes'; }}
                          class="rounded-md bg-surface-z3 px-2 py-0.5 font-mono text-[10px] text-surface-z6 transition-colors hover:bg-primary-z2 hover:text-primary-z7"
                        >{node}</button>
                      {/each}
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>

        {:else if viewMode === 'godNodes'}
          <div class="flex flex-1 min-h-0 overflow-hidden">
            <div class="flex w-64 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
              <div class="flex-1 overflow-y-auto px-2 py-2 space-y-1">
                {#each godNodes as node (node.name)}
                  <button
                    onclick={() => selectedNode = node.name}
                    class="w-full rounded-xl border px-3 py-2.5 text-left transition-all
                           {selectedNode === node.name ? 'border-primary-z4 bg-primary-z1' : 'border-surface-z3/50 bg-surface-z2/40 hover:border-surface-z3 hover:bg-surface-z2'}"
                  >
                    <div class="flex items-center gap-2 mb-1">
                      <span class="i-solar-star-bold-duotone text-sm text-warning-z6 shrink-0"></span>
                      <span class="font-mono text-sm font-medium text-surface-z8 truncate">{node.name}</span>
                    </div>
                    <div class="flex items-center gap-2 text-[10px] text-surface-z4">
                      <span class="ml-auto">deg {node.degree}</span>
                    </div>
                  </button>
                {/each}
              </div>
            </div>
            {#if selectedNode}
              {@const node = godNodes.find(n => n.name === selectedNode)}
              {#if node}
                <div class="flex-1 overflow-y-auto px-5 py-5">
                  <div class="flex items-center gap-3 mb-2">
                    <span class="i-solar-star-bold-duotone text-2xl text-warning-z6"></span>
                    <h2 class="text-lg font-semibold font-mono text-surface-z9">{node.name}</h2>
                  </div>
                  <p class="font-mono text-xs text-surface-z4 mb-5">{node.file}</p>
                  <div class="grid grid-cols-3 gap-3 mb-6">
                    <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
                      <p class="text-xs text-surface-z4">Degree</p>
                      <p class="mt-1 text-2xl font-semibold text-surface-z8">{node.degree}</p>
                    </div>
                    <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
                      <p class="text-xs text-surface-z4">Community</p>
                      <p class="mt-1 text-sm font-semibold text-surface-z8">{node.community}</p>
                    </div>
                  </div>
                  <p class="text-xs text-surface-z5">Load-bearing node — changes here cascade widely.</p>
                </div>
              {/if}
            {:else}
              <div class="flex flex-1 items-center justify-center text-center">
                <div>
                  <span class="i-solar-star-bold-duotone text-3xl text-surface-z3 block mx-auto mb-2"></span>
                  <p class="text-sm text-surface-z5">Select a god node</p>
                </div>
              </div>
            {/if}
          </div>

        {:else}
          <div class="flex-1 overflow-y-auto px-5 py-4 space-y-3">
            {#each rationale as r (r.file)}
              <div class="rounded-2xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-4">
                <div class="flex items-center gap-2 mb-2">
                  <span class="rounded-md px-2 py-0.5 text-[10px] font-bold {tagBg[r.tag] ?? 'bg-surface-z2 text-surface-z5'}">{r.tag}</span>
                  <span class="ml-auto font-mono text-[10px] text-surface-z4">{r.file.split('/').pop()}</span>
                </div>
                <p class="text-sm leading-relaxed text-surface-z7">{r.text}</p>
                <p class="mt-1.5 font-mono text-[10px] text-surface-z3">{r.file}</p>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
