<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  type ViewMode = 'list' | 'analytics';
  let viewMode = $state<ViewMode>('list');
  let projectFilter = $state('all');
  let expandedId = $state<string | null>(null);

  let projects = $derived([...new Set(data.sessions.map((s: any) => s.project))]);
  let filtered = $derived(
    data.sessions.filter((s: any) => projectFilter === 'all' || s.project === projectFilter)
  );

  // FTR chart
  const W = 480, H = 80;
  const L = 24, R = 8, T = 6, B = 18;

  let scored = $derived(
    [...data.sessions].filter((s: any) => s.ftr !== null).reverse()
  );

  function cx(i: number) {
    return scored.length <= 1 ? L + (W - L - R) / 2
      : L + (i / (scored.length - 1)) * (W - L - R);
  }
  function cy(v: number) { return T + (1 - v) * (H - T - B); }

  let rawPath = $derived(
    scored.length < 2 ? '' :
    scored.map((s: any, i: number) => `${i === 0 ? 'M' : 'L'}${cx(i).toFixed(1)},${cy(s.ftr).toFixed(1)}`).join(' ')
  );

  let avgPath = $derived(
    scored.length < 3 ? '' :
    scored.map((_: any, i: number) => {
      const slice = scored.slice(Math.max(0, i - 6), i + 1);
      const avg = slice.reduce((s: number, p: any) => s + p.ftr, 0) / slice.length;
      return `${i === 0 ? 'M' : 'L'}${cx(i).toFixed(1)},${cy(avg).toFixed(1)}`;
    }).join(' ')
  );

  let avgFtr = $derived(
    scored.length > 0
      ? Math.round(scored.reduce((s: number, x: any) => s + x.ftr, 0) / scored.length * 100)
      : null
  );

  function ftrClass(ftr: number | null) {
    if (ftr === null) return 'text-surface-z4';
    if (ftr >= 0.8)  return 'text-success-z6';
    if (ftr >= 0.5)  return 'text-warning-z6';
    return 'text-error-z6';
  }

  function fmt(ms: number) {
    return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
  }
</script>

