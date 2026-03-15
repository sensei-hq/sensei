<!-- apps/dashboard/src/routes/repos/[id]/libraries/+page.svelte -->
<script lang="ts">
  import type { PageData, ActionData } from './$types';
  const { data, form }: { data: PageData; form: ActionData } = $props();

  const freshnessColor: Record<string, string> = {
    fresh: 'status-fresh', stale: 'status-stale', missing: 'status-missing',
  };
  const freshnessLabel: Record<string, string> = {
    fresh: 'Fresh', stale: 'Stale', missing: 'Missing',
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
        <th>Freshness</th>
        {#if data.hasAnthropicKey}<th>Skill</th>{/if}
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
            <td class={lib.skill ? 'skill-generated' : ''}>{lib.skill ? 'Generated' : 'None'}</td>
          {/if}
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <p>No library docs indexed yet.</p>
  <p><code>Add custom_libs to .sensei/config.yaml then run sensei update-registry</code></p>
{/if}

<form method="POST" action="?/update">
  <button type="submit">Update Registry</button>
</form>

<style>
  .status-fresh   { color: green; }
  .status-stale   { color: goldenrod; }
  .status-missing { color: red; }
  .skill-generated { color: green; }
  .error { color: red; }
  table { border-collapse: collapse; width: 100%; }
  th, td { padding: 0.5rem 1rem; text-align: left; border-bottom: 1px solid #eee; }
</style>
