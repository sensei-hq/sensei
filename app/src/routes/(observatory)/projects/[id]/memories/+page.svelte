<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ProjectMemory } from '$lib/types.js';

  const projectId = $derived(page.params.id ?? '');
  let memories = $state<ProjectMemory[]>([]);
  let pendingShare = $state(0);
  let loading = $state(true);

  onMount(async () => {
    if (!projectId) {
      loading = false;
      return;
    }
    const api = senseiApi(appState.port);
    const resp = await api.getProjectMemories(projectId);
    memories = resp.active ?? [];
    pendingShare = resp.pendingShare ?? 0;
    loading = false;
  });
</script>

<div class="max-w-[820px] mx-auto px-12 py-10 pb-16">
  <div class="flex items-center justify-between mb-6">
    <h1 class="display text-xl font-normal m-0">Memories</h1>
    {#if pendingShare > 0}
      <span class="text-xs px-2 py-1 rounded-full bg-primary-soft text-primary">{pendingShare} ready to share</span>
    {/if}
  </div>
  {#if loading}
    <p class="text-sm text-ink-soft">Loading memories…</p>
  {:else if memories.length === 0}
    <div class="flex flex-col items-center gap-2 px-5 py-10 bg-paper-mute border border-paper-mute rounded-lg text-center">
      <span class="kanji text-3xl text-accent opacity-40">覚</span>
      <p class="text-sm text-ink-soft leading-normal max-w-[420px] m-0">
        No memories captured for this project yet.
      </p>
    </div>
  {:else}
    <ul class="flex flex-col gap-2">
      {#each memories as memory (memory.id)}
        <li class="border border-paper-mute rounded-md p-3">
          <div class="flex items-center gap-2 mb-1">
            <span class="text-xs uppercase tracking-wide text-ink-mute">{memory.kind}</span>
            <span class="text-xs text-ink-soft">{memory.scope}</span>
            <span class="text-xs text-ink-soft ml-auto">strength {(memory.strength * 100).toFixed(0)}%</span>
          </div>
          <div class="text-sm text-ink">{memory.title || memory.name}</div>
        </li>
      {/each}
    </ul>
  {/if}
</div>
