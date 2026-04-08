<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  type ViewMode = 'list' | 'stats';
  let viewMode = $state<ViewMode>('list');
  let projectFilter = $state('all');
  let statusFilter = $state<'all' | 'completed' | 'in-progress'>('all');

  let projects = $derived([...new Set(data.sessions.map((s: { project: string }) => s.project))]);

  // FTR stats — computed for stats view
  let ftrFirstTry = $derived(data.sessions.filter((s: { ftr: number | null }) => s.ftr === 1.0).length);
  let ftrPartial  = $derived(data.sessions.filter((s: { ftr: number | null }) => s.ftr !== null && (s.ftr as number) < 1.0 && (s.ftr as number) > 0).length);
  let ftrPending  = $derived(data.sessions.filter((s: { ftr: number | null }) => s.ftr === null).length);
  let ftrTotal    = $derived(data.sessions.length);

  let filtered = $derived(
    data.sessions.filter((s: { project: string; status: string }) => {
      if (projectFilter !== 'all' && s.project !== projectFilter) return false;
      if (statusFilter !== 'all' && s.status !== statusFilter) return false;
      return true;
    })
  );

  function ftrBadge(ftr: number | null): string {
    if (ftr === null) return 'bg-surface-z2 text-surface-z5';
    if (ftr >= 1)    return 'bg-success-z2 text-success-z7';
    if (ftr >= 0.5)  return 'bg-warning-z2 text-warning-z7';
    return 'bg-danger-z2 text-danger-z7';
  }
  function ftrLabel(ftr: number | null): string {
    if (ftr === null) return '—';
    if (ftr >= 1)    return 'First try';
    if (ftr >= 0.5)  return 'Partial';
    return 'Retry';
  }
</script>

