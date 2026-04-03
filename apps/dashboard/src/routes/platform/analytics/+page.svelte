<!-- apps/dashboard/src/routes/platform/analytics/+page.svelte -->
<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const ftrColor = (score: number | null) => {
    if (score === null) return 'ftr-null';
    if (score >= 0.8) return 'ftr-high';
    if (score >= 0.5) return 'ftr-mid';
    return 'ftr-low';
  };

  // FTR distribution histogram
  const W = 480;
  const H = 140;
  const PAD = { top: 12, right: 16, bottom: 28, left: 44 };

  const maxCount = $derived(
    data.ftrDistribution.length > 0
      ? Math.max(...data.ftrDistribution.map(b => b.session_count ?? 0))
      : 0
  );

  const chartInnerW = W - PAD.left - PAD.right;
  const chartInnerH = H - PAD.top - PAD.bottom;

  // 10 buckets total; bucket index is 1-based from width_bucket
  const allBuckets = $derived(() => {
    const map = new Map<number, number>();
    for (const b of data.ftrDistribution) {
      if (b.bucket !== null) map.set(b.bucket, b.session_count ?? 0);
    }
    return Array.from({ length: 10 }, (_, i) => ({
      bucket: i + 1,
      count: map.get(i + 1) ?? 0,
      label: `${(i * 0.1).toFixed(1)}–${((i + 1) * 0.1).toFixed(1)}`,
    }));
  });

  function barX(bucketIndex: number) {
    const slotW = chartInnerW / 10;
    return PAD.left + (bucketIndex - 1) * slotW;
  }
  function barW() {
    return (chartInnerW / 10) - 2;
  }
  function barH(count: number) {
    if (maxCount === 0) return 0;
    return (count / maxCount) * chartInnerH;
  }
  function barY(count: number) {
    return PAD.top + chartInnerH - barH(count);
  }
</script>

<h1>Platform Analytics</h1>
<p style="color:#64748b;font-size:0.875rem">Anonymized cross-account data. No personal identifiers.</p>

