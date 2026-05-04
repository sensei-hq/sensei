<script lang="ts">
  let { data } = $props();
  let ftr = $derived(Math.round((data.ftrMetrics?.ftr14d ?? 0) * 100));
  let ftrPrev = $derived(Math.round((data.ftrMetrics?.ftr14dPrev ?? 0) * 100));
  let ftrDelta = $derived(ftr - ftrPrev);
</script>

<div class="overview-page">
  <!-- Hero: top recommendation -->
  {#if data.topRecommendation}
    <div class="hero-card">
      <span class="hero-label">Top recommendation</span>
      <p class="hero-title">{data.topRecommendation.title}</p>
      <span class="urgency-badge">{data.topRecommendation.urgency}</span>
    </div>
  {:else}
    <div class="hero-card empty">
      <p class="hint">No pending recommendations.</p>
    </div>
  {/if}

  <!-- Stat blocks -->
  <div class="stats-row">
    <div class="stat-block">
      <span class="stat-value">{ftr}%</span>
      <span class="stat-label">FTR 14d</span>
      {#if ftrDelta !== 0}
        <span class="stat-delta" class:pos={ftrDelta > 0} class:neg={ftrDelta < 0}>
          {ftrDelta > 0 ? '+' : ''}{ftrDelta}%
        </span>
      {/if}
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.ftrMetrics?.sessions7d ?? 0}</span>
      <span class="stat-label">Sessions 7d</span>
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.memoryCount}</span>
      <span class="stat-label">Memories</span>
      {#if data.memoriesPendingShare > 0}
        <span class="stat-badge">{data.memoriesPendingShare} to share</span>
      {/if}
    </div>
    <div class="stat-block">
      <span class="stat-value">{data.repos.length}</span>
      <span class="stat-label">Repos</span>
    </div>
  </div>

  <!-- Recent sessions -->
  {#if data.recentSessions.length > 0}
    <section class="recent-sessions">
      <h3 class="section-title">Recent sessions</h3>
      {#each data.recentSessions as session (session.id)}
        <div class="session-row">
          <span class="session-task">{session.task}</span>
          <span class="session-ftr" class:ftr-pass={session.ftr} class:ftr-fail={session.ftr === false}>
            {session.ftr === true ? '✓' : session.ftr === false ? '✗' : '—'}
          </span>
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .overview-page { padding: 24px; max-width: 800px; }
  .hero-card { background: var(--surface-2); border-radius: 8px; padding: 20px; margin-bottom: 20px; }
  .hero-card.empty { opacity: 0.5; }
  .hero-label { font-size: 11px; opacity: 0.6; display: block; margin-bottom: 6px; }
  .hero-title { font-size: 16px; font-weight: 600; margin: 0 0 8px; }
  .urgency-badge { font-size: 11px; padding: 2px 8px; border-radius: 10px; background: var(--surface-3); }
  .stats-row { display: flex; gap: 16px; margin-bottom: 24px; flex-wrap: wrap; }
  .stat-block { background: var(--surface-2); border-radius: 8px; padding: 16px; min-width: 100px; }
  .stat-value { font-size: 28px; font-weight: 700; display: block; }
  .stat-label { font-size: 11px; opacity: 0.5; display: block; }
  .stat-delta.pos { color: var(--green, green); font-size: 12px; }
  .stat-delta.neg { color: var(--red, red); font-size: 12px; }
  .stat-badge { font-size: 11px; background: var(--shu, #c0392b); color: white; padding: 2px 6px; border-radius: 8px; }
  .section-title { font-size: 13px; font-weight: 600; margin: 0 0 10px; opacity: 0.7; }
  .session-row { display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border); font-size: 13px; }
  .ftr-pass { color: var(--green, green); }
  .ftr-fail { color: var(--red, red); }
  .hint { opacity: 0.5; font-size: 13px; }
</style>
