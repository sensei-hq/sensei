<script lang="ts">
  import { dashboardDummy } from '$lib/observatory/dummy.js';
  import type { SessionSummary } from '$lib/observatory/types.js';

  // TODO: swap for real API call (paginated session list)
  let sessions: SessionSummary[] = dashboardDummy().recentSessions;

  function outcomeBadge(outcome: string | null) {
    if (outcome === 'completed') return { cls: 'bg-success-z2 text-success-z7', label: 'completed' };
    if (outcome === 'partial') return { cls: 'bg-warning-z2 text-warning-z7', label: 'partial' };
    if (outcome === 'blocked') return { cls: 'bg-error-z2 text-error-z7', label: 'blocked' };
    return { cls: 'bg-surface-z3 text-surface-z5', label: '—' };
  }

  function ftrColor(ftr: number | null): string {
    if (ftr === null) return 'text-surface-z4';
    if (ftr >= 0.8) return 'text-success-z6';
    if (ftr >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-5">

  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold text-surface-z8">Sessions</h2>
    <p class="text-xs text-surface-z4">{sessions.length} sessions</p>
  </div>

  <!-- Table header -->
  <div class="grid grid-cols-[1fr_80px_60px_60px_60px_70px] gap-2 px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wide text-surface-z4">
    <span>Task</span>
    <span>Outcome</span>
    <span class="text-right">FTR</span>
    <span class="text-right">Turns</span>
    <span class="text-right">Fixes</span>
    <span class="text-right">Tokens</span>
  </div>

  <!-- Rows -->
  <div class="space-y-1">
    {#each sessions as session (session.id)}
      {@const badge = outcomeBadge(session.outcome)}
      <a href="/sessions/{session.id}" class="grid grid-cols-[1fr_80px_60px_60px_60px_70px] gap-2 items-center rounded-lg bg-surface-z2 px-3 py-2.5 text-sm hover:bg-surface-z3/60 transition-colors">
        <div class="truncate">
          <span class="text-surface-z7">{session.task}</span>
          <span class="ml-1.5 text-[10px] text-surface-z4">{session.project}</span>
        </div>
        <span class="rounded px-1.5 py-0.5 text-[10px] font-medium text-center {badge.cls}">{badge.label}</span>
        <span class="text-right text-xs font-medium {ftrColor(session.ftr)}">
          {session.ftr !== null ? `${Math.round(session.ftr * 100)}%` : '—'}
        </span>
        <span class="text-right text-xs text-surface-z6">{session.turns}</span>
        <span class="text-right text-xs {session.corrections > 0 ? 'text-warning-z6' : 'text-surface-z4'}">
          {session.corrections || '—'}
        </span>
        <span class="text-right text-xs text-surface-z4">
          {#if session.tokens.quality !== 'unavailable'}
            {Math.round((session.tokens.value as number) / 1000)}k
            {#if session.tokens.quality === 'estimated'}
              <span class="text-[8px] text-warning-z6">~</span>
            {/if}
          {:else}
            —
          {/if}
        </span>
      </a>
    {/each}
  </div>

</div>
