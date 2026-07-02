<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  interface ProjectLibrary {
    id: string;
    name: string;
    ecosystem: string;
    scope: 'global' | 'project';
    enabled: boolean;
  }

  const projectId = $derived(page.params.id ?? '');
  let libraries = $state<ProjectLibrary[]>([]);
  let loading = $state(true);

  onMount(async () => {
    if (!projectId) {
      loading = false;
      return;
    }
    const api = senseiApi(appState.port);
    const resp = await api.getProjectLibraries(projectId);
    libraries = resp.libraries ?? [];
    loading = false;
  });
</script>

<div class="max-w-[820px] mx-auto px-12 py-10 pb-16">
  <h1 class="display text-xl font-normal m-0 mb-6">Libraries</h1>
  {#if loading}
    <p class="text-sm text-ink-soft">Loading libraries…</p>
  {:else if libraries.length === 0}
    <div class="flex flex-col items-center gap-2 px-5 py-10 bg-paper-mute border border-paper-mute rounded-lg text-center">
      <span class="kanji text-3xl text-accent opacity-40">庫</span>
      <p class="text-sm text-ink-soft leading-normal max-w-[420px] m-0">
        No libraries linked to this project yet.
      </p>
    </div>
  {:else}
    <ul class="flex flex-col gap-1">
      {#each libraries as lib (lib.id)}
        <li class="flex items-center gap-3 px-3.5 py-2.5 rounded-md border border-paper-mute">
          <span class="text-sm font-medium text-ink">{lib.name}</span>
          <span class="text-xs uppercase tracking-wide text-ink-mute">{lib.ecosystem}</span>
          <span class="text-xs text-ink-soft ml-auto">{lib.scope}</span>
          {#if !lib.enabled}
            <span class="text-xs uppercase tracking-wide text-danger">disabled</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>
