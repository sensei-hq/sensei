<!-- apps/dashboard/src/routes/repos/[id]/drift/+page.svelte -->
<script lang="ts">
  import type { PageData } from './$types';

  const { data }: { data: PageData } = $props();

  const reasonLabel: Record<string, string> = {
    'code-changed': 'Code changed',
    'doc-changed': 'Doc changed (no code change)',
    'file-deleted': 'File deleted',
    'raw-modified': 'Modified since last index',
  };

  const reasonClass: Record<string, string> = {
    'code-changed': 'badge-warn',
    'doc-changed': 'badge-info',
    'file-deleted': 'badge-error',
    'raw-modified': 'badge-warn',
  };
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Doc Drift — {data.repo.name}</h1>

{#if data.lastIndexedCommit}
  <p class="meta">Last indexed commit: <code>{data.lastIndexedCommit.slice(0, 8)}</code></p>
{/if}
{#if data.repo.last_indexed_at}
  <p class="meta">Last indexed: {new Date(data.repo.last_indexed_at).toLocaleString()}</p>
{/if}

<div class="summary {data.drifted.length === 0 ? 'summary-clean' : 'summary-drift'}">
  {data.summary.split('\n')[0]}
</div>

{#if data.drifted.length > 0}
  <h2>Drifted Documents ({data.drifted.length})</h2>
  <div class="drift-list">
    {#each data.drifted as entry}
      <div class="drift-row">
        <div class="doc-path"><code>{entry.docPath}</code></div>
        <div class="drift-meta">
          <span class="badge {reasonClass[entry.reason] ?? 'badge-warn'}">
            {reasonLabel[entry.reason] ?? entry.reason}
          </span>
          {#if entry.changedFiles && entry.changedFiles.length > 0}
            <span class="changed-files">
              {entry.changedFiles.slice(0, 3).join(', ')}
              {entry.changedFiles.length > 3 ? ` +${entry.changedFiles.length - 3} more` : ''}
            </span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
{:else if data.drifted.length === 0 && !data.summary.includes('No index')}
  <p class="all-clear">All documents are in sync with the codebase.</p>
{/if}

<style>
  h1 { font-size: 1.25rem; font-weight: 600; margin: 8px 0 4px; }
  h2 { font-size: 1rem; font-weight: 600; margin: 24px 0 12px; color: #374151; }
  .meta { font-size: 0.8rem; color: #94a3b8; margin: 2px 0; }
  .summary { padding: 12px 16px; border-radius: 8px; font-size: 0.875rem; margin: 16px 0; }
  .summary-clean { background: #f0fdf4; border: 1px solid #86efac; color: #166534; }
  .summary-drift { background: #fefce8; border: 1px solid #fde68a; color: #92400e; }
  .all-clear { color: #166534; font-size: 0.875rem; }
  .drift-list { display: flex; flex-direction: column; gap: 8px; }
  .drift-row {
    display: flex; justify-content: space-between; align-items: flex-start;
    padding: 10px 14px; border: 1px solid #e2e8f0; border-radius: 8px; background: #f8fafc;
    gap: 12px;
  }
  .doc-path { font-size: 0.825rem; color: #1e293b; flex: 1; min-width: 0; word-break: break-all; }
  .drift-meta { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
  .badge { padding: 2px 8px; border-radius: 10px; font-size: 0.7rem; font-weight: 600; white-space: nowrap; }
  .badge-warn  { background: #fef3c7; color: #92400e; }
  .badge-info  { background: #dbeafe; color: #1d4ed8; }
  .badge-error { background: #fee2e2; color: #991b1b; }
  .changed-files { font-size: 0.75rem; color: #64748b; font-family: monospace; }
</style>
