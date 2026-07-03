<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { PageHeader, Eyebrow } from '$lib/components';
  import type { EnrichedProject } from './buckets.js';

  let { data } = $props();

  function ftrPct(v: number): string {
    return `${Math.round(v * 100)}%`;
  }

  function sessionsLabel(n: number): string {
    if (n === 0) return 'no sessions';
    if (n === 1) return '1 session';
    return `${n} sessions`;
  }
</script>

{#snippet section(label: string, projects: EnrichedProject[])}
  {#if projects.length > 0}
    <section class="mb-8" data-bucket={label.toLowerCase()}>
      <div class="flex items-baseline justify-between mb-3">
        <h2 class="display text-base font-normal m-0">{label}</h2>
        <Eyebrow>{projects.length} {projects.length === 1 ? 'project' : 'projects'}</Eyebrow>
      </div>
      <div class="grid gap-4" style="grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));">
        {#each projects as proj (proj.id)}
          <button
            type="button"
            data-project-card={proj.id}
            class="bg-paper-mute hover:bg-paper-soft rounded-lg p-5 flex flex-col gap-1.5 cursor-pointer border-none text-left text-inherit"
            onclick={() => openProjectWindow(proj.id, proj.name).catch(console.error)}
          >
            <div class="flex items-start justify-between gap-2">
              <span class="kanji text-3xl">{proj.icon?.value ?? '場'}</span>
              {#if proj.warn}
                <span
                  data-warn-dot
                  class="w-2 h-2 rounded-full bg-warning mt-2"
                  title="Open drift or low FTR"
                ></span>
              {/if}
            </div>
            <span class="text-base font-semibold text-ink truncate">{proj.name}</span>
            {#if proj.client}
              <span class="text-xs text-ink-mute truncate">{proj.client}</span>
            {/if}
            <div class="flex items-baseline gap-2 mt-1">
              <span
                data-ftr
                class="font-mono text-sm"
                class:text-success={proj.ftr14d >= 0.8}
                class:text-warning={proj.ftr14d < 0.6}
                class:text-ink={proj.ftr14d >= 0.6 && proj.ftr14d < 0.8}
              >{ftrPct(proj.ftr14d)}</span>
              <Eyebrow>FTR 14d</Eyebrow>
            </div>
            <span data-sessions class="text-xs text-ink-mute mt-0.5">{sessionsLabel(proj.sessions7d)} · 7d</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}
{/snippet}

<PageHeader kanji="場" eyebrow="Observatory" title="Projects" />
<div class="p-6">
  {#if data.buckets.active.length === 0 && data.buckets.recent.length === 0 && data.buckets.archived.length === 0}
    <p class="text-sm text-ink-soft opacity-50">No projects yet. Set up a project to get started.</p>
  {:else}
    {@render section('Active',   data.buckets.active)}
    {@render section('Recent',   data.buckets.recent)}
    {@render section('Archived', data.buckets.archived)}
  {/if}
</div>