<div class="flex h-full flex-col">

  <!-- Top bar -->
  <div class="flex items-center justify-between border-b border-surface-z0/50 px-5 py-3 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z7">Sessions</h1>
    <div class="flex items-center rounded-lg border border-surface-z3 bg-surface-z2 p-0.5">
      <button
        onclick={() => viewMode = 'list'}
        class="rounded-md px-3 py-1 text-xs transition-colors {viewMode === 'list' ? 'bg-primary-z2 text-primary-z7 font-medium' : 'text-surface-z5 hover:text-surface-z7'}"
      >List</button>
      <button
        onclick={() => viewMode = 'analytics'}
        class="rounded-md px-3 py-1 text-xs transition-colors {viewMode === 'analytics' ? 'bg-primary-z2 text-primary-z7 font-medium' : 'text-surface-z5 hover:text-surface-z7'}"
      >Analytics</button>
    </div>
  </div>

  {#if viewMode === 'list'}

    <!-- Filter -->
    <div class="flex items-center gap-3 border-b border-surface-z0/30 px-5 py-2 shrink-0">
      <select bind:value={projectFilter}
        class="rounded-md border border-surface-z3 bg-surface-z2 px-2.5 py-1 text-xs text-surface-z7 outline-none"
      >
        <option value="all">All projects</option>
        {#each projects as p}<option value={p}>{p}</option>{/each}
      </select>
      <span class="ml-auto text-xs text-surface-z4">{filtered.length} sessions</span>
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto">
      {#each filtered as s (s.id)}
        {@const expanded = expandedId === s.id}
        <div
          class="border-b border-surface-z0/30 hover:bg-surface-z2/40 cursor-pointer"
          onclick={() => expandedId = expanded ? null : s.id}
          role="button" tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && (expandedId = expanded ? null : s.id)}
        >
          <div class="flex items-center gap-4 px-5 py-3">
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm text-surface-z8">{s.task}</p>
              <p class="mt-0.5 text-xs text-surface-z4">{s.project} · {s.when}</p>
            </div>
            <span class="flex items-center gap-1 text-xs text-surface-z5 shrink-0">
              <span class="h-1.5 w-1.5 rounded-full {s.status === 'completed' ? 'bg-success-z5' : 'bg-primary-z6 animate-pulse'}"></span>
              {s.status}
            </span>
            <span class="w-12 text-right text-xs font-mono {ftrClass(s.ftr)} shrink-0">
              {s.ftr !== null ? `${Math.round(s.ftr * 100)}%` : '—'}
            </span>
            <span class="w-14 text-right text-xs text-surface-z5 shrink-0">${s.cost.toFixed(2)}</span>
          </div>
          {#if expanded}
            <div class="border-t border-surface-z0/20 bg-surface-z1/50 px-5 py-3 flex gap-6 text-xs">
              <span class="text-surface-z4">{Math.round(s.tokens.in / 1000)}k in · {Math.round(s.tokens.out / 1000)}k out</span>
              <span class="text-surface-z4">{s.turns} turns</span>
              <span class="{ftrClass(s.ftr)}">{s.ftr !== null ? `${Math.round(s.ftr * 100)}% FTR` : 'In progress'}</span>
            </div>
          {/if}
        </div>
      {/each}
    </div>

  {:else}

    <!-- Analytics -->
    <div class="flex-1 overflow-y-auto px-5 py-5 space-y-7">

      <!-- Key metrics -->
      <div class="flex gap-6 text-sm">
        <div>
          <p class="text-xs text-surface-z4">Sessions</p>
          <p class="mt-1 text-2xl font-semibold text-surface-z8">{data.stats.totalSessions}</p>
          <p class="text-xs text-surface-z4">{data.stats.completedSessions} completed</p>
        </div>
        <div class="w-px bg-surface-z3"></div>
        <div>
          <p class="text-xs text-surface-z4">Avg FTR</p>
          <p class="mt-1 text-2xl font-semibold {avgFtr !== null && avgFtr >= 70 ? 'text-success-z6' : 'text-warning-z6'}">{avgFtr !== null ? `${avgFtr}%` : '—'}</p>
          <p class="text-xs text-surface-z4">first-try-right</p>
        </div>
        <div class="w-px bg-surface-z3"></div>
        <div>
          <p class="text-xs text-surface-z4">Total cost</p>
          <p class="mt-1 text-2xl font-semibold text-surface-z8">${data.stats.totalCost.toFixed(2)}</p>
          <p class="text-xs text-surface-z4">{(data.stats.totalTokensIn / 1_000_000).toFixed(1)}M tokens</p>
        </div>
        <div class="w-px bg-surface-z3"></div>
        <div>
          <p class="text-xs text-surface-z4">Tool calls</p>
          <p class="mt-1 text-2xl font-semibold text-surface-z8">{data.stats.totalTurns.toLocaleString()}</p>
          <p class="text-xs text-surface-z4">across all sessions</p>
        </div>
      </div>

      <!-- FTR trend -->
      <div>
        <p class="mb-2 text-xs text-surface-z4">FTR trend</p>
        {#if scored.length < 2}
          <p class="text-xs text-surface-z4">Not enough scored sessions yet.</p>
        {:else}
          <svg width={W} height={H} viewBox="0 0 {W} {H}" class="w-full max-w-lg">
            {#each [0.5, 0.7, 1.0] as tick}
              <line x1={L} x2={W - R} y1={cy(tick)} y2={cy(tick)}
                stroke={tick === 0.7 ? 'rgb(var(--color-warning-z4))' : 'rgb(var(--color-surface-z3))'}
                stroke-width="1" stroke-dasharray={tick === 0.7 ? '3 3' : undefined} />
            {/each}
            <path d={rawPath} fill="none" stroke="rgb(var(--color-surface-z4))" stroke-width="1" opacity="0.5" />
            <path d={avgPath} fill="none" stroke="rgb(var(--color-primary-z6))" stroke-width="2" stroke-linejoin="round" />
            {#each scored as s, i}
              <circle cx={cx(i)} cy={cy(s.ftr)} r="3"
                fill={s.ftr >= 0.8 ? 'rgb(var(--color-success-z5))' : s.ftr >= 0.5 ? 'rgb(var(--color-warning-z5))' : 'rgb(var(--color-error-z5))'}
              ><title>{s.task} — {Math.round(s.ftr * 100)}%</title></circle>
            {/each}
          </svg>
          <p class="mt-1 text-[11px] text-surface-z4">
            <span class="text-primary-z6">— 7-session avg</span>
            &nbsp;· dots = raw score &nbsp;·&nbsp;
            <span class="text-warning-z6">-- 0.7 threshold</span>
          </p>
        {/if}
      </div>

      <!-- Tool usage -->
      <div>
        <p class="mb-2 text-xs text-surface-z4">Tool usage</p>
        <div class="space-y-1.5">
          {#each data.toolUsage as t}
            {@const max = data.toolUsage[0].calls}
            <div class="flex items-center gap-3">
              <span class="w-12 text-xs font-mono text-surface-z6">{t.tool}</span>
              <div class="flex-1 h-1 rounded-full bg-surface-z3 overflow-hidden">
                <div class="h-full rounded-full bg-primary-z4" style="width: {Math.round(t.calls / max * 100)}%"></div>
              </div>
              <span class="w-8 text-right text-xs text-surface-z5">{t.calls}</span>
              <span class="w-14 text-right text-[11px] text-surface-z4">{fmt(t.avgDurationMs)}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Benchmarks -->
      {#if data.benchmarkPairs.length > 0}
        <div>
          <p class="mb-2 text-xs text-surface-z4">vs without sensei</p>
          <div class="space-y-1.5">
            {#each data.benchmarkPairs as pair}
              {@const savings = Math.round((1 - pair.withSensei / pair.withoutSensei) * 100)}
              <div class="flex items-center gap-3">
                <span class="min-w-0 flex-1 truncate text-xs text-surface-z6">{pair.task}</span>
                <span class="text-xs text-surface-z4 shrink-0">${pair.withSensei.toFixed(2)} vs ${pair.withoutSensei.toFixed(2)}</span>
                <span class="w-10 text-right text-xs font-medium text-success-z6">−{savings}%</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    </div>
  {/if}

</div>
