<script lang="ts">
  import type { WizStage } from '../types.js';
  import { getPort } from '$lib/appstate.svelte.js';

  let { stages, currentIndex, onNavigate, onExit }: {
    stages: WizStage[];
    currentIndex: number;
    onNavigate: (i: number) => void;
    onExit: () => void;
  } = $props();

  const port = $derived(getPort());
</script>

<aside class="rail">
  <!-- Logo + ESC -->
  <div class="rail-header">
    <span class="kanji logo-kanji">先</span>
    <span class="display logo-text">Sensei</span>
    <span class="spacer"></span>
    <button class="esc-btn" onclick={onExit} title="Exit setup">ESC</button>
  </div>

  <!-- SETUP label -->
  <div class="section-label">Setup</div>

  <!-- Stage list -->
  <div class="stage-list">
    {#each stages as stage, i}
      {@const isCurrent = i === currentIndex}
      {@const isDone = i < currentIndex}
      {@const isFuture = i > currentIndex}
      <button
        class="stage-item"
        class:current={isCurrent}
        class:done={isDone}
        class:future={isFuture}
        onclick={() => onNavigate(i)}
        disabled={isFuture}
      >
        <span class="stage-icon" class:icon-current={isCurrent} class:icon-done={isDone} class:icon-future={isFuture}>
          {#if isDone}
            <span class="checkmark">&#10003;</span>
          {:else}
            <span class="kanji">{stage.n}</span>
          {/if}
        </span>
        <div class="stage-info">
          <div class="stage-title">{stage.title}</div>
          {#if isCurrent}
            <div class="stage-sub">{stage.sub}</div>
          {/if}
        </div>
      </button>
    {/each}
  </div>

  <div class="spacer"></div>

  <!-- Footer -->
  <div class="rail-footer">
    <div class="footer-daemon">daemon &middot; {port}</div>
    <div class="footer-note">setup can be resumed at any time from <span class="footer-mono">Settings</span></div>
  </div>
</aside>

<style>
  .rail {
    border-right: var(--hairline);
    padding: 26px 22px;
    display: flex;
    flex-direction: column;
    background: var(--paper-2);
    overflow: hidden;
  }

  .rail-header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 28px;
  }

  .logo-kanji {
    font-size: 22px;
    color: var(--shu);
  }

  .logo-text {
    font-size: 17px;
    color: var(--sumi);
  }

  .spacer {
    flex: 1;
  }

  .esc-btn {
    font-size: 11px;
    color: var(--sumi-3);
    letter-spacing: 0.1em;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: var(--font-mono);
  }

  .esc-btn:hover {
    color: var(--sumi-2);
  }

  .section-label {
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--sumi-3);
    text-transform: uppercase;
    margin-bottom: 14px;
  }

  .stage-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .stage-item {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 10px;
    padding: 7px 10px;
    border-radius: var(--radius);
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    color: var(--sumi-2);
    cursor: pointer;
    transition: all 0.14s;
    font-family: var(--font-ui);
  }

  .stage-item:disabled {
    cursor: default;
  }

  .stage-item.current {
    padding: 10px;
    background: var(--paper);
    border: var(--hairline);
    color: var(--sumi);
  }

  .stage-item.future {
    color: var(--sumi-4);
  }

  .stage-item.done:hover {
    background: var(--paper);
  }

  .stage-icon {
    font-size: 14px;
    text-align: center;
    line-height: 1.4;
  }

  .icon-current {
    color: var(--shu);
  }

  .icon-done {
    color: var(--jade);
  }

  .icon-future {
    color: var(--sumi-4);
  }

  .checkmark {
    font-size: 13px;
  }

  .stage-info {
    overflow: hidden;
  }

  .stage-title {
    font-size: 13px;
    line-height: 1.4;
  }

  .stage-sub {
    font-size: 10px;
    color: var(--sumi-3);
    margin-top: 2px;
    font-family: var(--font-mono);
  }

  .rail-footer {
    font-size: 10px;
    color: var(--sumi-3);
    line-height: 1.6;
    border-top: var(--hairline);
    padding-top: 14px;
  }

  .footer-daemon {
    font-family: var(--font-mono);
  }

  .footer-note {
    margin-top: 4px;
  }

  .footer-mono {
    font-family: var(--font-mono);
  }
</style>
