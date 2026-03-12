<script lang="ts">
  import type { PageData } from './$types';
  export let data: PageData;
</script>

<h1>Dashboard</h1>

<div class="stats-row">
  <div class="stat-card">
    <div class="stat-value">{data.repoCount}</div>
    <div class="stat-label">Indexed Repos</div>
  </div>
  <div class="stat-card">
    <div class="stat-value">{data.recentEvents.length}</div>
    <div class="stat-label">Recent Events</div>
  </div>
</div>

<section>
  <h2>Recent Activity</h2>
  <table>
    <thead>
      <tr><th>Tool</th><th>Phase</th><th>Time</th></tr>
    </thead>
    <tbody>
      {#each data.recentEvents as event}
        <tr>
          <td>{event.tool}</td>
          <td>{event.phase}</td>
          <td>{new Date(event.ts).toLocaleTimeString()}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</section>

<section>
  <h2>Indexed Repos</h2>
  <ul>
    {#each data.repos as repo}
      <li>
        <strong>{repo.name}</strong>
        {#if repo.stack}<span class="tags">{repo.stack.join(', ')}</span>{/if}
        {#if repo.last_indexed_at}
          — indexed {new Date(repo.last_indexed_at).toLocaleDateString()}
        {/if}
      </li>
    {/each}
  </ul>
</section>
