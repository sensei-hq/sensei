<script lang="ts">
  let { data } = $props();
</script>
<div class="section-page">
  <h2>Sessions</h2>
  <p class="stat">{data.ftrMetrics?.sessions7d ?? 0} in last 7 days</p>
  <ul class="session-list">
    {#each data.sessions as s (s.id)}
      <li class="session-row">
        <span class="task">{s.task}</span>
        <span class="ftr" class:pass={s.ftr} class:fail={s.ftr === false}>{s.ftr === true ? '✓' : s.ftr === false ? '✗' : '—'}</span>
        <span class="date">{new Date(s.startedAt).toLocaleDateString()}</span>
      </li>
    {/each}
  </ul>
</div>
<style>
  .section-page { padding: 24px; }
  .stat { font-size: 13px; opacity: 0.6; margin-bottom: 16px; }
  .session-list { list-style: none; margin: 0; padding: 0; }
  .session-row { display: flex; gap: 12px; padding: 8px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .task { flex: 1; }
  .pass { color: var(--green, green); }
  .fail { color: var(--red, red); }
  .date { opacity: 0.5; font-size: 12px; }
</style>