<!-- ── FTR Distribution ─────────────────────────────────────────────────── -->
<h2>FTR Distribution</h2>
{#if data.ftrDistribution.length === 0}
  <p>No completed sessions with FTR scores yet.</p>
{:else}
  <div class="chart-wrap" aria-label="FTR score distribution histogram">
    <svg width={W} height={H} viewBox="0 0 {W} {H}">
      <!-- y-axis gridlines -->
      {#each [0.25, 0.5, 0.75, 1.0] as tick}
        {@const y = PAD.top + chartInnerH - tick * chartInnerH}
        <line x1={PAD.left} x2={W - PAD.right} y1={y} y2={y}
              stroke="#e2e8f0" stroke-width="1" />
        <text x={PAD.left - 4} y={y + 4} text-anchor="end" font-size="9" fill="#94a3b8">
          {Math.round(tick * maxCount)}
        </text>
      {/each}

      <!-- bars -->
      {#each allBuckets() as b}
        {@const x = barX(b.bucket)}
        {@const bw = barW()}
        {@const bh = barH(b.count)}
        {@const by = barY(b.count)}
        {@const midFtr = (b.bucket - 0.5) * 0.1}
        {@const barColor = midFtr >= 0.75 ? '#16a34a' : midFtr >= 0.45 ? '#d97706' : '#dc2626'}
        {#if b.count > 0}
          <rect x={x} y={by} width={bw} height={bh} fill={barColor} opacity="0.8" rx="1">
            <title>{b.label}: {b.count} sessions</title>
          </rect>
        {:else}
          <rect x={x} y={PAD.top + chartInnerH} width={bw} height="0" fill="none" />
        {/if}
      {/each}

      <!-- x-axis labels -->
      {#each [0, 0.2, 0.4, 0.6, 0.8, 1.0] as tick}
        {@const x = PAD.left + tick * chartInnerW}
        <text x={x} y={H - 6} text-anchor="middle" font-size="9" fill="#94a3b8">{tick.toFixed(1)}</text>
      {/each}

      <!-- axis lines -->
      <line x1={PAD.left} x2={PAD.left} y1={PAD.top} y2={PAD.top + chartInnerH} stroke="#e2e8f0" stroke-width="1" />
      <line x1={PAD.left} x2={W - PAD.right} y1={PAD.top + chartInnerH} y2={PAD.top + chartInnerH} stroke="#e2e8f0" stroke-width="1" />
    </svg>
    <p class="chart-legend">
      <span style="color:#16a34a">■ ≥ 0.8 high</span>
      &nbsp;&nbsp;
      <span style="color:#d97706">■ 0.5–0.8 mid</span>
      &nbsp;&nbsp;
      <span style="color:#dc2626">■ &lt; 0.5 low</span>
    </p>
  </div>
{/if}

<!-- ── Account Stats ────────────────────────────────────────────────────── -->
<h2>Account Stats</h2>
{#if data.accountStats.length === 0}
  <p>No account data available yet.</p>
{:else}
  <table class="data-table">
    <thead>
      <tr>
        <th>Account</th>
        <th>Type</th>
        <th class="num">Repos</th>
        <th class="num">Sessions (30d)</th>
        <th class="num">Avg FTR</th>
        <th class="num">Total Cost (USD)</th>
      </tr>
    </thead>
    <tbody>
      {#each data.accountStats as row}
        <tr>
          <td class="mono" style="font-size:0.8125rem">{row.account_slug ?? '—'}</td>
          <td>{row.account_type ?? '—'}</td>
          <td class="num">{row.repo_count ?? 0}</td>
          <td class="num">{row.sessions_30d ?? 0}</td>
          <td class="num {ftrColor(row.avg_ftr)}">
            {row.avg_ftr !== null ? row.avg_ftr.toFixed(3) : '—'}
          </td>
          <td class="num mono">
            {row.total_cost_usd !== null ? `$${Number(row.total_cost_usd).toFixed(4)}` : '—'}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<!-- ── Tool Usage ───────────────────────────────────────────────────────── -->
<h2>Tool Usage</h2>
{#if data.toolUsage.length === 0}
  <p>No tool call data available yet.</p>
{:else}
  <table class="data-table">
    <thead>
      <tr>
        <th>Tool</th>
        <th class="num">Total Calls</th>
        <th class="num">Error Rate</th>
        <th class="num">Avg Duration</th>
      </tr>
    </thead>
    <tbody>
      {#each data.toolUsage as row}
        {@const errorPct = row.error_rate !== null ? Math.round(Number(row.error_rate) * 100) : null}
        {@const durationMs = row.avg_duration_ms !== null ? Math.round(Number(row.avg_duration_ms)) : null}
        <tr>
          <td class="mono" style="font-size:0.8125rem">{row.tool ?? '—'}</td>
          <td class="num">{(row.total_calls ?? 0).toLocaleString()}</td>
          <td class="num" class:error-high={errorPct !== null && errorPct >= 10}>
            {errorPct !== null ? `${errorPct}%` : '—'}
          </td>
          <td class="num mono">
            {#if durationMs === null}
              —
            {:else if durationMs >= 1000}
              {(durationMs / 1000).toFixed(1)}s
            {:else}
              {durationMs}ms
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  h2 { margin: 24px 0 12px; font-size: 1rem; font-weight: 600; color: #374151; }
  .chart-wrap { background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 12px 16px 4px; display: inline-block; }
  .chart-legend { font-size: 0.7rem; color: #94a3b8; margin: 4px 0 0; }
  .data-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; margin-bottom: 8px; }
  .data-table th { text-align: left; padding: 6px 12px 6px 0; border-bottom: 2px solid #e2e8f0; color: #64748b; font-weight: 600; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.03em; }
  .data-table td { padding: 8px 12px 8px 0; border-bottom: 1px solid #f1f5f9; }
  .data-table tr:hover td { background: #f8fafc; }
  .data-table .num { text-align: right; }
  .data-table .mono { font-family: monospace; }
  .ftr-high { color: #166534; font-weight: 600; font-family: monospace; }
  .ftr-mid  { color: #92400e; font-weight: 600; font-family: monospace; }
  .ftr-low  { color: #991b1b; font-weight: 600; font-family: monospace; }
  .ftr-null { color: #94a3b8; font-family: monospace; }
  .error-high { color: #991b1b; font-weight: 600; }
</style>
