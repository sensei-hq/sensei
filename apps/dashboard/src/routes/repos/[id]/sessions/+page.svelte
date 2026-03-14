<script lang="ts">
  import { Table } from '@rokkit/ui';
  import type { PageData } from './$types';

  const { data } = $props();

  let expandedId = $state<string | null>(null);

  const toggle = (id: string) => { expandedId = expandedId === id ? null : id; };
  const fmt = (iso: string) => new Date(iso).toLocaleString();

  const snapshotColumns = [
    { name: 'kind',            label: 'Kind',     sortable: true },
    { name: 'progressSummary', label: 'Progress', sortable: false },
    { name: 'createdAt',       label: 'Created',  sortable: true },
  ];

  const memoryColumns = [
    { name: 'type',   label: 'Type',   sortable: true },
    { name: 'title',  label: 'Title',  sortable: false },
    { name: 'status', label: 'Status', sortable: true },
  ];

  function snapshotRows(snaps: any[]) {
    return snaps.map(s => ({ ...s, createdAt: fmt(s.createdAt) }));
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Sessions — {data.repo.name}</h1>

{#if data.sessions.length === 0}
  <p>No sessions yet. The MCP server creates one on the first <code>get_session_context</code> call.</p>
{:else}
  <p>{data.sessions.length} session{data.sessions.length !== 1 ? 's' : ''}</p>

  {#each data.sessions as session (session.id)}
    <div class="session">
      <div class="session-header" onclick={() => toggle(session.id)} role="button" tabindex="0"
           onkeydown={(e) => e.key === 'Enter' && toggle(session.id)}>
        <div class="session-title">
          <span class="status status-{session.status}">{session.status}</span>
          <span class="session-id">{session.id.slice(0, 8)}…</span>
        </div>
        <div class="session-meta">
          <span>♥ {fmt(session.lastHeartbeat)}</span>
          <span>{fmt(session.createdAt)}</span>
          <span>{expandedId === session.id ? '▲' : '▼'}</span>
        </div>
      </div>

      {#if expandedId === session.id}
        <div class="session-detail">
          <h3>Snapshots ({session.snapshots.length})</h3>
          {#if session.snapshots.length === 0}
            <p class="empty">No snapshots in this session.</p>
          {:else}
            <Table data={snapshotRows(session.snapshots)} columns={snapshotColumns} />
          {/if}

          <h3>Memory Items ({session.memoryItems.length})</h3>
          {#if session.memoryItems.length === 0}
            <p class="empty">No memory items recorded in this session.</p>
          {:else}
            <Table data={session.memoryItems} columns={memoryColumns} />
          {/if}
        </div>
      {/if}
    </div>
  {/each}
{/if}

<style>
  .session { border: 1px solid #e2e8f0; border-radius: 8px; margin-bottom: 12px; overflow: hidden; }
  .session-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; cursor: pointer; background: #f8fafc; }
  .session-header:hover { background: #f1f5f9; }
  .session-title { display: flex; align-items: center; gap: 12px; }
  .session-meta { display: flex; gap: 16px; color: #64748b; font-size: 0.875rem; }
  .session-detail { padding: 16px; }
  .session-id { font-family: monospace; color: #64748b; font-size: 0.875rem; }
  .status { padding: 2px 8px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; text-transform: uppercase; }
  .status-active { background: #dcfce7; color: #166534; }
  .status-completed { background: #f1f5f9; color: #475569; }
  .status-crashed { background: #fee2e2; color: #991b1b; }
  .empty { color: #94a3b8; font-style: italic; }
  h3 { font-size: 0.875rem; font-weight: 600; color: #374151; margin: 12px 0 8px; }
</style>
