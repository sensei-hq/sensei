<script lang="ts">
  import { onMount } from 'svelte';
  import type { WizardState, WizUpdate, ComponentStatus } from '../types.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort } from '$lib/appstate.svelte.js';

  let { wizState, update }: {
    wizState: WizardState;
    update: WizUpdate;
  } = $props();

  const port = $derived(getPort());

  // Load components directly when this step mounts
  let components = $state<ComponentStatus[]>(wizState.components);

  onMount(async () => {
    try {
      const api = senseiApi(getPort());
      const data = await api.getComponents();
      if (data.components?.length > 0) {
        components = data.components as ComponentStatus[];
        update({ components: components });
      }
    } catch {
      // Keep whatever state was passed in
    }
  });
</script>

<section class="step">
  <div class="step-label"><span class="kanji">二</span> STEP</div>
  <h1 class="display headline">Components</h1>
  <p class="subtitle">{components.length > 0 ? 'Everything is in place.' : 'Checking components...'}</p>

  <div class="cards">
    {#each components as comp}
      <div class="card">
        <div class="card-icon kanji">{comp.icon}</div>
        <div class="card-info">
          <div class="card-name">{comp.name}</div>
          <div class="card-meta">
            {#if comp.version}{comp.version}{/if}
            {#if comp.version && comp.status}
              <span class="dot">&middot;</span>
            {/if}
            {comp.status}
          </div>
        </div>
        {#if comp.status === 'ready'}
          <div class="badge-ready">
            <span class="badge-dot"></span>
            READY
          </div>
        {:else}
          <div class="badge-other">{comp.status.toUpperCase()}</div>
        {/if}
      </div>
    {/each}
  </div>

  <p class="footer-note">Nothing leaves <code>localhost:{port}</code>.</p>
</section>

<style>
  .step { max-width: 780px; }
  .step-label { font-size: 12px; letter-spacing: 0.12em; color: var(--sumi-3); margin-bottom: var(--space-2); }
  .step-label .kanji { color: var(--shu); margin-right: 4px; }
  .headline { font-size: 40px; color: var(--sumi); margin: 0 0 var(--space-2) 0; line-height: 1.15; }
  .subtitle { font-size: 15px; color: var(--sumi-3); margin: 0 0 var(--space-8) 0; }
  .cards { display: flex; flex-direction: column; gap: var(--space-3); }
  .card {
    display: flex; align-items: center; gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
    background: var(--paper-2); border-radius: var(--radius-lg);
  }
  .card-icon {
    width: 40px; height: 40px;
    display: flex; align-items: center; justify-content: center;
    font-size: 18px; color: var(--sumi-3);
    border: var(--border-card); border-radius: var(--radius);
    background: var(--paper); flex-shrink: 0;
  }
  .card-info { flex: 1; min-width: 0; }
  .card-name { font-size: 15px; font-weight: 500; color: var(--sumi); }
  .card-meta { font-size: 13px; color: var(--sumi-3); margin-top: 2px; }
  .dot { margin: 0 4px; }
  .badge-ready { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 600; letter-spacing: 0.06em; color: var(--jade); flex-shrink: 0; }
  .badge-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--jade); }
  .badge-other { font-size: 11px; font-weight: 600; letter-spacing: 0.06em; color: var(--sumi-3); flex-shrink: 0; }
  .footer-note { margin-top: var(--space-8); font-size: 13px; color: var(--sumi-3); }
  .footer-note code { font-family: var(--font-mono); font-size: 12px; }
</style>
