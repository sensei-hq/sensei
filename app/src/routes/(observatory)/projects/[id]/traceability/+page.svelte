<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { DriftItem } from '$lib/types.js';

  const projectId = $derived(page.params.id ?? '');
  let drift = $state<DriftItem[]>([]);
  let total = $state(0);
  let drifted = $state(0);
  let broken = $state(0);
  let loading = $state(true);

  onMount(async () => {
    if (!projectId) {
      loading = false;
      return;
    }
    const api = senseiApi(appState.port);
    const resp = await api.getProjectDrift(projectId);
    drift = resp.items ?? [];
    total = resp.total ?? 0;
    drifted = resp.drifted ?? 0;
    broken = resp.broken ?? 0;
    loading = false;
  });
</script>

<div class="max-w-[820px] mx-auto px-12 py-10 pb-16">
  <h1 class="display text-xl font-normal m-0 mb-2">Traceability</h1>
  <p class="text-sm text-ink-mute m-0 mb-6">
    Docs whose claims contradict the current code (drift detection).
  </p>

  {#if !loading && total > 0}
    <div class="flex gap-4 mb-6 text-xs">
      <span class="text-ink-soft">Total {total}</span>
      <span class="text-warning">Drifted {drifted}</span>
      <span class="text-danger">Broken {broken}</span>
    </div>
  {/if}

  {#if loading}
    <p class="text-sm text-ink-soft">Loading drift signals…</p>
  {:else if drift.length === 0}
    <div class="flex flex-col items-center gap-2 px-5 py-10 bg-paper-mute border border-paper-mute rounded-lg text-center">
      <span class="kanji text-3xl text-accent opacity-40">巻</span>
      <p class="text-sm text-ink-soft leading-normal max-w-[420px] m-0">
        No drift signals for this project. The doc-drift detector runs
        nightly — new signals appear here once T3 Slice 2.3 lands.
      </p>
    </div>
  {:else}
    <ul class="flex flex-col gap-2">
      {#each drift as item (item.id)}
        <li class="border border-paper-mute rounded-md p-3">
          <div class="flex items-center gap-2 mb-1">
            <span class="text-xs font-mono text-ink">{item.file}</span>
            <span
              class="text-xs uppercase tracking-wide ml-auto"
              class:text-warning={item.status === 'drifted'}
              class:text-danger={item.status === 'broken'}
            >{item.status}</span>
            <span class="text-xs text-ink-soft">{(item.confidence * 100).toFixed(0)}%</span>
          </div>
          {#if item.detail}
            <div class="text-sm text-ink-soft">{item.detail}</div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>
