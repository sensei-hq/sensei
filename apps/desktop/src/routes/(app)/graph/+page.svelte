<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  type ViewMode = 'communities' | 'godNodes' | 'rationale';
  let viewMode = $state<ViewMode>('communities');
  let projectFilter = $state('all');
  let selectedNode = $state<string | null>(null);

  const projectColors: Record<string, string> = {
    sensei: 'bg-primary-z5',
    rokkit: 'bg-secondary-z5',
    kavach: 'bg-success-z5',
    dbd:    'bg-warning-z5',
  };

  const tagBg: Record<string, string> = {
    WHY:      'bg-info-z2 text-info-z7',
    DECISION: 'bg-primary-z2 text-primary-z7',
    NOTE:     'bg-surface-z3 text-surface-z6',
    HACK:     'bg-danger-z2 text-danger-z7',
  };

  let filteredCommunities = $derived(
    data.communities.filter((c: { project: string }) =>
      projectFilter === 'all' || c.project === projectFilter
    )
  );

  let filteredGodNodes = $derived(
    data.godNodes.filter((n: { project: string }) =>
      projectFilter === 'all' || n.project === projectFilter
    )
  );

  let filteredRationale = $derived(
    data.rationale.filter((r: { project: string }) =>
      projectFilter === 'all' || r.project === projectFilter
    )
  );

  let allProjects = $derived([...new Set(data.communities.map((c: { project: string }) => c.project))]);
</script>

