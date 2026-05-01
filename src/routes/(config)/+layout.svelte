<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { STAGES, stageIndex, nextStagePath, prevStagePath } from './stages.js';
  import { wizardState } from '$lib/wizard-state.svelte.js';
  import type { WizardLoadData } from '$lib/setup/contracts.js';

  let { children, data }: { children: any; data: WizardLoadData } = $props();

  const currentIdx = $derived(stageIndex(page.url.pathname));
  const stage = $derived(STAGES[currentIdx]);
  const isFirst = $derived(currentIdx === 0);
  const isLast = $derived(currentIdx === STAGES.length - 1);
  const total = STAGES.length;
  const canAdvance = $derived(wizardState.canAdvance(stage?.id ?? ''));
  let committing = $state(false);

  onMount(() => {
    wizardState.hydrate(data);
  });

  async function next() {
    if (committing) return;
    if (!canAdvance) return;

    committing = true;
    if (isLast) {
      await wizardState.commitStage('done');
      committing = false;
      goto('/observatory');
      return;
    }
    const ok = await wizardState.commitStage(stage.id);
    committing = false;
    if (ok) {
      const path = nextStagePath(page.url.pathname);
      if (path) goto(path);
    }
  }

  function back() {
    const path = prevStagePath(page.url.pathname);
    if (path) goto(path);
  }

  function exitSetup() {
    goto('/');
  }
</script>

