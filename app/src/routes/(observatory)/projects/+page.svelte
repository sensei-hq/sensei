<script lang="ts">
  import { Toggle } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { Eyebrow } from '$lib/components';
  import { projectStatus, type EnrichedProject } from './buckets.js';
  import { projectsView, type ProjectFilter, type ProjectView } from './projects-view.svelte.js';
  import ProjectCard from './ProjectCard.svelte';
  import ProjectRow from './ProjectRow.svelte';

  let { data } = $props();

  const allProjects = $derived<EnrichedProject[]>([
    ...data.buckets.active,
    ...data.buckets.dormant,
    ...data.buckets.archived,
  ]);

  const counts = $derived({
    all:      allProjects.length,
    active:   data.buckets.active.length,
    dormant:  data.buckets.dormant.length,
    archived: data.buckets.archived.length,
  });

  // Buckets arrive pre-ordered (active by FTR, then dormant, then archived),
  // and Array.filter is stable, so filtering preserves that order.
  const filtered = $derived.by(() => {
    const q = projectsView.query.trim().toLowerCase();
    const f = projectsView.filter;
    return allProjects.filter((p) => {
      if (f !== 'all' && projectStatus(p) !== f) return false;
      if (q) {
        const hay = `${p.name} ${p.client ?? ''}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  });

  function open(id: string, name: string): void {
    openProjectWindow(id, name).catch((e) => console.error(e));
  }

  // Rokkit Toggle options. Toggle reads `label`; extra keys (kanji/count/glyph)
  // are read in the itemContent snippet via proxy.get(). Filter counts are live,
  // so its options are derived.
  const filterOptions = $derived([
    { value: 'all',      label: 'All',      kanji: '全', count: counts.all },
    { value: 'active',   label: 'Active',   kanji: '動', count: counts.active },
    { value: 'dormant',  label: 'Dormant',  kanji: '眠', count: counts.dormant },
    { value: 'archived', label: 'Archived', kanji: '蔵', count: counts.archived },
  ]);

  const viewOptions = [
    { value: 'grid', label: 'grid view', icon: 'i-solar:widget-2-linear' },
    { value: 'list', label: 'list view', icon: 'i-solar:hamburger-menu-linear' },
  ];
</script>

<div data-component="projects-page" class="w-full h-full flex flex-col bg-paper overflow-hidden">
  <!-- Header -->
  <div class="pt-5 pb-4 px-7 border-b border-paper-edge flex items-center gap-5">
    <span class="kanji text-2xl text-accent">場</span>
    <div>
      <p class="m-0 mb-0.5"><Eyebrow>Projects</Eyebrow></p>
      <h1 class="display text-xl font-normal m-0">All the places you work.</h1>
    </div>
    <span class="flex-1"></span>
    <span class="font-mono text-xs text-ink-soft py-1 px-2 border border-paper-edge rounded">
      ⌘K to jump
    </span>
  </div>

  <!-- Filter chips + view toggle + search -->
  <div class="py-3 px-7 gap-4 border-b border-paper-edge flex items-center">
    <Toggle
      options={filterOptions}
      value={projectsView.filter}
      onchange={(v: unknown) => projectsView.setFilter(v as ProjectFilter)}
      aria-label="filter projects by status"
    >
      {#snippet itemContent(proxy: ProxyItem)}
        <span class="kanji text-xs">{proxy.get('kanji')}</span>
        <span>{proxy.label}</span>
        <!-- Inherit the option's text colour (rokkit sets a contrast-correct colour
             per selected/unselected state) instead of a hardcoded text-ink-mute,
             which failed contrast on the SELECTED pill's background (a11y). -->
        <span class="font-mono text-xs">{proxy.get('count')}</span>
      {/snippet}
    </Toggle>
    <span class="flex-1"></span>

    <!-- View switcher: grid 田 / list ≣ -->
    <Toggle
      options={viewOptions}
      value={projectsView.view}
      onchange={(v: unknown) => projectsView.setView(v as ProjectView)}
      aria-label="view"
    >
      {#snippet itemContent(proxy: ProxyItem)}
        <span class="{proxy.get('icon')} text-base" aria-hidden="true"></span>
      {/snippet}
    </Toggle>

    <div class="flex items-center bg-paper-soft border border-paper-edge rounded gap-2 py-1 px-2 min-w-[260px]">
      <span class="kanji text-xs text-ink-soft">探</span>
      <input
        type="text"
        bind:value={projectsView.query}
        placeholder="search projects or clients…"
        aria-label="search projects"
        class="border-none outline-none bg-transparent text-sm flex-1 text-ink placeholder:text-ink-faint"
      />
      {#if projectsView.query}
        <button
          type="button"
          onclick={() => projectsView.clearQuery()}
          class="text-xs text-ink-faint hover:text-ink"
          aria-label="clear search"
        >×</button>
      {/if}
    </div>
    <span class="font-mono text-xs text-ink-soft whitespace-nowrap">
      {filtered.length} of {counts.all}
    </span>
  </div>

  <!-- Body -->
  <main class="flex-1 overflow-auto pt-5 pb-6 px-7">
    {#if counts.all === 0}
      <p class="text-sm text-ink-soft opacity-70 py-6 text-center">
        No projects yet. Set up a project to get started.
      </p>
    {:else if filtered.length === 0}
      <p class="text-sm text-ink-soft py-6 text-center">
        {projectsView.query ? `No projects match “${projectsView.query}”.` : 'No projects in this bucket.'}
      </p>
    {:else if projectsView.view === 'grid'}
      <div class="grid grid-cols-3 gap-3 items-start">
        {#each filtered as p (p.id)}
          <ProjectCard {p} onOpen={open} />
        {/each}
      </div>
    {:else}
      <div class="border border-paper-edge rounded-lg overflow-hidden bg-paper-soft">
        {#each filtered as p, i (p.id)}
          <ProjectRow {p} onOpen={open} last={i === filtered.length - 1} />
        {/each}
      </div>
    {/if}
  </main>
</div>
