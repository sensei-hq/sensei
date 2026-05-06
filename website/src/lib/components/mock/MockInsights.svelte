<script lang="ts">
  import AppFrame from './AppFrame.svelte';
  import MockSidebar from './MockSidebar.svelte';

  let { width = 720, height = 460 } = $props<{ width?: number; height?: number }>();

  const cards = [
    { kanji: '繰', title: 'useEffect when state stays local', conf: 0.86, projects: 3, ages: '3 weeks' },
    { kanji: '問', title: 'Tests skipped on hot paths', conf: 0.72, projects: 1, ages: 'this week' },
    { kanji: '灯', title: 'Shoji-style composable panels', conf: 0.64, projects: 1, ages: '5 days' },
    { kanji: '結', title: 'Edge-only async, pure cores', conf: 0.91, projects: 4, ages: '2 months' },
  ];
</script>

<AppFrame title="Sensei · Insights" {width} {height}>
  <MockSidebar active="insights" />
  <main class="content">
    <div class="heading">
      <div class="section-tag">Insights · 今</div>
      <h1 class="display title">What sensei has noticed</h1>
    </div>
    <div class="grid">
      {#each cards as c}
        <div class="card">
          <div class="card-header">
            <span class="kanji" style="font-size: 18px; color: var(--shu);">{c.kanji}</span>
            <span class="display card-title">{c.title}</span>
          </div>
          <div class="card-meta">
            <span class="mono">{Math.round(c.conf * 100)}% confident</span>
            <span>{c.projects} projects · {c.ages}</span>
          </div>
          <div class="bar-track">
            <div class="bar-fill" style="width: {c.conf * 100}%;"></div>
          </div>
        </div>
      {/each}
    </div>
  </main>
</AppFrame>

<style>
  .content {
    flex: 1;
    padding: 24px 32px;
    overflow: hidden;
    background: var(--paper);
  }
  .heading { margin-bottom: 20px; }
  .section-tag {
    font-size: 9.5px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .title {
    font-size: 22px;
    font-weight: 400;
    margin: 4px 0 0;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .card {
    padding: 14px 16px;
    background: var(--paper-2);
    border: var(--hairline);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .card-header {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .card-title {
    font-size: 13px;
    color: var(--sumi);
    flex: 1;
  }
  .card-meta {
    display: flex;
    justify-content: space-between;
    font-size: 9.5px;
    color: var(--sumi-3);
  }
  .bar-track {
    height: 2px;
    background: var(--paper-3);
    border-radius: 1px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: var(--shu);
  }
</style>
