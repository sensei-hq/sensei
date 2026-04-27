<script lang="ts">
  import type { WizardState, WizUpdate, WizStage } from '../types.js';
  import { MOCK_MCPS } from '../mock.js';
  import StepHeader from './StepHeader.svelte';

  let { wizState, update, stage }: {
    wizState: WizardState;
    update: WizUpdate;
    stage: WizStage;
  } = $props();

  const mcps = MOCK_MCPS;

  const recommended = $derived(mcps.filter(m => m.recommended));
  const alsoAvailable = $derived(mcps.filter(m => !m.recommended));
  const installCount = $derived(
    mcps.filter(m => wizState.mcps[m.id]).length
  );

  function toggle(id: string) {
    update({ mcps: { ...wizState.mcps, [id]: !wizState.mcps[id] } });
  }

  const stack = $derived(wizState.detectedStack);
  const stackChips = $derived([
    ...stack.languages,
    ...stack.frameworks,
    ...stack.runtimes,
    ...stack.services,
  ]);
</script>

<section class="step">
  <StepHeader {stage} subtitle="Sensei recommends these based on what it detected in your stack. Each one brings its own tools — no wrapping needed." />

  <!-- Detected stack -->
  <div class="section-heading">Detected in your stack</div>
  <div class="stack-row">
    {#each stackChips as chip}
      <span class="stack-chip">{chip}</span>
    {/each}
  </div>

  <!-- Install count chip -->
  <div class="install-count">
    <span class="count-chip">{installCount} MCPs to be installed</span>
  </div>

  <!-- Recommended section -->
  <div class="section-heading">Recommended for your stack</div>
  <div class="mcp-list">
    {#each recommended as mcp}
      {@const enabled = !!wizState.mcps[mcp.id]}
      <div class="mcp-card">
        <div class="mcp-icon-wrap">
          <span class="kanji mcp-kanji">{mcp.kanji}</span>
        </div>

        <div class="mcp-info">
          <div class="mcp-name-row">
            <span class="mcp-name">{mcp.name}</span>
            <span class="publisher-badge">{mcp.publisher}</span>
            {#if mcp.verified}
              <span class="verified-mark" title="Verified">&#10003;</span>
            {/if}
          </div>
          <div class="mcp-summary">{mcp.summary}</div>
        </div>

        <div class="mcp-actions">
          <span class="tool-count">{mcp.tools} tools</span>
          {#if mcp.installed}
            <span class="installed-badge">installed</span>
          {:else}
            <button class="toggle-switch" class:toggle-on={enabled} onclick={() => toggle(mcp.id)} aria-label="Toggle {mcp.name}">
              <span class="toggle-thumb"></span>
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <!-- Also available section -->
  {#if alsoAvailable.length > 0}
    <div class="section-heading also-heading">Also available</div>
    <div class="mcp-list">
      {#each alsoAvailable as mcp}
        {@const enabled = !!wizState.mcps[mcp.id]}
        <div class="mcp-card mcp-card-dim">
          <div class="mcp-icon-wrap icon-dim">
            <span class="kanji mcp-kanji">{mcp.kanji}</span>
          </div>

          <div class="mcp-info">
            <div class="mcp-name-row">
              <span class="mcp-name">{mcp.name}</span>
              <span class="publisher-badge">{mcp.publisher}</span>
              {#if mcp.verified}
                <span class="verified-mark" title="Verified">&#10003;</span>
              {/if}
            </div>
            <div class="mcp-summary">{mcp.summary}</div>
          </div>

          <div class="mcp-actions">
            <span class="tool-count">{mcp.tools} tools</span>
            <button class="toggle-switch" class:toggle-on={enabled} onclick={() => toggle(mcp.id)} aria-label="Toggle {mcp.name}">
              <span class="toggle-thumb"></span>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .step {
    max-width: 780px;
  }

  /* ── Section heading ────────────────────────────────────── */
  .section-heading {
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin-bottom: var(--space-3);
  }

  .also-heading {
    margin-top: var(--space-8);
  }

  /* ── Stack chips ────────────────────────────────────────── */
  .stack-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }

  .stack-chip {
    font-size: 12px;
    font-weight: 500;
    padding: 4px 12px;
    border-radius: 4px;
    background: var(--paper-2);
    border: var(--hairline);
    color: var(--sumi-2);
  }

  /* ── Install count ──────────────────────────────────────── */
  .install-count {
    margin-bottom: var(--space-6);
  }

  .count-chip {
    font-size: 12px;
    font-weight: 500;
    padding: 5px 14px;
    border-radius: 20px;
    border: var(--hairline);
    color: var(--sumi-2);
    background: var(--paper);
  }

  /* ── MCP list ───────────────────────────────────────────── */
  .mcp-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .mcp-card {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    background: var(--paper-2);
    border-radius: var(--radius-lg);
    transition: background 0.12s;
  }

  .mcp-card:hover {
    background: var(--paper-3);
  }

  .mcp-card-dim {
    opacity: 0.7;
  }

  .mcp-card-dim:hover {
    opacity: 1;
  }

  /* ── Icon ────────────────────────────────────────────────── */
  .mcp-icon-wrap {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--shu-soft);
    flex-shrink: 0;
  }

  .mcp-kanji {
    font-size: 16px;
    color: var(--shu);
  }

  .icon-dim {
    background: var(--paper-3);
  }

  .icon-dim .mcp-kanji {
    color: var(--sumi-3);
  }

  /* ── Info ─────────────────────────────────────────────────── */
  .mcp-info {
    flex: 1;
    min-width: 0;
  }

  .mcp-name-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .mcp-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--sumi);
  }

  .publisher-badge {
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.04em;
    padding: 1px 7px;
    border-radius: 4px;
    background: var(--paper-3);
    color: var(--sumi-3);
    text-transform: uppercase;
  }

  .verified-mark {
    font-size: 11px;
    color: var(--jade);
    font-weight: 700;
  }

  .mcp-summary {
    font-size: 13px;
    color: var(--sumi-3);
    margin-top: 3px;
    line-height: 1.4;
  }

  /* ── Actions ─────────────────────────────────────────────── */
  .mcp-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .tool-count {
    font-size: 11px;
    color: var(--sumi-3);
    white-space: nowrap;
  }

  .installed-badge {
    font-size: 11px;
    font-weight: 600;
    color: var(--jade);
    padding: 3px 10px;
    border: 1px solid var(--jade);
    border-radius: 20px;
    white-space: nowrap;
  }

  /* ── Toggle switch ──────────────────────────────────────── */
  .toggle-switch {
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: 10px;
    border: none;
    background: var(--paper-edge);
    cursor: pointer;
    padding: 0;
    transition: background 0.16s;
    flex-shrink: 0;
  }

  .toggle-switch.toggle-on {
    background: var(--shu);
  }

  .toggle-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    transition: transform 0.16s;
    pointer-events: none;
  }

  .toggle-on .toggle-thumb {
    transform: translateX(16px);
  }
</style>
