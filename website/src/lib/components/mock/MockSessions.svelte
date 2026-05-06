<script lang="ts">
  import AppFrame from './AppFrame.svelte';
  import MockSidebar from './MockSidebar.svelte';

  let { width = 720, height = 460 } = $props<{ width?: number; height?: number }>();

  const lanes = [
    { kanji: '良', title: 'Going well', accent: 'var(--jade)',
      items: ['Compound refactors', 'Naming consistent', 'Clear test boundaries'] },
    { kanji: '破', title: 'Not going well', accent: 'var(--shu)',
      items: ['Tests skipped 4×', 'useEffect overreach', 'PRs sit > 24h'] },
    { kanji: '観', title: 'Insights', accent: 'var(--sumi-2)',
      items: ['New shoji idiom forming', 'Pattern in error handling', 'Tea-ceremony slower start'] },
  ];
</script>

<AppFrame title="Sensei · Sessions" {width} {height}>
  <MockSidebar active="sessions" />
  <main class="content">
    <div class="header">
      <div>
        <div class="section-tag">Sessions · 録</div>
        <h1 class="display title">The week in review</h1>
      </div>
      <div class="mono stats">8 · 5 · 2 · 1h 4m</div>
    </div>

    <div class="sparkline">
      <svg viewBox="0 0 600 60" preserveAspectRatio="none" style="width: 100%; height: 100%;">
        <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10 L 600 60 L 0 60 Z"
              fill="var(--jade-soft)" stroke="none"/>
        <path d="M 0 42 L 60 38 L 120 30 L 180 32 L 240 24 L 300 26 L 360 18 L 420 22 L 480 14 L 540 12 L 600 10"
              fill="none" stroke="var(--jade)" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
    </div>

    <div class="lanes">
      {#each lanes as lane}
        <div>
          <div class="lane-header">
            <span class="kanji" style="font-size: 13px; color: {lane.accent};">{lane.kanji}</span>
            <span class="lane-title">{lane.title}</span>
          </div>
          <div class="lane-items">
            {#each lane.items as item}
              <div class="lane-item" style="border-left-color: {lane.accent}33;">
                {item}
              </div>
            {/each}
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
  .header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 18px;
  }
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
  .stats { font-size: 11px; color: var(--sumi-3); }
  .sparkline {
    height: 60px;
    margin-bottom: 22px;
    position: relative;
  }
  .lanes {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 20px;
  }
  .lane-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 10px;
  }
  .lane-title {
    font-size: 10px;
    letter-spacing: 0.16em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .lane-items {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 11px;
    color: var(--sumi-2);
  }
  .lane-item {
    padding-left: 8px;
    border-left: 2px solid;
  }
</style>
