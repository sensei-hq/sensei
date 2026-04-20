<script lang="ts">
  import { page } from '$app/stores';
  import { projectDashboardDummy } from '$lib/observatory/dummy.js';
  import type { ProjectDashboardData, MetricValue } from '$lib/observatory/types.js';

  let data: ProjectDashboardData = projectDashboardDummy($page.params.id ?? '', $page.params.pid ?? '');

  function fmtMetric(m: MetricValue, fmt: (v: number) => string): string {
    if (m.quality === 'unavailable') return '—';
    return fmt(m.value as number);
  }

  function outcomeBadge(outcome: string | null) {
    if (outcome === 'completed') return { cls: 'bg-success-z2 text-success-z7', icon: '\u2713' };
    if (outcome === 'partial') return { cls: 'bg-warning-z2 text-warning-z7', icon: '\u25B3' };
    if (outcome === 'blocked') return { cls: 'bg-error-z2 text-error-z7', icon: '\u2717' };
    return { cls: 'bg-surface-z3 text-surface-z5', icon: '\u00B7' };
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <!-- Breadcrumb -->
  <div>
    <div class="flex items-center gap-1.5 text-[10px] text-surface-z4">
      <a href="/s/{data.solutionId}" class="text-primary-z5 hover:text-primary-z6">{data.solutionName}</a>
      <span>/</span>
      <span class="text-surface-z6">{data.projectName}</span>
    </div>
    <h2 class="mt-1 text-lg font-semibold text-surface-z8">{data.projectName}</h2>
    {#if data.activeTask}
      <p class="text-xs text-surface-z4 mt-0.5">
        {data.activeTask.phase} &middot; {data.activeTask.issue ?? ''} &middot; {data.activeTask.task ?? ''}
      </p>
    {/if}
  </div>

  <!-- Metrics -->
  <div class="grid grid-cols-5 gap-3">
    {#each [
      { label: 'FTR', value: fmtMetric(data.metrics.ftr, v => `${Math.round(v * 100)}%`) },
      { label: 'Sessions', value: fmtMetric(data.metrics.sessionCount, v => String(v)) },
      { label: 'Rework', value: fmtMetric(data.metrics.reworkRate, v => `${Math.round(v * 100)}%`) },
      { label: 'Tokens', value: fmtMetric(data.metrics.tokens, v => `${(v / 1000).toFixed(0)}k`) },
      { label: 'Cost', value: fmtMetric(data.metrics.cost, v => `$${v.toFixed(2)}`) },
    ] as card}
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z5 uppercase tracking-wide font-medium">{card.label}</p>
        <p class="mt-1 text-xl font-semibold {card.value === '—' ? 'text-surface-z3' : 'text-primary-z6'}">{card.value}</p>
      </div>
    {/each}
  </div>

  <!-- Quick nav -->
  <div class="flex gap-2">
    {#each [
      { href: `/s/${data.solutionId}/p/${data.projectId}/sessions`, label: 'Sessions', icon: 'i-solar-history-bold-duotone' },
      { href: `/s/${data.solutionId}/p/${data.projectId}/code`, label: 'Code', icon: 'i-solar-code-square-bold-duotone' },
      { href: `/s/${data.solutionId}/p/${data.projectId}/profiles`, label: 'Profiles', icon: 'i-solar-user-id-bold-duotone' },
      { href: `/s/${data.solutionId}/p/${data.projectId}/indexer`, label: 'Indexer', icon: 'i-solar-database-bold-duotone' },
    ] as nav}
      <a href={nav.href} class="flex items-center gap-1.5 rounded-md bg-surface-z2 px-3 py-1.5 text-xs text-surface-z6 hover:bg-surface-z3/60 hover:text-surface-z7 transition-colors">
        <span class="{nav.icon} text-sm"></span>
        {nav.label}
      </a>
    {/each}
  </div>

  <!-- Recent sessions -->
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Recent Sessions</p>
      <a href="/s/{data.solutionId}/p/{data.projectId}/sessions" class="text-[10px] text-primary-z5 hover:text-primary-z6">View all &rarr;</a>
    </div>
    <div class="space-y-1">
      {#each data.recentSessions as session (session.id)}
        {@const badge = outcomeBadge(session.outcome)}
        <a href="/sessions/{session.id}" class="flex items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2.5 text-sm hover:bg-surface-z3/60 transition-colors">
          <span class="flex h-5 w-5 items-center justify-center rounded-md text-[10px] font-bold {badge.cls}">{badge.icon}</span>
          <span class="flex-1 truncate text-surface-z7">{session.task}</span>
          <span class="text-[10px] text-surface-z4 shrink-0">{session.turns} turns</span>
          {#if session.corrections > 0}
            <span class="text-[10px] text-warning-z6 shrink-0">{session.corrections} fixes</span>
          {/if}
        </a>
      {/each}
      {#if data.recentSessions.length === 0}
        <p class="text-xs text-surface-z4 text-center py-4">No sessions yet for this project</p>
      {/if}
    </div>
  </div>

</div>
