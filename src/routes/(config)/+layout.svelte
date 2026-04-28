<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { STAGES, stageIndex, nextStagePath, prevStagePath } from './stages.js';

  let { children } = $props();

  const currentIdx = $derived(stageIndex(page.url.pathname));
  const stage = $derived(STAGES[currentIdx]);
  const isFirst = $derived(currentIdx === 0);
  const isLast = $derived(currentIdx === STAGES.length - 1);

  function next() {
    if (isLast) {
      appState.setSetupComplete();
      goto('/observatory');
      return;
    }
    const path = nextStagePath(page.url.pathname);
    if (path) goto(path);
  }

  function back() {
    const path = prevStagePath(page.url.pathname);
    if (path) goto(path);
  }
</script>

<div class="wizard">
  <div class="drag-spacer drag-region"></div>

  <div class="body">
    <!-- Rail -->
    <nav class="rail">
      {#each STAGES as s, i (s.id)}
        <button
          class="rail-item"
          class:active={i === currentIdx}
          class:done={i < currentIdx}
          onclick={() => { if (i <= currentIdx) goto(s.path); }}
          disabled={i > currentIdx}
        >
          <span class="rail-icon kanji">{s.icon}</span>
          <span class="rail-label">{s.title}</span>
        </button>
      {/each}
    </nav>

    <!-- Content -->
    <div class="main">
      <div class="content">
        {#if stage?.watermark}
          <span class="watermark kanji">{stage.icon}</span>
        {/if}
        <div class="content-inner">
          {@render children()}
        </div>
      </div>

      <!-- Bottom nav -->
      <div class="bottom">
        <div class="bottom-progress">
          {currentIdx + 1} / {STAGES.length}
        </div>
        <div class="bottom-buttons">
          {#if !isFirst}
            <button class="btn-outline" onclick={back}>Back</button>
          {/if}
          <button class="btn-solid" onclick={next}>
            {isLast ? 'Enter Observatory' : 'Continue'}
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .wizard {
    width: 100%; height: 100vh;
    display: flex; flex-direction: column;
    background: var(--paper);
    font-family: var(--font-ui);
    color: var(--sumi);
    overflow: hidden;
  }

  .drag-spacer { height: 32px; flex-shrink: 0; }

  .body {
    flex: 1;
    display: grid;
    grid-template-columns: 220px 1fr;
    min-height: 0;
  }

  /* ── Rail ──────────────────────────────────────── */
  .rail {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 20px 16px;
    border-right: var(--hairline);
    background: var(--paper-2);
    overflow-y: auto;
  }

  .rail-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: none;
    background: none;
    border-radius: var(--radius);
    cursor: pointer;
    text-align: left;
    color: var(--sumi-3);
    font-family: var(--font-ui);
    font-size: 13px;
    transition: background 0.12s, color 0.12s;
  }

  .rail-item:hover:not(:disabled) { background: var(--paper-3); }
  .rail-item:disabled { cursor: default; opacity: 0.4; }
  .rail-item.active { color: var(--sumi); background: var(--paper); }
  .rail-item.done { color: var(--sumi-2); }

  .rail-icon { font-size: 18px; color: var(--shu); opacity: 0.6; }
  .rail-item.active .rail-icon { opacity: 1; }
  .rail-label { font-weight: 500; }

  /* ── Content ──────────────────────────────────── */
  .main {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 44px 64px 32px;
    position: relative;
  }

  .content-inner { position: relative; z-index: 1; }

  .watermark {
    position: absolute;
    right: 64px; bottom: 32px;
    font-size: 220px;
    color: var(--shu);
    opacity: 0.035;
    line-height: 1;
    user-select: none;
    pointer-events: none;
    z-index: 0;
  }

  /* ── Bottom ───────────────────────────────────── */
  .bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 64px;
    border-top: var(--hairline);
    flex-shrink: 0;
  }

  .bottom-progress {
    font-size: 11px;
    color: var(--sumi-4);
    font-family: var(--font-mono);
  }

  .bottom-buttons { display: flex; gap: 8px; }
</style>
