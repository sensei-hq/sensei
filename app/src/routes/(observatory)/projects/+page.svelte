<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { PageHeader } from '$lib/components';
  let { data } = $props();
</script>

<PageHeader kanji="場" eyebrow="Observatory" title="Projects" />
<div class="p-6">
  {#if data.projects.length === 0}
    <p class="text-sm text-surface-z6 opacity-50">No projects yet. Set up a project to get started.</p>
  {:else}
    <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));">
      {#each data.projects as proj (proj.id)}
        <button
          type="button"
          class="project-card bg-surface-z2 rounded-lg p-5 flex flex-col gap-1.5 cursor-pointer border-none text-left text-inherit transition-colors duration"
          onclick={() => openProjectWindow(proj.id, proj.name).catch(console.error)}
        >
          <span class="kanji text-3xl">{proj.icon?.value ?? '場'}</span>
          <span class="text-base font-bold text-surface-z9">{proj.name}</span>
          {#if proj.client}<span class="text-xs text-surface-z7 opacity-60">{proj.client}</span>{/if}
          <span class="text-xs text-surface-z9 opacity-40 font-mono">{proj.maturity}</span>
          <span class="text-xs text-surface-z9 opacity-30 mt-auto">↗</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .project-card:hover { background: oklch(var(--color-surface-z3) / 1); }
</style>
