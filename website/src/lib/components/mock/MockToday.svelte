<script lang="ts">
  import AppFrame from './AppFrame.svelte';
  import MockSidebar from './MockSidebar.svelte';

  let { width = 720, height = 460 } = $props<{ width?: number; height?: number }>();
</script>

<AppFrame title="Sensei · Today" {width} {height}>
  <MockSidebar active="home" />
  <main class="content">
    <div class="date-label">Tuesday, March 12</div>
    <h1 class="display greeting">Good morning, Aiko.</h1>

    <div class="hero-observation">
      <div class="obs-left">
        <div class="obs-header">
          <span class="kanji" style="font-size: 22px; color: var(--shu);">繰</span>
          <span class="obs-tag">Pattern recurring</span>
        </div>
        <div class="display obs-text">
          You've reached for <em>useEffect</em> three times this week
          when state could've stayed local. Worth a closer look?
        </div>
      </div>
      <div class="obs-right">
        <div class="mono obs-count">3×</div>
        <div class="obs-period">this week</div>
      </div>
    </div>

    <div class="columns">
      <div>
        <div class="col-label">Also worth noticing</div>
        <div class="notice-list">
          {#each [
            { k: '結', l: 'Refactor compounding well', d: 'kazoku-app' },
            { k: '問', l: 'Tests skipped 4 sessions', d: 'shoji-ui' },
            { k: '灯', l: 'New idiom emerging in shoji-ui', d: 'shoji-ui' },
          ] as it, i}
            <div class="notice-item" class:bordered={i < 2}>
              <span class="kanji" style="font-size: 13px; color: var(--sumi-2);">{it.k}</span>
              <span class="notice-text">{it.l}</span>
              <span class="mono notice-project">{it.d}</span>
            </div>
          {/each}
        </div>
      </div>
      <div>
        <div class="col-label">Adopted teachings</div>
        <div class="teachings">
          <div>· Prefer local state</div>
          <div>· Co-locate tests</div>
          <div>· Keep async at edges</div>
          <div class="faded">+ 21 more</div>
        </div>
      </div>
    </div>
  </main>
</AppFrame>

<style>
  .content {
    flex: 1;
    padding: 28px 36px;
    overflow: hidden;
    background: var(--paper);
  }
  .date-label {
    font-size: 9.5px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .greeting {
    font-size: 24px;
    font-weight: 400;
    margin: 6px 0 18px;
    letter-spacing: -0.01em;
  }
  .hero-observation {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 24px;
    align-items: start;
    padding-bottom: 18px;
    border-bottom: var(--hairline);
  }
  .obs-header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 6px;
  }
  .obs-tag {
    font-size: 9.5px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .obs-text {
    font-size: 17px;
    line-height: 1.4;
    color: var(--sumi);
    max-width: 380px;
  }
  .obs-right {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-end;
    color: var(--sumi-3);
    font-size: 10px;
  }
  .obs-count { font-size: 18px; color: var(--sumi); }
  .obs-period { font-size: 10px; }
  .columns {
    display: grid;
    grid-template-columns: 1.4fr 1fr;
    gap: 24px;
    margin-top: 22px;
  }
  .col-label {
    font-size: 9px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 10px;
  }
  .notice-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .notice-item {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 10px;
    align-items: baseline;
    padding: 8px 0;
  }
  .notice-item.bordered {
    border-bottom: var(--ink-line);
  }
  .notice-text { font-size: 11.5px; color: var(--sumi); }
  .notice-project { font-size: 9px; color: var(--sumi-3); }
  .teachings {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 11px;
    color: var(--sumi-2);
  }
  .faded { color: var(--sumi-4); }
</style>
