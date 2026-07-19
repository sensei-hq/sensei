<script lang="ts">
  import AppFrame from './AppFrame.svelte';
  import MockSidebar from './MockSidebar.svelte';

  let { width = 720, height = 460 } = $props<{ width?: number; height?: number }>();

  const tabs = [
    { k: '具', l: 'Playground', on: true },
    { k: '録', l: 'Replay', on: false },
    { k: '健', l: 'Health', on: false },
  ];
  const mcps = ['filesystem', 'git', 'shell', 'search', 'sensei'];
  const tools = [
    { name: 'fs.read', kind: 'query' },
    { name: 'fs.write', kind: 'action' },
    { name: 'git.log', kind: 'query' },
    { name: 'git.diff', kind: 'query' },
    { name: 'shell.run', kind: 'action' },
    { name: 'search.code', kind: 'query' },
    { name: 'test.run', kind: 'action' },
    { name: 'pattern.promote', kind: 'action' },
  ];
</script>

<AppFrame title="Sensei · Instruments" {width} {height}>
  <MockSidebar active="instruments" />
  <main class="content">
    <div class="tabs">
      {#each tabs as t}
        <div class="tab" class:active={t.on}>
          <span class="kanji text-xs" style="color: {t.on ? 'var(--shu)' : 'var(--sumi-3)'}">
            {t.k}
          </span>
          <span class="text-xs" style="color: {t.on ? 'var(--sumi)' : 'var(--sumi-3)'}">
            {t.l}
          </span>
        </div>
      {/each}
    </div>

    <div class="body">
      <div class="desc">
        What can these tools do? Try them in isolation; sensei watches
        usage in the background.
      </div>

      <div class="mcp-chooser">
        {#each mcps as m, i}
          <div class="mcp-chip" class:selected={i === 0}>{m}</div>
        {/each}
      </div>

      <div class="tool-grid">
        {#each tools as t}
          <div class="tool-row">
            <span class="mono tool-kind" class:action={t.kind === 'action'}>
              {t.kind}
            </span>
            <span class="mono tool-name">{t.name}</span>
            <span class="tool-arrow">try →</span>
          </div>
        {/each}
      </div>
    </div>
  </main>
</AppFrame>

<style>
  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--paper);
  }
  .tabs {
    display: flex;
    gap: 0;
    padding: 12px 24px 0;
    border-bottom: var(--hairline);
  }
  .tab {
    padding: 8px 16px;
    display: flex;
    align-items: baseline;
    gap: 6px;
    border-bottom: 1.5px solid transparent;
    margin-bottom: -1px;
  }
  .tab.active {
    border-bottom-color: var(--shu);
  }
  .body {
    flex: 1;
    padding: 16px 24px;
    overflow: hidden;
  }
  .desc {
    font-size: var(--text-xs);
    color: var(--sumi-2);
    margin-bottom: 14px;
    max-width: 480px;
  }
  .mcp-chooser {
    display: flex;
    gap: 6px;
    margin-bottom: 16px;
  }
  .mcp-chip {
    padding: 4px 8px;
    font-size: var(--text-xs);
    border-radius: 4px;
    border: var(--hairline);
    color: var(--sumi-2);
  }
  .mcp-chip.selected {
    background: var(--paper-3);
    color: var(--sumi);
  }
  .tool-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .tool-row {
    padding: 8px 8px;
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 8px;
    align-items: center;
    font-size: var(--text-xs);
    border-bottom: var(--ink-line);
  }
  .tool-kind {
    font-size: var(--text-xs);
    padding: 4px 4px;
    border-radius: 2px;
    background: var(--paper-3);
    color: var(--sumi-3);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .tool-kind.action {
    background: var(--shu-soft);
    color: var(--shu);
  }
  .tool-name { color: var(--sumi); }
  .tool-arrow { font-size: var(--text-xs); color: var(--sumi-3); }
</style>
