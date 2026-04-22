<script lang="ts">
  import type { WizStage } from '../types.js';

  let { stage, stageIndex, total, onBack, onNext, onDone }: {
    stage: WizStage;
    stageIndex: number;
    total: number;
    onBack: () => void;
    onNext: () => void;
    onDone: () => void;
  } = $props();

  const isFirst = $derived(stageIndex === 0);
  const isLast = $derived(stageIndex === total - 1);
  const displayIndex = $derived(String(stageIndex + 1).padStart(2, '0'));
  const segments = $derived(Array.from({ length: total }, (_, i) => i));
</script>

<div class="bottom-bar">
  <!-- Left: counter + stage title -->
  <div class="progress-label">
    <span class="progress-index">{displayIndex}</span>
    <span class="progress-divider">/ {total}</span>
    <span class="progress-title">{stage.title}</span>
  </div>

  <!-- Center: progress segments -->
  <div class="progress-track">
    {#each segments as i}
      <span
        class="progress-segment"
        class:filled={i <= stageIndex}
      ></span>
    {/each}
  </div>

  <!-- Right: back + primary action -->
  <button
    class="back-btn"
    onclick={onBack}
    disabled={isFirst}
    class:back-disabled={isFirst}
  >
    &larr; Back
  </button>

  {#if isLast}
    <button class="cta-btn" onclick={onDone}>
      Enter observatory &rarr;
    </button>
  {:else}
    <button class="cta-btn" onclick={onNext}>
      Continue &rarr;
    </button>
  {/if}
</div>

<style>
  .bottom-bar {
    border-top: var(--hairline);
    padding: 14px 64px;
    display: flex;
    align-items: center;
    gap: 20px;
    background: var(--paper);
  }

  .progress-label {
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .progress-index {
    color: var(--sumi-3);
  }

  .progress-divider {
    color: var(--sumi-4);
  }

  .progress-title {
    margin-left: 12px;
    color: var(--sumi-2);
    text-transform: none;
    letter-spacing: 0;
    font-size: 13px;
  }

  .progress-track {
    flex: 1;
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .progress-segment {
    flex: 1;
    height: 2px;
    border-radius: 1px;
    background: var(--paper-edge);
    transition: background 0.2s;
  }

  .progress-segment.filled {
    background: var(--sumi);
  }

  .back-btn {
    font-size: 12px;
    color: var(--sumi-2);
    padding: 8px 14px;
    background: none;
    border: none;
    cursor: pointer;
    font-family: var(--font-ui);
    white-space: nowrap;
  }

  .back-btn:hover:not(:disabled) {
    color: var(--sumi);
  }

  .back-disabled {
    color: var(--sumi-4);
    cursor: default;
  }

  .cta-btn {
    font-size: 13px;
    background: var(--sumi);
    color: var(--paper);
    padding: 10px 22px;
    border-radius: var(--radius);
    letter-spacing: 0.2px;
    border: none;
    cursor: pointer;
    font-family: var(--font-ui);
    white-space: nowrap;
    transition: opacity 0.14s;
  }

  .cta-btn:hover {
    opacity: 0.88;
  }
</style>
