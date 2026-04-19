<script lang="ts">
  import { dashboardDummy } from '$lib/observatory/dummy.js';
  import type { DashboardData, MetricValue } from '$lib/observatory/types.js';

  // TODO: swap for real API call
  let data: DashboardData = dashboardDummy();

  function fmtMetric(m: MetricValue, formatter: (v: number) => string): string {
    if (m.quality === 'unavailable') return '—';
    return formatter(m.value as number);
  }

  let adherencePct = $derived(data.toolAdherence.total > 0 ? (data.toolAdherence.mcp / data.toolAdherence.total) * 100 : 0);

  function outcomeBadge(outcome: string | null) {
    if (outcome === 'completed') return { cls: 'bg-success-z2 text-success-z7', icon: '✓' };
    if (outcome === 'partial') return { cls: 'bg-warning-z2 text-warning-z7', icon: '△' };
    if (outcome === 'blocked') return { cls: 'bg-error-z2 text-error-z7', icon: '✗' };
    return { cls: 'bg-surface-z3 text-surface-z5', icon: '·' };
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-lg font-semibold text-surface-z8">Dashboard</h2>
      <p class="text-xs text-surface-z4">{data.period.label} · {data.period.from} — {data.period.to}</p>
    </div>
    {#if data.activeTask}
      <div class="rounded-lg bg-primary-z2/50 px-3 py-1.5 text-xs text-primary-z7">
        <span class="font-medium">{data.activeTask.phase}</span>
        {#if data.activeTask.issue} · {data.activeTask.issue}{/if}
        {#if data.activeTask.task} · {data.activeTask.task}{/if}
      </div>
    {/if}
  </div>

  <!-- Metrics grid -->
  <div class="grid grid-cols-5 gap-3">
    {#each [
      { label: 'FTR', metric: data.ftr, fmt: (v: number) => `${Math.round(v * 100)}%`, trend: '\u2191 5%' },
      { label: 'Sessions', metric: data.sessionCount, fmt: (v: number) => String(v), trend: null },
      { label: 'Rework', metric: data.reworkRate, fmt: (v: number) => `${Math.round(v * 100)}%`, trend: '\u2193 4%' },
      { label: 'Tokens', metric: data.tokens, fmt: (v: number) => v.toLocaleString(), trend: null },
      { label: 'Cost', metric: data.cost, fmt: (v: number) => `$${v.toFixed(2)}`, trend: null },
    ] as card}
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z5 uppercase tracking-wide font-medium">{card.label}</p>
        {#if card.metric.quality === 'unavailable'}
          <p class="mt-1 text-xl font-semibold text-surface-z3">&mdash;</p>
          {#if card.metric.trackingUrl}
            <a href={card.metric.trackingUrl} target="_blank" class="text-[9px] text-primary-z5 hover:text-primary-z6">Track issue</a>
          {/if}
        {:else}
          <div class="mt-1 flex items-baseline gap-1.5">
            <p class="text-xl font-semibold {card.metric.quality === 'estimated' ? 'text-surface-z6' : 'text-primary-z6'}">
              {fmtMetric(card.metric, card.fmt)}
            </p>
            {#if card.metric.quality === 'estimated'}
              <span class="rounded bg-warning-z2 px-1 py-0.5 text-[8px] font-medium text-warning-z7">est.</span>
            {/if}
          </div>
          {#if card.trend}
            <p class="mt-0.5 text-[10px] text-surface-z4">{card.trend}</p>
          {/if}
        {/if}
      </div>
    {/each}
  </div>

  <!-- Tool adherence -->
  <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-4">
    <div class="flex items-center justify-between mb-2">
      <p class="text-xs font-medium text-surface-z6">Tool Adherence</p>
      <p class="text-xs text-surface-z4">{data.toolAdherence.mcp} MCP · {data.toolAdherence.fallback} fallback</p>
    </div>
    <div class="h-2 rounded-full bg-surface-z3 overflow-hidden">
      <div class="h-full rounded-full bg-primary-z5 transition-all" style="width: {adherencePct}%"></div>
    </div>
    <p class="mt-1 text-[10px] text-surface-z4">
      {Math.round(adherencePct)}% MCP usage
      {#if data.toolAdherence.fallback > 0}
        — <span class="text-warning-z6">{data.toolAdherence.fallback} fallback calls could use MCP</span>
      {/if}
    </p>
  </div>

  <!-- Recent sessions -->
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">Recent Sessions</p>
      <a href="/sessions" class="text-[10px] text-primary-z5 hover:text-primary-z6">View all →</a>
    </div>

    <div class="space-y-1">
      {#each data.recentSessions as session (session.id)}
        {@const badge = outcomeBadge(session.outcome)}
        <a href="/sessions/{session.id}" class="flex items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2.5 text-sm hover:bg-surface-z3/60 transition-colors">
          <span class="flex h-5 w-5 items-center justify-center rounded-md text-[10px] font-bold {badge.cls}">{badge.icon}</span>
          <span class="flex-1 truncate text-surface-z7">{session.task}</span>
          <span class="text-[10px] text-surface-z4 shrink-0">{session.turns} turns</span>
          {#if session.corrections > 0}
            <span class="text-[10px] text-warning-z6 shrink-0">{session.corrections} corrections</span>
          {/if}
          {#if session.tokens.quality !== 'unavailable'}
            <span class="text-[10px] text-surface-z4 shrink-0">{Math.round((session.tokens.value as number) / 1000)}k tokens</span>
          {/if}
        </a>
      {/each}
    </div>
  </div>

</div>
