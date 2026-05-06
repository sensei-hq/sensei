<script lang="ts">
  import AppFrame from './AppFrame.svelte';
  import MockSidebar from './MockSidebar.svelte';

  let { width = 720, height = 460 } = $props<{ width?: number; height?: number }>();
</script>

<AppFrame title="Sensei · Memory" {width} {height}>
  <MockSidebar active="memories" />
  <main class="content">
    <div class="header-row">
      <span class="kanji" style="font-size: 24px; color: var(--shu);">覚</span>
      <span class="section-tag">Memory · adopted</span>
    </div>
    <h1 class="display title">Prefer local component state to lifted state</h1>

    <div class="two-col">
      <div>
        <div class="label">When to apply</div>
        <div class="body-text">
          When state is read by a single component and its direct children,
          keep it local. Lift only when a sibling needs it. Premature
          lifting creates effect chains that are hard to reason about.
        </div>
        <div class="label" style="margin-top: 14px;">Examples sensei watched</div>
        <div class="examples">
          <div>· kazoku-app · Mar 8 · accordion state</div>
          <div>· shoji-ui · Mar 5 · panel collapse</div>
          <div>· tea-ceremony · Feb 28 · form draft</div>
        </div>
      </div>
      <aside class="provenance">
        <div class="label">Provenance</div>
        <div class="prov-list">
          <div><span class="mono prov-key">seen</span> 17 sessions</div>
          <div><span class="mono prov-key">first</span> Feb 14</div>
          <div><span class="mono prov-key">conf</span> 91%</div>
          <div><span class="mono prov-key">by</span> you</div>
        </div>
        <div class="divider"></div>
        <div class="prov-desc">
          Adopted into 4 projects. Sensei surfaces it when local
          state could replace a useEffect chain.
        </div>
      </aside>
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
  .header-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 4px;
  }
  .section-tag {
    font-size: 9.5px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .title {
    font-size: 20px;
    font-weight: 400;
    margin: 0 0 16px;
    letter-spacing: -0.01em;
  }
  .two-col {
    display: grid;
    grid-template-columns: 1.5fr 1fr;
    gap: 24px;
  }
  .label {
    font-size: 9px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 10px;
  }
  .body-text {
    font-size: 12px;
    color: var(--sumi);
    line-height: 1.65;
    margin-bottom: 14px;
  }
  .examples {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 11px;
    color: var(--sumi-2);
  }
  .provenance {
    padding-left: 18px;
    border-left: var(--hairline);
  }
  .prov-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 10.5px;
    color: var(--sumi-2);
  }
  .prov-key { color: var(--sumi-3); }
  .divider {
    height: 1px;
    background: var(--paper-edge);
    margin: 14px 0;
  }
  .prov-desc {
    font-size: 11px;
    color: var(--sumi-2);
    line-height: 1.5;
  }
</style>