<div class="flex h-full flex-col">

  <!-- Top bar -->
  <div class="flex items-center justify-between border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z8">Sessions</h1>
    <div class="flex items-center gap-1.5">
      <div class="flex items-center rounded-lg border border-surface-z3 bg-surface-z2 p-0.5">
        <button onclick={() => viewMode = 'list'} title="List" class="rounded-md p-1.5 transition-colors {viewMode === 'list' ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}">
          <span class="i-solar-list-bold-duotone text-sm"></span>
        </button>
        <button onclick={() => viewMode = 'stats'} title="Stats" class="rounded-md p-1.5 transition-colors {viewMode === 'stats' ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}">
          <span class="i-solar-chart-2-bold-duotone text-sm"></span>
        </button>
      </div>
    </div>
  </div>

  {#if viewMode === 'list'}
    <!-- Filters -->
    <div class="flex items-center gap-2 border-b border-surface-z0/50 px-4 py-2 shrink-0">
      <select
        bind:value={projectFilter}
        class="rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 py-1 text-xs text-surface-z7 outline-none"
      >
        <option value="all">All projects</option>
        {#each projects as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
      <div class="flex gap-0.5">
        {#each (['all', 'completed', 'in-progress'] as const) as f}
          <button
            onclick={() => statusFilter = f}
            class="rounded-md px-2 py-1 text-[10px] font-medium transition-colors capitalize
                   {statusFilter === f ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z5 hover:text-surface-z7'}"
          >{f === 'all' ? 'All' : f}</button>
        {/each}
      </div>
      <span class="ml-auto text-xs text-surface-z4">{filtered.length} sessions</span>
    </div>

    <!-- Table -->
    <div class="flex-1 overflow-y-auto">
      <table class="w-full text-sm">
        <thead class="sticky top-0 bg-surface-z1">
          <tr class="border-b border-surface-z3">
            <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Task</th>
            <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Project</th>
            <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Status</th>
            <th class="px-4 py-2.5 text-left text-[10px] font-semibold uppercase tracking-wide text-surface-z4">FTR</th>
            <th class="px-4 py-2.5 text-right text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Turns</th>
            <th class="px-4 py-2.5 text-right text-[10px] font-semibold uppercase tracking-wide text-surface-z4">Cost</th>
            <th class="px-4 py-2.5 text-right text-[10px] font-semibold uppercase tracking-wide text-surface-z4">When</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as s (s.id)}
            <tr class="border-b border-surface-z3/50 transition-colors hover:bg-surface-z2/50">
              <td class="max-w-xs px-4 py-3">
                <span class="block truncate font-medium text-surface-z8">{s.task}</span>
              </td>
              <td class="px-4 py-3 text-xs text-surface-z5">{s.project}</td>
              <td class="px-4 py-3">
                <span class="inline-flex items-center gap-1 text-xs">
                  <span class="h-1.5 w-1.5 rounded-full {s.status === 'completed' ? 'bg-success-z5' : 'bg-primary-z6 animate-pulse'}"></span>
                  {s.status}
                </span>
              </td>
              <td class="px-4 py-3">
                <span class="rounded-full px-2 py-0.5 text-[10px] font-semibold {ftrBadge(s.ftr)}">{ftrLabel(s.ftr)}</span>
              </td>
              <td class="px-4 py-3 text-right text-xs text-surface-z5">{s.turns}</td>
              <td class="px-4 py-3 text-right text-xs text-surface-z5">${s.cost.toFixed(2)}</td>
              <td class="px-4 py-3 text-right text-xs text-surface-z4">{s.when}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

  {:else}
    <!-- Stats view -->
    <div class="flex-1 overflow-y-auto px-5 py-4">

      <!-- Summary cards -->
      <div class="grid grid-cols-2 gap-3 mb-6 xl:grid-cols-4">
        {#each [
          { label: 'Total sessions',     value: data.stats.totalSessions,                              sub: `${data.stats.completedSessions} completed` },
          { label: 'Avg FTR score',      value: `${Math.round(data.stats.avgFtr * 100)}%`,             sub: 'First-try-right rate' },
          { label: 'Total cost',         value: `$${data.stats.totalCost.toFixed(2)}`,                 sub: 'Claude Code tokens' },
          { label: 'Total tool calls',   value: data.stats.totalTurns.toLocaleString(),                sub: `${(data.stats.totalTokensIn / 1_000_000).toFixed(1)}M tokens in` },
        ] as stat}
          <div class="rounded-2xl border border-surface-z3 bg-surface-z2 px-5 py-4">
            <p class="text-xs text-surface-z4">{stat.label}</p>
            <p class="mt-1 text-2xl font-semibold text-surface-z9">{stat.value}</p>
            <p class="mt-0.5 text-[11px] text-surface-z4">{stat.sub}</p>
          </div>
        {/each}
      </div>

      <!-- FTR breakdown -->
      <div class="mb-5">
        <p class="mb-3 text-xs font-semibold uppercase tracking-wide text-surface-z4">FTR breakdown</p>
        <div class="rounded-2xl border border-surface-z3 bg-surface-z2 px-5 py-4">
          <div class="flex gap-0.5 h-4 rounded-full overflow-hidden mb-3">
            <div class="bg-success-z5 transition-all" style="width: {Math.round(ftrFirstTry/ftrTotal*100)}%"></div>
            <div class="bg-warning-z5 transition-all" style="width: {Math.round(ftrPartial/ftrTotal*100)}%"></div>
            <div class="bg-surface-z3 transition-all" style="width: {Math.round(ftrPending/ftrTotal*100)}%"></div>
          </div>
          <div class="flex gap-5 text-xs">
            <span><span class="inline-block h-2 w-2 rounded-full bg-success-z5 mr-1.5"></span>First try ({ftrFirstTry})</span>
            <span><span class="inline-block h-2 w-2 rounded-full bg-warning-z5 mr-1.5"></span>Partial ({ftrPartial})</span>
            <span><span class="inline-block h-2 w-2 rounded-full bg-surface-z3 mr-1.5"></span>In progress ({ftrPending})</span>
          </div>
        </div>
      </div>

      <!-- By project -->
      <div>
        <p class="mb-3 text-xs font-semibold uppercase tracking-wide text-surface-z4">Sessions by project</p>
        <div class="space-y-2">
          {#each projects as proj}
            {@const count = data.sessions.filter((s: { project: string }) => s.project === proj).length}
            <div class="flex items-center gap-3 rounded-xl border border-surface-z3/50 bg-surface-z2/50 px-4 py-2.5">
              <span class="i-solar-code-square-bold-duotone text-sm text-primary-z5 shrink-0"></span>
              <span class="w-28 truncate text-sm font-medium text-surface-z7">{proj}</span>
              <div class="flex-1 rounded-full bg-surface-z3 h-1.5 overflow-hidden">
                <div class="h-full rounded-full bg-primary-z5" style="width: {Math.round(count / data.sessions.length * 100)}%"></div>
              </div>
              <span class="text-xs text-surface-z4 w-8 text-right">{count}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

</div>
