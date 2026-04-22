<script lang="ts">
  import { MOCK_ACPS } from '../mock.js';
  import type { WizardState, WizUpdate } from '../types.js';

  let { wizState, update }: {
    wizState: WizardState;
    update: WizUpdate;
  } = $props();

  function toggle(id: string) {
    update({ acps: { ...wizState.acps, [id]: !wizState.acps[id] } });
  }
</script>

<section class="step">
  <div class="step-label"><span class="kanji">三</span> STEP</div>
  <h1 class="display headline">Assistants</h1>
  <p class="subtitle">Registers plugins, skills, commands, agents, logging and metrics.</p>

  <div class="grid">
    {#each MOCK_ACPS as acp}
      {@const checked = !!wizState.acps[acp.id]}
      {@const found = acp.found}
      <button
        class="card"
        class:card-found={found}
        class:card-missing={!found}
        onclick={() => toggle(acp.id)}
      >
        <div class="card-body">
          <div class="card-top">
            <span class="card-name">{acp.name}</span>
            {#if acp.version}
              <span class="card-version">v{acp.version}</span>
            {/if}
          </div>
          <div class="card-bottom">
            {#if found && acp.path}
              <span class="card-path">{acp.path}</span>
            {:else}
              <span class="card-notfound">not found</span>
            {/if}
          </div>
        </div>
        <div class="checkbox" class:checkbox-checked={checked}>
          {#if checked}
            <span class="check-icon">&#10003;</span>
          {/if}
        </div>
      </button>
    {/each}
  </div>
</section>

<style>
  .step {
    padding: var(--space-10) var(--space-12);
    max-width: 780px;
  }

  .step-label {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    margin-bottom: var(--space-2);
  }

  .step-label .kanji {
    color: var(--shu);
    margin-right: 4px;
  }

  .headline {
    font-size: 40px;
    color: var(--sumi);
    margin: 0 0 var(--space-2) 0;
    line-height: 1.15;
  }

  .subtitle {
    font-size: 15px;
    color: var(--sumi-3);
    margin: 0 0 var(--space-8) 0;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }

  .card {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
    border-radius: var(--radius-lg);
    cursor: pointer;
    text-align: left;
    background: var(--paper);
    border: var(--border-card);
    transition: border-color 0.14s, opacity 0.14s;
    font-family: var(--font-ui);
  }

  .card-found {
    border-color: var(--sumi-4);
  }

  .card-found:hover {
    border-color: var(--sumi-4);
  }

  .card-missing {
    opacity: 0.55;
  }

  .card-missing:hover {
    opacity: 0.7;
  }

  .card-body {
    flex: 1;
    min-width: 0;
  }

  .card-top {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }

  .card-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--sumi);
  }

  .card-version {
    font-size: 12px;
    color: var(--sumi-3);
    font-family: var(--font-mono);
  }

  .card-bottom {
    margin-top: 4px;
  }

  .card-path {
    font-size: 12px;
    color: var(--sumi-3);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .card-notfound {
    font-size: 12px;
    color: var(--sumi-4);
    font-style: italic;
  }

  .checkbox {
    width: 22px;
    height: 22px;
    border-radius: 4px;
    border: 2px solid var(--paper-edge);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: all 0.14s;
    background: var(--paper);
  }

  .checkbox-checked {
    border-color: var(--sumi-4);
    background: var(--sumi);
  }

  .check-icon {
    color: var(--paper);
    font-size: 12px;
    line-height: 1;
    font-weight: 700;
  }
</style>
