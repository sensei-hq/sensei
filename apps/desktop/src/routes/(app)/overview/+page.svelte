<script lang="ts">
  import { overviewDummy } from '$lib/observatory/dummy.js';
  import type { OverviewData } from '$lib/observatory/types.js';

  let data: OverviewData = overviewDummy();

  function stateCls(state: string): string {
    if (state === 'active') return 'bg-success-z2 text-success-z7';
    if (state === 'recent') return 'bg-info-z2 text-info-z7';
    if (state === 'inactive') return 'bg-surface-z3 text-surface-z5';
    return 'bg-surface-z3 text-surface-z4';
  }

  function ftrColor(v: number): string {
    if (v >= 0.8) return 'text-success-z6';
    if (v >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }

  function pctFmt(v: number): string { return `${Math.round(v * 100)}%`; }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <div>
    <h2 class="text-lg font-semibold text-surface-z8">Overview</h2>
    <p class="text-xs text-surface-z4">Across all solutions &middot; {data.globalMetrics.period.label}</p>
  </div>

  <!-- Global metrics -->
  <div class="grid grid-cols-4 gap-3">
    {#each [
      { label: 'FTR', value: pctFmt(data.globalMetrics.ftr.value as number) },
      { label: 'Sessions', value: String(data.globalMetrics.sessionCount.value) },
      { label: 'Rework', value: pctFmt(data.globalMetrics.reworkRate.value as number) },
      { label: 'MCP Usage', value: `${Math.round(data.globalMetrics.toolAdherence.total > 0 ? (data.globalMetrics.toolAdherence.mcp / data.globalMetrics.toolAdherence.total) * 100 : 0)}%` },
    ] as card}
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z5 uppercase tracking-wide font-medium">{card.label}</p>
        <p class="mt-1 text-xl font-semibold text-primary-z6">{card.value}</p>
      </div>
    {/each}
  </div>

  <!-- Solutions list -->
  <div class="space-y-2">
    <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Solutions</p>

    {#each data.solutions as sol (sol.id)}
      <a href="/s/{sol.id}" class="block rounded-lg bg-surface-z2 px-4 py-3 hover:bg-surface-z3/60 transition-colors">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary-z3 text-sm font-bold text-primary-z7">
              {sol.name.charAt(0).toUpperCase()}
            </div>
            <div>
              <p class="text-sm font-medium text-surface-z7">{sol.name}</p>
              <p class="text-[10px] text-surface-z4">{sol.projects.length} projects &middot; {sol.description ?? ''}</p>
            </div>
          </div>
          <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {stateCls(sol.state)}">{sol.state}</span>
        </div>

        <!-- Per-solution sparkline row -->
        <div class="mt-2 grid grid-cols-4 gap-4 text-[10px]">
          <div>
            <span class="text-surface-z4">FTR</span>
            <span class="ml-1 font-medium {ftrColor(sol.metrics.ftr.value as number)}">{pctFmt(sol.metrics.ftr.value as number)}</span>
          </div>
          <div>
            <span class="text-surface-z4">Sessions</span>
            <span class="ml-1 text-surface-z6">{sol.metrics.sessionCount.value}</span>
          </div>
          <div>
            <span class="text-surface-z4">Rework</span>
            <span class="ml-1 text-surface-z6">{pctFmt(sol.metrics.reworkRate.value as number)}</span>
          </div>
          <div>
            <span class="text-surface-z4">Tokens</span>
            <span class="ml-1 text-surface-z6">{((sol.metrics.tokens.value as number) / 1000).toFixed(0)}k</span>
            {#if sol.metrics.tokens.quality === 'estimated'}
              <span class="text-warning-z6">~</span>
            {/if}
          </div>
        </div>
      </a>
    {/each}
  </div>

</div>
