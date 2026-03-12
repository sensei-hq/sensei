<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>Stats</h1>

<section>
  <h2>Tool Usage</h2>
  <table>
    <thead>
      <tr><th>Tool</th><th>Calls</th><th>Bar</th></tr>
    </thead>
    <tbody>
      {#each data.toolRows as { tool, count }}
        {@const max = data.toolRows[0]?.count ?? 1}
        <tr>
          <td>{tool}</td>
          <td>{count}</td>
          <td>
            <div style="width: {Math.round((count / max) * 200)}px; height: 12px; background: var(--color-primary, #6366f1)"></div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</section>

<section>
  <h2>Sessions</h2>
  <p>{data.sessionCount} recent sessions</p>
  {#if data.gapCount > 0}
    <p class="gap-warning">{data.gapCount} gap session(s) — no sensei context tool used</p>
  {/if}
</section>
