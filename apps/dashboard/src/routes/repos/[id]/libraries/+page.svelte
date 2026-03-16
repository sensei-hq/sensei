<!-- apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte -->
<script lang="ts">
  import type { PageData, ActionData } from './$types';
  const { data, form }: { data: PageData; form: ActionData } = $props();

  const freshnessColor: Record<string, string> = {
    fresh: 'status-fresh', stale: 'status-stale', missing: 'status-missing',
  };
  const freshnessLabel: Record<string, string> = {
    fresh: 'Fresh', stale: 'Stale', missing: 'Not indexed',
  };

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    return new Date(iso).toLocaleString();
  }
</script>

<a href="/repos/{data.repo.id}">← {data.repo.name}</a>
<h1>Library Docs</h1>

{#if form?.error}
  <p class="error">{form.error}</p>
{/if}

{#if data.libs.length > 0}
  <table>
    <thead>
      <tr>
        <th>Library</th>
        <th>Source</th>
        <th>Sections</th>
        <th>Last Fetched</th>
        <th>Status</th>
        {#if data.hasAnthropicKey}<th>Skill</th>{/if}
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {#each data.libs as lib}
        <tr>
          <td><a href="/repos/{data.repo.id}/libraries/{lib.libName}">{lib.libName}</a></td>
          <td>{lib.sourceType}</td>
          <td>{lib.sectionCount}</td>
          <td>{formatDate(lib.lastFetched)}</td>
          <td class={freshnessColor[lib.freshness] ?? ''}>{freshnessLabel[lib.freshness] ?? lib.freshness}</td>
          {#if data.hasAnthropicKey}
            <td class={lib.skillPath ? 'skill-generated' : ''}>{lib.skillPath ? 'Generated' : 'None'}</td>
          {/if}
          <td>
            {#if lib.freshness === 'missing'}
              <form method="POST" action="?/add" class="inline-form">
                <input type="hidden" name="name" value={lib.libName} />
                <input type="text" name="url" placeholder="https://example.com/llms.txt" required />
                <button type="submit">Add Docs</button>
              </form>
            {:else}
              <form method="POST" action="?/reindex">
                <input type="hidden" name="name" value={lib.libName} />
                <button type="submit">Re-index</button>
              </form>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No library docs configured yet.</p>
{/if}

<h2>Add Library</h2>
<form method="POST" action="?/add" class="add-form">
  <label>Name <input type="text" name="name" placeholder="my-lib" required /></label>
  <label>URL or path <input type="text" name="url" placeholder="https://example.com/llms.txt" required /></label>
  <button type="submit">Add &amp; Index</button>
</form>

<form method="POST" action="?/update">
  <button type="submit">Re-index All</button>
</form>

<style>
  .status-fresh   { color: green; }
  .status-stale   { color: goldenrod; }
  .status-missing { color: red; }
  .skill-generated { color: green; }
  .error { color: red; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
  .inline-form { display: flex; gap: 0.5rem; align-items: center; }
  .add-form { display: flex; flex-direction: column; gap: 0.5rem; max-width: 400px; margin: 1rem 0; }
</style>
