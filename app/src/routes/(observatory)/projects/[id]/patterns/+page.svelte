<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { PatternEntry } from '$lib/types.js';

  const projectId = $derived(page.params.id ?? '');
  let followed = $state<PatternEntry[]>([]);
  let antiPatterns = $state<PatternEntry[]>([]);
  let loading = $state(true);

  onMount(async () => {
    if (!projectId) {
      loading = false;
      return;
    }
    const api = senseiApi(appState.port);
    const resp = await api.getProjectPatterns(projectId);
    followed = resp.followed ?? [];
    antiPatterns = resp.antiPatterns ?? [];
    loading = false;
  });
</script>

{#snippet PatternList(items: PatternEntry[], emptyText: string)}
  {#if items.length === 0}
    <p class="text-sm text-ink-soft leading-normal">{emptyText}</p>
  {:else}
    <ul class="flex flex-col gap-2">
      {#each items as pat (pat.id)}
        <li class="border border-paper-mute rounded-md p-3">
          <div class="flex items-center gap-2 mb-1">
            <span class="text-sm font-medium text-ink">{pat.name}</span>
            <span class="text-xs uppercase tracking-wide text-ink-mute">{pat.kind}</span>
            <span class="text-xs text-ink-soft ml-auto">confidence {(pat.confidence * 100).toFixed(0)}%</span>
          </div>
          <div class="text-xs text-ink-soft">
            {pat.members.length} member{pat.members.length === 1 ? '' : 's'}
            {#if pat.lifecycle}· {pat.lifecycle}{/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

<div class="max-w-[820px] mx-auto px-12 py-10 pb-16">
  <h1 class="display text-xl font-normal m-0 mb-6">Patterns</h1>
  {#if loading}
    <p class="text-sm text-ink-soft">Loading patterns…</p>
  {:else}
    <section class="mb-8">
      <h3 class="text-sm font-medium m-0 mb-3.5 text-ink">Followed patterns</h3>
      {@render PatternList(followed, 'No confirmed patterns yet.')}
    </section>
    <section>
      <h3 class="text-sm font-medium m-0 mb-3.5 text-ink">Anti-patterns</h3>
      {@render PatternList(antiPatterns, 'No anti-patterns flagged.')}
    </section>
  {/if}
</div>
