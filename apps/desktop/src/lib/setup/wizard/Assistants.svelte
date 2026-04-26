<script lang="ts">
  import { onMount } from 'svelte';
  import type { WizardState, WizUpdate, AcpEntry, WizStage } from '../types.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort } from '$lib/appstate.svelte.js';
  import StepHeader from './StepHeader.svelte';

  let { wizState, update, stage }: {
    wizState: WizardState;
    update: WizUpdate;
    stage: WizStage;
  } = $props();

  let acps = $state<AcpEntry[]>(wizState.acpList);

  onMount(async () => {
    try {
      const api = senseiApi(getPort());
      const detected = await api.detectAcps();
      if (detected.length > 0) {
        acps = detected.map(a => ({
          id: a.id, name: a.name, version: null,
          found: a.installed, path: a.config_path ?? null,
        }));
        update({
          acps: Object.fromEntries(detected.map(a => [a.id, a.installed])),
          acpList: acps,
        });
      }
    } catch { /* keep whatever was passed */ }
  });

  function toggle(id: string) {
    update({ acps: { ...wizState.acps, [id]: !wizState.acps[id] } });
  }
</script>

<section class="step">
  <StepHeader {stage} subtitle="Registers plugins, skills, commands, agents, logging and metrics." />

  <div class="grid">
    {#each acps as acp}
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
    max-width: 780px;
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
