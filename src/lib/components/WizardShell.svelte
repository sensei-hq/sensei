<script lang="ts">
  import type { Stage } from '$lib/types.js';

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
      {@const isActive = i === current}
      {@const isDone = i < current}
      <button
        class="rail-item"
        class:active={isActive}
        class:done={isDone}
        onclick={() => { if (i <= current) current = i; }}
        disabled={i > current}
      >
        <span class="rail-icon kanji">{s.icon}</span>
        <div class="rail-text">
          <span class="rail-label">{s.title}</span>
          {#if isActive}<span class="rail-sub">{s.description}</span>{/if}
        </div>
        <span class="rail-tick" style="opacity: {isDone ? 1 : 0}">✓</span>
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
    background: oklch(var(--color-surface-z1) / 1);
    font-family: var(--font-ui);
    color: oklch(var(--color-surface-z9) / 1);
  }

  /* ── Rail ──────────────────────────────────────── */
  .rail {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 52px 16px 16px;
    border-right: var(--hairline);
    background: oklch(var(--color-surface-z2) / 1);
    overflow-y: auto;
  }

  .rail-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 7px 10px;
    border: 1px solid transparent;
    background: none;
    border-radius: var(--radius);
    cursor: pointer;
    text-align: left;
    color: oklch(var(--color-surface-z7) / 1);
    font-family: var(--font-ui);
    font-size: 13px;
    transition: background 0.12s, color 0.12s;
  }

  .rail-item:hover:not(:disabled) { background: oklch(var(--color-surface-z3) / 1); }
  .rail-item:disabled { cursor: default; opacity: 0.4; }
  .rail-item.active { color: oklch(var(--color-surface-z9) / 1); background: oklch(var(--color-surface-z1) / 1); padding: 10px 10px; border-color: oklch(var(--color-surface-z9) / 0.08); }
  .rail-item.done { color: oklch(var(--color-surface-z8) / 1); }

  .rail-icon { font-size: 14px; color: oklch(var(--color-primary-z6) / 1); opacity: 0.6; flex-shrink: 0; margin-top: 1px; }
  .rail-item.active .rail-icon { opacity: 1; }
  .rail-text { flex: 1; overflow: hidden; }
  .rail-label { font-weight: 500; display: block; line-height: 1.3; }
  .rail-sub { display: block; font-size: 10px; font-family: var(--font-mono); color: oklch(var(--color-surface-z7) / 1); margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .rail-tick { font-size: 11px; line-height: 1; color: oklch(var(--color-success-z6) / 1); flex-shrink: 0; margin-top: 2px; transition: opacity 0.14s; }

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
    color: oklch(var(--color-primary-z6) / 1);
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
    color: oklch(var(--color-surface-z7) / 1);
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
    color: oklch(var(--color-primary-z6) / 1);
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
    color: oklch(var(--color-surface-z6) / 1);
    font-family: var(--font-mono);
  }

  .nav-buttons {
    display: flex;
    gap: 8px;
  }
</style>
