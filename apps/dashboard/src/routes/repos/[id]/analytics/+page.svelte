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
</style>