<div class="flex h-full flex-col">

  <!-- Top bar -->
  <div class="flex items-center justify-between border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <div class="flex items-center gap-3">
      <h1 class="text-sm font-semibold text-surface-z8">Graph Intelligence</h1>
      <div class="flex items-center gap-2 text-xs text-surface-z4">
        <span>{data.summary.totalSymbols.toLocaleString()} symbols</span>
        <span>·</span>
        <span>{data.summary.totalEdges.toLocaleString()} edges</span>
        <span>·</span>
        <span>{data.summary.communities} communities</span>
      </div>
    </div>
    <div class="flex items-center gap-1.5">
      <!-- View switcher -->
      <div class="flex items-center rounded-lg border border-surface-z3 bg-surface-z2 p-0.5">
        {#each (['communities', 'godNodes', 'rationale'] as const) as v}
          <button
            onclick={() => viewMode = v}
            class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors capitalize
                   {viewMode === v ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
          >{v === 'godNodes' ? 'God nodes' : v}</button>
        {/each}
      </div>
    </div>
  </div>

  <!-- Project filter -->
  <div class="flex items-center gap-1 border-b border-surface-z0/50 px-4 py-1.5 shrink-0">
    <button
      onclick={() => projectFilter = 'all'}
      class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors {projectFilter === 'all' ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
    >All</button>
    {#each allProjects as proj}
      <button
        onclick={() => projectFilter = proj}
        class="flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition-colors
               {projectFilter === proj ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
      >
        <span class="h-1.5 w-1.5 rounded-full {projectColors[proj] ?? 'bg-surface-z4'}"></span>
        {proj}
      </button>
    {/each}
  </div>

  <!-- Content -->
  <div class="flex flex-1 min-h-0 overflow-hidden">

    {#if viewMode === 'communities'}
      <div class="flex-1 overflow-y-auto px-5 py-4">
        <div class="grid grid-cols-2 gap-3 xl:grid-cols-3">
          {#each filteredCommunities as c (c.id)}
            <div class="rounded-2xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-4 transition-all hover:border-surface-z4 hover:bg-surface-z2">
              <div class="flex items-center gap-2.5 mb-3">
                <div class="h-3 w-3 rounded-full shrink-0 {c.color}"></div>
                <span class="font-semibold text-surface-z8">{c.label}</span>
                <span class="ml-auto text-[10px] text-surface-z4 rounded bg-surface-z3 px-1.5 py-0.5">{c.project}</span>
              </div>
              <p class="text-xs text-surface-z5 mb-3">{c.symbolCount} symbols</p>
              <div>
                <p class="text-[9px] font-semibold uppercase tracking-wide text-surface-z4 mb-1.5">God nodes</p>
                <div class="flex flex-wrap gap-1">
                  {#each c.godNodes as node}
                    <button
                      onclick={() => { selectedNode = node; viewMode = 'godNodes'; }}
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
        <!-- Node list -->
        <div class="flex w-64 shrink-0 flex-col border-r border-surface-z0/50 overflow-hidden">
          <div class="flex-1 overflow-y-auto px-2 py-2 space-y-1">
            {#each filteredGodNodes as node (node.name)}
              <button
                onclick={() => selectedNode = node.name}
                class="w-full rounded-xl border px-3 py-2.5 text-left transition-all
                       {selectedNode === node.name
                         ? 'border-primary-z4 bg-primary-z1'
                         : 'border-surface-z3/50 bg-surface-z2/40 hover:border-surface-z3 hover:bg-surface-z2'}"
              >
                <div class="flex items-center gap-2 mb-1">
                  <span class="i-solar-star-bold-duotone text-sm text-warning-z6 shrink-0"></span>
                  <span class="font-mono text-sm font-medium text-surface-z8 truncate">{node.name}</span>
                </div>
                <div class="flex items-center gap-2 text-[10px] text-surface-z4">
                  <span class="h-1.5 w-1.5 rounded-full {projectColors[node.project] ?? 'bg-surface-z4'}"></span>
                  <span>{node.project}</span>
                  <span class="ml-auto">deg {node.degree}</span>
                </div>
              </button>
            {/each}
          </div>
        </div>

        <!-- Node detail -->
        {#if selectedNode}
          {@const node = filteredGodNodes.find((n: { name: string }) => n.name === selectedNode)}
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
                  <p class="text-[10px] text-surface-z4">connections</p>
                </div>
                <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
                  <p class="text-xs text-surface-z4">Community</p>
                  <p class="mt-1 text-sm font-semibold text-surface-z8">{node.community}</p>
                </div>
                <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
                  <p class="text-xs text-surface-z4">Project</p>
                  <p class="mt-1 text-sm font-semibold text-surface-z8">{node.project}</p>
                </div>
              </div>

              <p class="text-xs text-surface-z5">
                This symbol has the highest connection degree in its community, making it a load-bearing
                node in the codebase graph. Changes here cascade widely — treat with care.
              </p>
            </div>
          {/if}
        {:else}
          <div class="flex flex-1 items-center justify-center">
            <div class="text-center">
              <span class="i-solar-star-bold-duotone text-3xl text-surface-z3 block mx-auto mb-2"></span>
              <p class="text-sm text-surface-z5">Select a god node</p>
              <p class="mt-1 text-xs text-surface-z4">The highest-degree symbols in each community</p>
            </div>
          </div>
        {/if}
      </div>

    {:else}
      <!-- Rationale view -->
      <div class="flex-1 overflow-y-auto px-5 py-4">
        <div class="space-y-3">
          {#each filteredRationale as r (r.file)}
            <div class="rounded-2xl border border-surface-z3/60 bg-surface-z2/50 px-4 py-4">
              <div class="flex items-center gap-2 mb-2">
                <span class="rounded-md px-2 py-0.5 text-[10px] font-bold {tagBg[r.tag] ?? 'bg-surface-z2 text-surface-z5'}">{r.tag}</span>
                <span class="h-1.5 w-1.5 rounded-full {projectColors[r.project] ?? 'bg-surface-z4'}"></span>
                <span class="text-[11px] text-surface-z4">{r.project}</span>
                <span class="ml-auto font-mono text-[10px] text-surface-z4">{r.file.split('/').pop()}</span>
              </div>
              <p class="text-sm leading-relaxed text-surface-z7">{r.text}</p>
              <p class="mt-1.5 font-mono text-[10px] text-surface-z3">{r.file}</p>
            </div>
          {/each}
        </div>
      </div>
    {/if}

  </div>
</div>
