<script lang="ts">
  import { onMount } from 'svelte';
  import type { WizardState, WizUpdate, AssistantEntry, WizStage } from '../types.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort } from '$lib/appstate.svelte.js';
  import StepHeader from './StepHeader.svelte';

  let { wizState, update, stage }: {
    wizState: WizardState;
    update: WizUpdate;
    stage: WizStage;
  } = $props();

  let assistants = $state<AssistantEntry[]>([]);

  // Sync from wizState on init (avoid state_referenced_locally by reading inside $effect)
  $effect(() => {
    if (assistants.length === 0 && wizState.assistantList.length > 0) {
      assistants = wizState.assistantList;
    }
  });

  /** Replace home dir with ~ for display */
  function shortPath(p: string | null): string {
    if (!p) return '';
    const home = typeof process !== 'undefined' ? process.env.HOME : null;
    if (home && p.startsWith(home)) return '~' + p.slice(home.length);
    return p.replace(/^\/Users\/[^/]+/, '~');
  }

  onMount(async () => {
    try {
      const api = senseiApi(getPort());
      const families = await api.detectAssistantFamilies();
      if (families.length > 0) {
        assistants = families.map(f => ({
          id: f.family, name: f.name, version: null,
          found: f.installed, path: f.config_path ? shortPath(f.config_path) : null,
        }));
        update({
          assistants: Object.fromEntries(families.map(f => [f.family, f.installed])),
          assistantList: assistants,
        });
      }
    } catch { /* keep whatever was passed */ }
  });

  function toggle(id: string) {
    update({ assistants: { ...wizState.assistants, [id]: !wizState.assistants[id] } });
  }
</script>

<section class="step">
  <StepHeader {stage} subtitle="Registers plugins, skills, commands, agents, logging and metrics." />

  <div class="grid">
    {#each assistants as asst}
      {@const checked = !!wizState.assistants[asst.id]}
      {@const found = asst.found}
      <button
        class="card"
        class:card-found={found}
        class:card-missing={!found}
        onclick={() => toggle(asst.id)}
      >
        <div class="card-body">
          <div class="card-top">
            <span class="card-name">{asst.name}</span>
            {#if asst.version}
              <span class="card-version">v{asst.version}</span>
            {/if}
          </div>
          <div class="card-bottom">
            {#if found && asst.path}
              <span class="card-path">{asst.path}</span>
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
