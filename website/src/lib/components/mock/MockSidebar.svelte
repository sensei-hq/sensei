<script lang="ts">
  let { active = 'home' } = $props<{ active?: string }>();

  const items = [
    { id: 'home',      kanji: '今', label: 'Today' },
    { id: 'projects',  kanji: '場', label: 'Projects',  badge: 4 },
    { id: 'sessions',  kanji: '録', label: 'Sessions',  badge: 41 },
    { id: 'insights',  kanji: '察', label: 'Insights',  badge: 6 },
    { id: 'memories',  kanji: '覚', label: 'Memories',  badge: 24 },
    { id: 'libraries', kanji: '庫', label: 'Libraries', badge: 14 },
  ];
</script>

<aside class="sidebar">
  <div class="logo">
    <span class="kanji" style="font-size: 16px; color: var(--shu);">先</span>
    <span class="display" style="font-size: 13px;">Sensei</span>
  </div>

  <div class="section">
    <div class="section-label">Observatory</div>
    <div class="nav-items">
      {#each items as it}
        <div class="nav-item" class:active={active === it.id}>
          <span class="kanji item-kanji" style="color: {active === it.id ? 'var(--shu)' : 'var(--sumi-3)'};">
            {it.kanji}
          </span>
          <span class="item-label" style="color: {active === it.id ? 'var(--sumi)' : 'var(--sumi-2)'};">
            {it.label}
          </span>
          {#if it.badge != null}
            <span class="mono item-badge">{it.badge}</span>
          {/if}
        </div>
      {/each}
    </div>
  </div>

  <div class="section">
    <div class="section-label">Active</div>
    <div class="active-projects">
      <div>kazoku-app</div>
      <div>tea-ceremony</div>
      <div>shoji-ui</div>
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: 168px;
    padding: 16px 10px;
    background: var(--paper-2);
    border-right: var(--hairline);
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex-shrink: 0;
  }
  .logo {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 0 6px;
  }
  .section-label {
    font-size: 8.5px;
    letter-spacing: 0.18em;
    color: var(--sumi-3);
    text-transform: uppercase;
    padding: 0 8px 6px;
  }
  .nav-items {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .nav-item {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 5px;
    font-size: 11px;
  }
  .nav-item.active {
    background: var(--paper-3);
  }
  .item-kanji { font-size: 11px; }
  .item-badge { font-size: 8.5px; color: var(--sumi-3); }
  .active-projects {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 0 8px;
    font-size: 10.5px;
    color: var(--sumi-2);
  }
</style>
