<script lang="ts">
  import type { PageData } from './$types';
  const { data } = $props();
</script>

<h1>Benchmark Reports</h1>

{#if data.reports.length === 0}
  <p>No benchmark runs yet.</p>
{:else}
  <table>
    <thead>
      <tr>
        <th>Run</th>
        <th>Strategy</th>
        <th>Score</th>
        <th>Tokens</th>
        <th>Time (ms)</th>
        <th>Promoted</th>
        <th>Created</th>
      </tr>
    </thead>
    <tbody>
      {#each data.reports as r}
        <tr class:promoted={r.promoted}>
          <td>{r.run_name}</td>
          <td>{r.strategy}</td>
          <td>{r.score ?? '—'}</td>
          <td>{r.tokens ?? '—'}</td>
          <td>{r.elapsed_ms ?? '—'}</td>
          <td>{r.promoted ? '★' : ''}</td>
          <td>{new Date(r.created_at).toLocaleDateString()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  tr.promoted { background: var(--color-success-soft, #f0fdf4); }
</style>