<div class="wizard">
  <div class="drag-spacer drag-region"></div>

  <div class="body">
    <!-- Rail -->
    <aside class="rail">
      <div class="rail-header">
        <span class="kanji rail-logo">先生</span>
        <span class="display rail-brand">Sensei</span>
        <span class="rail-spacer"></span>
        <button class="rail-esc" onclick={exitSetup} title="Exit setup">ESC</button>
      </div>

      <div class="rail-section-label">Setup</div>

      <div class="rail-stages">
        {#each STAGES as s, i (s.id)}
          {@const isCur = i === currentIdx}
          {@const isDone = wizardState.isStageComplete(s.id)}
          {@const isNavigable = isDone || isCur}
          <button
            class="rail-item"
            class:active={isCur}
            class:done={isDone}
            onclick={() => { if (isNavigable) goto(s.path); }}
            disabled={!isNavigable}
          >
            <span class="kanji rail-kanji" class:active={isCur} class:done={isDone}>{s.icon}</span>
            <div class="rail-text">
              <div class="rail-title">{s.title}</div>
              {#if isCur}
                <div class="mono rail-sub">{s.sub}</div>
              {/if}
            </div>
            <span class="rail-tick" class:visible={isDone}>✓</span>
          </button>
        {/each}
      </div>

      <div class="rail-footer">
        <div class="services-status">
          <span class="services-dot"></span>
          <div class="services-text">
            <div class="services-label">Services</div>
            <div class="services-value">all green</div>
          </div>
        </div>
      </div>
    </aside>

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
        <div class="bottom-left">
          <span class="bottom-counter">
            {String(currentIdx + 1).padStart(2, '0')}
            <span class="bottom-counter-dim">/ {total}</span>
          </span>
          <span class="bottom-stage-title">{stage.title}</span>
        </div>

        <div class="bottom-ticks">
          {#each Array(total) as _, i}
            <span class="bottom-tick" class:filled={i <= currentIdx}></span>
          {/each}
        </div>

        <div class="bottom-buttons">
          <button class="btn-back" onclick={back} disabled={isFirst}>
            ← Back
          </button>
          <button class="btn-primary" onclick={next} disabled={!canAdvance || committing}>
            {committing ? 'Saving...' : isLast ? 'Enter observatory →' : 'Continue →'}
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
    grid-template-columns: 260px 1fr;
    min-height: 0;
  }

  /* ── Rail ──────────────────────────────────────── */
  .rail {
    display: flex;
    flex-direction: column;
    padding: 26px 22px;
    border-right: var(--hairline);
    background: var(--paper-2);
    overflow: hidden;
  }

  .rail-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 28px;
  }
  .rail-logo { font-size: 22px; color: var(--shu); }
  .rail-brand { font-size: 17px; }
  .rail-spacer { flex: 1; }
  .rail-esc {
    font-size: 11px;
    color: var(--sumi-3);
    letter-spacing: 0.1em;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: var(--font-ui);
  }
  .rail-esc:hover { color: var(--sumi-2); }

  .rail-section-label {
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 14px;
  }

  .rail-stages {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .rail-item {
    display: grid;
    grid-template-columns: 24px 1fr 14px;
    gap: 10px;
    align-items: center;
    padding: 7px 10px;
    border-radius: 6px;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    color: var(--sumi-4);
    cursor: default;
    transition: all 0.14s;
    font-family: var(--font-ui);
    font-size: 13px;
  }
  .rail-item:not(:disabled) { cursor: pointer; }
  .rail-item:disabled { cursor: default; }
  .rail-item.active {
    padding: 10px;
    background: var(--paper);
    border: var(--hairline);
    color: var(--sumi);
  }
  .rail-item.done { color: var(--sumi-2); }

  .rail-kanji {
    font-size: 14px;
    text-align: center;
    color: var(--sumi-4);
  }
  .rail-kanji.active { color: var(--shu); }
  .rail-kanji.done { color: var(--sumi-2); }

  .rail-text { overflow: hidden; }
  .rail-title { font-size: 13px; }
  .rail-sub { font-size: 10px; color: var(--sumi-3); margin-top: 2px; }

  .rail-tick {
    font-size: 11px;
    text-align: center;
    line-height: 1;
    color: var(--jade);
    opacity: 0;
    transition: opacity 0.14s;
  }
  .rail-tick.visible { opacity: 1; }

  .rail-footer {
    margin-top: auto;
    border-top: var(--hairline);
    padding-top: 12px;
  }

  .services-status {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .services-dot {
    width: 7px; height: 7px;
    border-radius: 4px;
    background: var(--matcha);
    flex-shrink: 0;
  }
  .services-text { font-size: 11px; color: var(--sumi-2); line-height: 1.4; }
  .services-label {
    letter-spacing: 0.1em;
    text-transform: uppercase;
    font-size: 10px;
    color: var(--sumi-3);
  }
  .services-value { margin-top: 2px; }

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
    gap: 20px;
    padding: 14px 64px;
    border-top: var(--hairline);
    background: var(--paper);
    flex-shrink: 0;
  }

  .bottom-left {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .bottom-counter {
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    text-transform: uppercase;
  }
  .bottom-counter-dim { color: var(--sumi-4); }
  .bottom-stage-title {
    font-size: 13px;
    color: var(--sumi-2);
  }

  .bottom-ticks {
    flex: 1;
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .bottom-tick {
    flex: 1;
    height: 2px;
    border-radius: 1px;
    background: var(--paper-edge);
    transition: background 0.2s;
  }
  .bottom-tick.filled { background: var(--sumi); }

  .bottom-buttons { display: flex; gap: 8px; align-items: center; }

  .btn-back {
    font-size: 12px;
    color: var(--sumi-2);
    padding: 8px 14px;
    background: none;
    border: none;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .btn-back:disabled { color: var(--sumi-4); cursor: default; }

  .btn-primary {
    font-size: 13px;
    background: var(--sumi);
    color: var(--paper);
    padding: 10px 22px;
    border-radius: 6px;
    border: none;
    letter-spacing: 0.2px;
    cursor: pointer;
    font-family: var(--font-ui);
  }
  .btn-primary:hover:not(:disabled) { opacity: 0.9; }
  .btn-primary:disabled { background: var(--paper-edge); color: var(--sumi-3); cursor: default; }
</style>
