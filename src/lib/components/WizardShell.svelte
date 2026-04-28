<script lang="ts">
  import type { Stage } from '$lib/stage.js';

  let {
    stages,
    current = $bindable(0),
    onDone,
  }: {
    stages: Stage[];
    current: number;
    onDone?: () => void;
  } = $props();

  const stage = $derived(stages[current]);
  const isFirst = $derived(current === 0);
  const isLast = $derived(current === stages.length - 1);
  const canGoNext = $derived(stage?.canAdvance() ?? false);

  function next() {
    if (isLast && onDone) { onDone(); return; }
    if (canGoNext && current < stages.length - 1) current++;
  }

  function back() {
    if (current > 0) current--;
  }

  // Run stage load() on enter
  $effect(() => {
    const s = stages[current];
    if (s?.load) s.load();
    if (s?.source) {
      const unsub = s.source.subscribe(() => {}); // keep connection alive
      return unsub;
    }
  });
</script>

<div class="wizard-shell">
  <!-- Rail (left sidebar) -->
  <nav class="rail">
    {#each stages as s, i (s.id)}
      <button
        class="rail-item"
        class:active={i === current}
        class:done={i < current}
        onclick={() => { if (i <= current) current = i; }}
        disabled={i > current}
      >
        <span class="rail-icon kanji">{s.icon}</span>
        <span class="rail-label">{s.title}</span>
      </button>
    {/each}
  </nav>

  <!-- Content area -->
  <div class="stage-area">
    <div class="stage-content">
      {#if stage?.watermark}
        <span class="watermark kanji">{stage.icon}</span>
      {/if}

      <div class="stage-header">
        <span class="kanji stage-icon">{stage?.icon}</span>
        <div>
          <h2 class="display stage-title">{stage?.title}</h2>
          <p class="stage-desc">{stage?.description}</p>
        </div>
      </div>

      <div class="stage-body">
        {#if stage}
          {@const Comp = stage.component}
          <Comp />
        {/if}
      </div>
    </div>

    <!-- Bottom nav -->
    <div class="stage-nav">
      <div class="nav-progress">
        {current + 1} / {stages.length}
      </div>
      <div class="nav-buttons">
        {#if !isFirst}
          <button class="btn-outline" onclick={back}>Back</button>
        {/if}
        <button class="btn-solid" onclick={next} disabled={!canGoNext}>
          {isLast ? 'Done' : 'Continue'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .wizard-shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100vh;
    background: var(--paper);
    font-family: var(--font-ui);
    color: var(--sumi);
  }

  /* ── Rail ──────────────────────────────────────── */
  .rail {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 52px 16px 16px;
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
  .stage-area {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .stage-content {
    flex: 1;
    overflow-y: auto;
    padding: 48px 64px 32px;
    position: relative;
  }

  .stage-header {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 32px;
  }

  .stage-icon {
    font-size: 32px;
    color: var(--shu);
    opacity: 0.5;
    margin-top: 2px;
  }

  .stage-title {
    font-size: 24px;
    font-weight: 300;
    margin: 0 0 4px;
    letter-spacing: -0.01em;
  }

  .stage-desc {
    font-size: 13px;
    color: var(--sumi-3);
    margin: 0;
  }

  .stage-body {
    position: relative;
    z-index: 1;
  }

  .watermark {
    position: absolute;
    right: 64px;
    bottom: 32px;
    font-size: 220px;
    color: var(--shu);
    opacity: 0.035;
    line-height: 1;
    user-select: none;
    pointer-events: none;
    z-index: 0;
  }

  /* ── Bottom nav ───────────────────────────────── */
  .stage-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 64px;
    border-top: var(--hairline);
    flex-shrink: 0;
  }

  .nav-progress {
    font-size: 11px;
    color: var(--sumi-4);
    font-family: var(--font-mono);
  }

  .nav-buttons {
    display: flex;
    gap: 8px;
  }
</style>
