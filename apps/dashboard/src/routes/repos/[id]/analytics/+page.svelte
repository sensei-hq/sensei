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

  const toolRows = data.toolUsage.map((t) => ({
    tool: t.tool,
    calls: t.calls,
    successPct: `${Math.round(t.successRate * 100)}%`,
    avgDuration: t.avgDurationMs !== null
      ? (t.avgDurationMs >= 1000 ? `${(t.avgDurationMs / 1000).toFixed(1)}s` : `${t.avgDurationMs}ms`)
      : '—',
  }));
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Analytics — {data.repo.name}</h1>
<p style="color:#64748b;font-size:0.875rem">Last 30 days</p>

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
