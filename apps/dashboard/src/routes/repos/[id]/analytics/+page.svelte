<!-- apps/dashboard/src/routes/repos/[id]/analytics/+page.svelte -->
<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const fmt = (iso: string) => new Date(iso).toLocaleString();
  const truncate = (s: string | null, n = 60) => s ? (s.length > n ? s.slice(0, n) + '…' : s) : '—';
  const ftrColor = (score: number | null) => {
    if (score === null) return 'ftr-null';
    if (score >= 0.8) return 'ftr-high';
    if (score >= 0.5) return 'ftr-mid';
    return 'ftr-low';
  };

  const toolColumns = [
    { name: 'tool',         label: 'Tool',         sortable: true },
    { name: 'calls',        label: 'Calls',        sortable: true },
    { name: 'successPct',   label: 'Success',      sortable: true },
    { name: 'avgDuration',  label: 'Avg Duration', sortable: true },
  ];

  const toolRows = $derived(data.toolUsage.map((t) => ({
    tool: t.tool,
    calls: t.calls,
    successPct: `${Math.round(t.successRate * 100)}%`,
    avgDuration: t.avgDurationMs !== null
      ? (t.avgDurationMs >= 1000 ? `${(t.avgDurationMs / 1000).toFixed(1)}s` : `${t.avgDurationMs}ms`)
      : '—',
  })));

  // FTR trend chart — sessions ordered oldest→newest with a score
  const W = 560;
  const H = 120;
  const PAD = { top: 12, right: 16, bottom: 24, left: 36 };

  const ftrPoints = $derived(
    [...data.sessions]
      .reverse()
      .filter(s => s.ftrScore !== null)
      .map((s, i, arr) => ({ x: i, y: s.ftrScore as number, label: s.createdAt }))
  );

  function toSvgX(i: number, total: number) {
    return total <= 1
      ? PAD.left + (W - PAD.left - PAD.right) / 2
      : PAD.left + (i / (total - 1)) * (W - PAD.left - PAD.right);
  }
  function toSvgY(score: number) {
    return PAD.top + (1 - score) * (H - PAD.top - PAD.bottom);
  }

  const chartPath = $derived(() => {
    if (ftrPoints.length < 2) return '';
    return ftrPoints.map((p, i) => {
      const x = toSvgX(i, ftrPoints.length);
      const y = toSvgY(p.y);
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(' ');
  });

  // Rolling 7-session average line
  const rollingAvgPath = $derived(() => {
    if (ftrPoints.length < 3) return '';
    const window = 7;
    return ftrPoints.map((_, i) => {
      const slice = ftrPoints.slice(Math.max(0, i - window + 1), i + 1);
      const avg = slice.reduce((s, p) => s + p.y, 0) / slice.length;
      const x = toSvgX(i, ftrPoints.length);
      const y = toSvgY(avg);
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
    }).join(' ');
  });

  // Outcome breakdown
  const outcomes = $derived({
    completed: data.sessions.filter(s => s.status === 'completed').length,
    in_progress: data.sessions.filter(s => s.status === 'in_progress').length,
    abandoned: data.sessions.filter(s => s.status === 'abandoned').length,
    total: data.sessions.length,
  });

  // Avg FTR
  const scoredSessions = $derived(data.sessions.filter(s => s.ftrScore !== null));
  const avgFtr = $derived(
    scoredSessions.length > 0
      ? scoredSessions.reduce((s, x) => s + (x.ftrScore ?? 0), 0) / scoredSessions.length
      : null
  );
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Analytics — {data.repo.name}</h1>
<p style="color:#64748b;font-size:0.875rem">Last 30 days</p>

<h2>FTR Trend — Last 30 Days</h2>
{#if ftrPoints.length === 0}
  <p>No scored sessions yet. Sessions get an FTR score after calling <code>checkpoint()</code>.</p>
{:else}
  <div class="ftr-summary-row">
    <div class="ftr-stat">
      <span class="ftr-stat-label">Avg FTR</span>
      <span class="ftr-stat-value {ftrColor(avgFtr)}">{avgFtr !== null ? avgFtr.toFixed(3) : '—'}</span>
    </div>
    <div class="ftr-stat">
      <span class="ftr-stat-label">Completed</span>
      <span class="ftr-stat-value">{outcomes.completed}</span>
    </div>
    <div class="ftr-stat">
      <span class="ftr-stat-label">In progress</span>
      <span class="ftr-stat-value">{outcomes.in_progress}</span>
    </div>
    <div class="ftr-stat">
      <span class="ftr-stat-label">Abandoned</span>
      <span class="ftr-stat-value">{outcomes.abandoned}</span>
    </div>
    <div class="ftr-stat">
      <span class="ftr-stat-label">Scored</span>
      <span class="ftr-stat-value">{scoredSessions.length} / {outcomes.total}</span>
    </div>
  </div>

  <div class="chart-wrap" aria-label="FTR score trend chart">
    <svg width={W} height={H} viewBox="0 0 {W} {H}">
      <!-- grid lines at 0.3, 0.5, 0.7, 1.0 -->
      {#each [0.3, 0.5, 0.7, 1.0] as tick}
        {@const y = toSvgY(tick)}
        <line x1={PAD.left} x2={W - PAD.right} y1={y} y2={y}
              stroke={tick === 0.7 ? '#fbbf24' : '#e2e8f0'} stroke-width="1" stroke-dasharray={tick === 0.7 ? '4 3' : undefined} />
        <text x={PAD.left - 4} y={y + 4} text-anchor="end" font-size="9" fill="#94a3b8">{tick.toFixed(1)}</text>
      {/each}

      <!-- raw score line -->
      {#if chartPath()}
        <path d={chartPath()} fill="none" stroke="#94a3b8" stroke-width="1" opacity="0.6" />
      {/if}

      <!-- rolling avg line -->
      {#if rollingAvgPath()}
        <path d={rollingAvgPath()} fill="none" stroke="#3b82f6" stroke-width="2" stroke-linejoin="round" />
      {/if}

      <!-- data points colored by FTR band -->
      {#each ftrPoints as p, i}
        {@const cx = toSvgX(i, ftrPoints.length)}
        {@const cy = toSvgY(p.y)}
        {@const dotColor = p.y >= 0.8 ? '#16a34a' : p.y >= 0.5 ? '#d97706' : '#dc2626'}
        <circle cx={cx} cy={cy} r="3" fill={dotColor}>
          <title>FTR {p.y.toFixed(3)} — {new Date(p.label).toLocaleDateString()}</title>
        </circle>
      {/each}

      <!-- x-axis label -->
      <text x={PAD.left} y={H - 4} font-size="9" fill="#94a3b8">oldest</text>
      <text x={W - PAD.right} y={H - 4} text-anchor="end" font-size="9" fill="#94a3b8">newest</text>
    </svg>
    <p class="chart-legend">
      <span style="color:#94a3b8">— raw score</span>
      &nbsp;&nbsp;
      <span style="color:#3b82f6">— 7-session rolling avg</span>
      &nbsp;&nbsp;
      <span style="color:#fbbf24">— 0.7 threshold</span>
    </p>
  </div>
{/if}

<h2>Tool Usage</h2>
{#if data.toolUsage.length === 0}
  <p>No tool calls recorded yet. Start a session to see usage.</p>
{:else}
  <Table data={toolRows} columns={toolColumns} />
{/if}

<h2>Task Sessions ({data.sessions.length})</h2>
{#if data.sessions.length === 0}
  <p>No task sessions recorded yet. Pass <code>task_description</code> to <code>get_session_context</code> to start tracking.</p>
{:else}
  <div class="session-list">
    {#each data.sessions as session (session.id)}
      <div class="session-row">
        <div class="session-desc">{truncate(session.taskDescription)}</div>
        <div class="session-meta">
          {#if session.taskType}
            <span class="badge badge-type">{session.taskType}</span>
          {/if}
          <span class="badge badge-status-{session.status}">{session.status}</span>
          <span class="ftr {ftrColor(session.ftrScore)}">
            FTR {session.ftrScore !== null ? session.ftrScore.toFixed(3) : '—'}
          </span>
          <span class="ts">{fmt(session.createdAt)}</span>
        </div>
      </div>
    {/each}
  </div>
{/if}

{#if Object.keys(data.costBySession).length > 0}
<h2>Cost — Last 30 Days</h2>
<table class="cost-table">
  <thead>
    <tr>
      <th>Session</th>
      <th class="num">Input tokens</th>
      <th class="num">Output tokens</th>
      <th class="num">Cache read</th>
      <th class="num">Cache write</th>
      <th class="num">Cost (USD)</th>
    </tr>
  </thead>
  <tbody>
    {#each data.sessions as session}
      {#if data.costBySession[session.id]}
        {@const cost = data.costBySession[session.id]}
        <tr>
          <td class="desc">{truncate(session.taskDescription)}</td>
          <td class="num mono">{cost.inputTokens.toLocaleString()}</td>
          <td class="num mono">{cost.outputTokens.toLocaleString()}</td>
          <td class="num mono">{cost.cacheReadTokens.toLocaleString()}</td>
          <td class="num mono">{cost.cacheCreationTokens.toLocaleString()}</td>
          <td class="num mono">${cost.costUsd.toFixed(4)}</td>
        </tr>
      {/if}
    {/each}
  </tbody>
</table>
{/if}

{#if data.benchmarkPairs.length > 0}
<h2>Benchmark Comparison</h2>
<table class="cost-table">
  <thead>
    <tr>
      <th>Task</th>
      <th>Branch</th>
      <th class="num">Without sensei</th>
      <th class="num">With sensei</th>
      <th class="num">Savings ($)</th>
      <th class="num">Savings (%)</th>
    </tr>
  </thead>
  <tbody>
    {#each data.benchmarkPairs as pair}
      {@const savings = (pair.withoutSensei?.costUsd ?? 0) - (pair.withSensei?.costUsd ?? 0)}
      {@const savingsPct = pair.withoutSensei?.costUsd ? (savings / pair.withoutSensei.costUsd) * 100 : 0}
      <tr>
        <td class="desc">{truncate(pair.taskDescription)}</td>
        <td class="mono" style="font-size:0.75rem">{pair.branch}</td>
        <td class="num mono">${pair.withoutSensei?.costUsd.toFixed(4)}</td>
        <td class="num mono">${pair.withSensei?.costUsd.toFixed(4)}</td>
        <td class="num mono" class:savings-pos={savings > 0} class:savings-neg={savings <= 0}>${savings.toFixed(4)}</td>
        <td class="num" class:savings-pos={savingsPct > 0} class:savings-neg={savingsPct <= 0}>{savingsPct.toFixed(1)}%</td>
      </tr>
    {/each}
  </tbody>
</table>
{/if}

<style>
  h2 { margin: 24px 0 12px; font-size: 1rem; font-weight: 600; color: #374151; }
  .ftr-summary-row { display: flex; gap: 24px; margin-bottom: 12px; }
  .ftr-stat { display: flex; flex-direction: column; gap: 2px; }
  .ftr-stat-label { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; color: #94a3b8; font-weight: 600; }
  .ftr-stat-value { font-size: 1.125rem; font-weight: 700; font-family: monospace; color: #1e293b; }
  .chart-wrap { background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 12px 16px 4px; display: inline-block; }
  .chart-legend { font-size: 0.7rem; color: #94a3b8; margin: 4px 0 0; }
  .session-list { display: flex; flex-direction: column; gap: 8px; }
  .session-row { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border: 1px solid #e2e8f0; border-radius: 8px; background: #f8fafc; }
  .session-desc { font-size: 0.875rem; color: #1e293b; flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .session-meta { display: flex; align-items: center; gap: 10px; flex-shrink: 0; margin-left: 12px; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: 0.7rem; font-weight: 600; text-transform: uppercase; }
  .badge-type { background: #ede9fe; color: #5b21b6; }
  .badge-status-completed { background: #f1f5f9; color: #475569; }
  .badge-status-in_progress { background: #dbeafe; color: #1d4ed8; }
  .badge-status-abandoned { background: #fee2e2; color: #991b1b; }
  .ftr { font-size: 0.75rem; font-weight: 600; font-family: monospace; }
  .ftr-high { color: #166534; }
  .ftr-mid  { color: #92400e; }
  .ftr-low  { color: #991b1b; }
  .ftr-null { color: #94a3b8; }
  .ts { font-size: 0.75rem; color: #94a3b8; white-space: nowrap; }
  .cost-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; margin-bottom: 8px; }
  .cost-table th { text-align: left; padding: 6px 12px 6px 0; border-bottom: 2px solid #e2e8f0; color: #64748b; font-weight: 600; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.03em; }
  .cost-table td { padding: 8px 12px 8px 0; border-bottom: 1px solid #f1f5f9; }
  .cost-table tr:hover td { background: #f8fafc; }
  .cost-table .num { text-align: right; }
  .cost-table .mono { font-family: monospace; }
  .cost-table .desc { max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .savings-pos { color: #166534; }
  .savings-neg { color: #991b1b; }
</style>
