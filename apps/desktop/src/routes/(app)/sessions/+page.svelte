<script lang="ts">
  import type { PageData } from './$types';
  let { data }: { data: PageData } = $props();

  type ViewMode = 'list' | 'analytics';
  let viewMode = $state<ViewMode>('list');
  let projectFilter = $state('all');
  let expandedId = $state<string | null>(null);

  let projects = $derived([...new Set(data.sessions.map((s: any) => s.project))]);

  let filtered = $derived(
    data.sessions.filter((s: any) =>
      projectFilter === 'all' || s.project === projectFilter
    )
  );

  // FTR chart — scored sessions oldest→newest
  const W = 480, H = 96;
  const PAD = { t: 8, r: 12, b: 20, l: 28 };

  let ftrPoints = $derived(
    [...data.sessions]
      .filter((s: any) => s.ftr !== null)
      .reverse()
      .map((s: any, i: number, arr: any[]) => ({ x: i, y: s.ftr as number, label: s.task }))
  );

  function svgX(i: number, total: number) {
    return total <= 1 ? PAD.l + (W - PAD.l - PAD.r) / 2
      : PAD.l + (i / (total - 1)) * (W - PAD.l - PAD.r);
  }
  function svgY(score: number) {
    return PAD.t + (1 - score) * (H - PAD.t - PAD.b);
  }

  let rawPath = $derived(
    ftrPoints.length < 2 ? '' :
    ftrPoints.map((p: any, i: number) =>
      `${i === 0 ? 'M' : 'L'}${svgX(i, ftrPoints.length).toFixed(1)},${svgY(p.y).toFixed(1)}`
    ).join(' ')
  );

  let avgPath = $derived(
    ftrPoints.length < 3 ? '' :
    ftrPoints.map((_: any, i: number) => {
      const slice = ftrPoints.slice(Math.max(0, i - 6), i + 1);
      const avg = slice.reduce((s: number, p: any) => s + p.y, 0) / slice.length;
      return `${i === 0 ? 'M' : 'L'}${svgX(i, ftrPoints.length).toFixed(1)},${svgY(avg).toFixed(1)}`;
    }).join(' ')
  );

  let scoredSessions = $derived(data.sessions.filter((s: any) => s.ftr !== null));
  let avgFtr = $derived(
    scoredSessions.length > 0
      ? Math.round((scoredSessions.reduce((s: number, x: any) => s + x.ftr, 0) / scoredSessions.length) * 100)
      : null
  );

  function ftrColor(ftr: number | null) {
    if (ftr === null) return 'text-surface-z4';
    if (ftr >= 0.8)  return 'text-success-z6';
    if (ftr >= 0.5)  return 'text-warning-z6';
    return 'text-error-z6';
  }

  function fmtDuration(ms: number) {
    return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
  }

  function fmtTokens(n: number) {
    return n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : `${Math.round(n / 1000)}k`;
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

    <!-- Filter bar -->
    <div class="flex items-center gap-3 border-b border-surface-z0/30 px-5 py-2 shrink-0">
      <select
        bind:value={projectFilter}
        class="rounded-md border border-surface-z3 bg-surface-z2 px-2.5 py-1 text-xs text-surface-z7 outline-none"
      >
        <option value="all">All projects</option>
        {#each projects as p}<option value={p}>{p}</option>{/each}
      </select>
      <span class="ml-auto text-xs text-surface-z4">{filtered.length} sessions</span>
    </div>

    <!-- Session list -->
    <div class="flex-1 overflow-y-auto">
      {#each filtered as s (s.id)}
        {@const expanded = expandedId === s.id}
        <div
          class="border-b border-surface-z0/30 transition-colors hover:bg-surface-z2/40 cursor-pointer"
          onclick={() => expandedId = expanded ? null : s.id}
          role="button"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && (expandedId = expanded ? null : s.id)}
        >
          <!-- Row -->
          <div class="flex items-center gap-4 px-5 py-3">
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm text-surface-z8">{s.task}</p>
              <p class="mt-0.5 text-xs text-surface-z4">{s.project} · {s.when}</p>
            </div>
            <div class="flex items-center gap-3 shrink-0">
              <span class="flex items-center gap-1 text-xs text-surface-z5">
                <span class="h-1.5 w-1.5 rounded-full {s.status === 'completed' ? 'bg-success-z5' : 'bg-primary-z6 animate-pulse'}"></span>
                {s.status}
              </span>
              <span class="w-14 text-right text-xs font-mono {ftrColor(s.ftr)}">
                {s.ftr !== null ? `${Math.round(s.ftr * 100)}%` : '—'}
              </span>
              <span class="w-12 text-right text-xs text-surface-z4">{s.turns} turns</span>
              <span class="w-14 text-right text-xs text-surface-z5">${s.cost.toFixed(2)}</span>
              <span class="text-surface-z3 text-xs transition-transform {expanded ? 'rotate-180' : ''}">▾</span>
            </div>
          </div>

          <!-- Expanded detail -->
          {#if expanded}
            <div class="border-t border-surface-z0/30 bg-surface-z1/50 px-5 py-3">
              <div class="grid grid-cols-3 gap-4 text-xs">
                <div>
                  <p class="text-surface-z4 mb-1">Tokens</p>
                  <p class="text-surface-z6">{fmtTokens(s.tokens.in)} in · {fmtTokens(s.tokens.out)} out</p>
                </div>
                <div>
                  <p class="text-surface-z4 mb-1">Cost</p>
                  <p class="text-surface-z6">${s.cost.toFixed(4)}</p>
                </div>
                <div>
                  <p class="text-surface-z4 mb-1">FTR</p>
                  <p class="{ftrColor(s.ftr)}">{s.ftr !== null ? `${Math.round(s.ftr * 100)}% first-try-right` : 'In progress'}</p>
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>

  {:else}

    <!-- Analytics view -->
    <div class="flex-1 overflow-y-auto px-5 py-5 space-y-6">

      <!-- Summary row -->
      <div class="grid grid-cols-4 gap-3">
        {#each [
          { label: 'Sessions',   value: data.stats.totalSessions,                        sub: `${data.stats.completedSessions} completed` },
          { label: 'Avg FTR',    value: avgFtr !== null ? `${avgFtr}%` : '—',             sub: 'First-try-right' },
          { label: 'Total cost', value: `$${data.stats.totalCost.toFixed(2)}`,            sub: `${fmtTokens(data.stats.totalTokensIn)} tokens` },
          { label: 'Tool calls', value: data.stats.totalTurns.toLocaleString(),            sub: 'Across all sessions' },
        ] as card}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 px-4 py-3">
            <p class="text-xs text-surface-z4">{card.label}</p>
            <p class="mt-1 text-xl font-semibold text-surface-z9">{card.value}</p>
            <p class="mt-0.5 text-[11px] text-surface-z4">{card.sub}</p>
          </div>
        {/each}
      </div>

      <!-- FTR trend -->
      <div>
        <p class="mb-3 text-xs font-medium text-surface-z5">FTR trend</p>
        {#if ftrPoints.length === 0}
          <p class="text-xs text-surface-z4">No scored sessions yet.</p>
        {:else}
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 p-4">
            <svg width={W} height={H} viewBox="0 0 {W} {H}" class="w-full" preserveAspectRatio="none">
              {#each [0.5, 0.7, 1.0] as tick}
                {@const y = svgY(tick)}
                <line x1={PAD.l} x2={W - PAD.r} y1={y} y2={y}
                      stroke={tick === 0.7 ? 'rgb(var(--color-warning-z4))' : 'rgb(var(--color-surface-z3))'}
                      stroke-width="1" stroke-dasharray={tick === 0.7 ? '4 3' : undefined} />
                <text x={PAD.l - 4} y={y + 4} text-anchor="end" font-size="8" fill="rgb(var(--color-surface-z4))">{tick.toFixed(1)}</text>
              {/each}
              {#if rawPath}
                <path d={rawPath} fill="none" stroke="rgb(var(--color-surface-z4))" stroke-width="1" opacity="0.5" />
              {/if}
              {#if avgPath}
                <path d={avgPath} fill="none" stroke="rgb(var(--color-primary-z6))" stroke-width="2" stroke-linejoin="round" />
              {/if}
              {#each ftrPoints as p, i}
                {@const cx = svgX(i, ftrPoints.length)}
                {@const cy = svgY(p.y)}
                <circle cx={cx} cy={cy} r="3"
                  fill={p.y >= 0.8 ? 'rgb(var(--color-success-z5))' : p.y >= 0.5 ? 'rgb(var(--color-warning-z5))' : 'rgb(var(--color-error-z5))'}
                >
                  <title>{p.label} — {Math.round(p.y * 100)}%</title>
                </circle>
              {/each}
            </svg>
            <div class="mt-2 flex gap-4 text-[11px] text-surface-z4">
              <span>● raw score</span>
              <span class="text-primary-z6">— 7-session avg</span>
              <span class="text-warning-z6">-- 0.7 threshold</span>
            </div>
          </div>
        {/if}
      </div>

      <!-- Tool usage -->
      <div>
        <p class="mb-3 text-xs font-medium text-surface-z5">Tool usage</p>
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
          {#each data.toolUsage as t, i}
            {@const maxCalls = data.toolUsage[0].calls}
            <div class="flex items-center gap-4 px-4 py-2.5 {i < data.toolUsage.length - 1 ? 'border-b border-surface-z0/30' : ''}">
              <span class="w-14 text-xs font-mono text-surface-z7">{t.tool}</span>
              <div class="flex-1 h-1.5 rounded-full bg-surface-z3 overflow-hidden">
                <div class="h-full rounded-full bg-primary-z5" style="width: {Math.round(t.calls / maxCalls * 100)}%"></div>
              </div>
              <span class="w-10 text-right text-xs text-surface-z5">{t.calls}</span>
              <span class="w-16 text-right text-xs text-surface-z4">{Math.round(t.successRate * 100)}% ok</span>
              <span class="w-14 text-right text-xs text-surface-z4">{fmtDuration(t.avgDurationMs)}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Cost by session -->
      <div>
        <p class="mb-3 text-xs font-medium text-surface-z5">Cost by session</p>
        <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
          {#each data.sessions as s, i}
            <div class="flex items-center gap-4 px-4 py-2.5 {i < data.sessions.length - 1 ? 'border-b border-surface-z0/30' : ''}">
              <span class="min-w-0 flex-1 truncate text-xs text-surface-z7">{s.task}</span>
              <span class="text-xs text-surface-z4 shrink-0">{fmtTokens(s.tokens.in)} in</span>
              <span class="text-xs text-surface-z4 shrink-0">{fmtTokens(s.tokens.out)} out</span>
              <span class="w-14 text-right text-xs font-mono text-surface-z6">${s.cost.toFixed(4)}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Benchmarks -->
      {#if data.benchmarkPairs.length > 0}
        <div>
          <p class="mb-3 text-xs font-medium text-surface-z5">Benchmark — with vs without sensei</p>
          <div class="rounded-xl border border-surface-z3 bg-surface-z2 overflow-hidden">
            {#each data.benchmarkPairs as pair, i}
              {@const savings = pair.withoutSensei - pair.withSensei}
              {@const pct = Math.round((savings / pair.withoutSensei) * 100)}
              <div class="flex items-center gap-4 px-4 py-2.5 {i < data.benchmarkPairs.length - 1 ? 'border-b border-surface-z0/30' : ''}">
                <span class="min-w-0 flex-1 truncate text-xs text-surface-z7">{pair.task}</span>
                <span class="text-xs text-surface-z4">${pair.withoutSensei.toFixed(2)} → ${pair.withSensei.toFixed(2)}</span>
                <span class="text-xs font-medium text-success-z6">−{pct}%</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

    </div>
  {/if}

</div>
