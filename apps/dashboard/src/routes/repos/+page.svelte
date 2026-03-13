<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();
</script>

<h1>Repos</h1>

{#if data.repos.length === 0}
  <p>No repos indexed yet. Run <code>sensei index</code> to get started.</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>Name</th>
        <th>Stack</th>
        <th>Branch</th>
        <th>Last Indexed</th>
        <th>Commit</th>
      </tr>
    </thead>
    <tbody>
      {#each data.repos as repo}
        <tr>
          <td><strong>{repo.name}</strong>{#if repo.description}<br><small>{repo.description}</small>{/if}</td>
          <td>{repo.stack?.join(', ') ?? '—'}</td>
          <td>{repo.default_branch ?? '—'}</td>
          <td>{repo.last_indexed_at ? new Date(repo.last_indexed_at).toLocaleString() : '—'}</td>
          <td><code>{repo.last_indexed_commit?.slice(0, 7) ?? '—'}</code></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
