<script lang="ts">
  import { openProjectWindow } from '$lib/stores/windows.svelte.js';
  import { Eyebrow } from '$lib/components';
  import { projectStatus, type EnrichedProject } from './buckets.js';
  import { projectsView, type ProjectFilter } from './projects-view.svelte.js';
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

  const chips: Array<{ v: ProjectFilter; label: string; kanji: string }> = [
    { v: 'all',      label: 'All',      kanji: '全' },
    { v: 'active',   label: 'Active',   kanji: '動' },
    { v: 'dormant',  label: 'Dormant',  kanji: '眠' },
    { v: 'archived', label: 'Archived', kanji: '蔵' },
  ];

  const views: Array<{ v: 'grid' | 'list'; glyph: string; label: string }> = [
    { v: 'grid', glyph: '田', label: 'grid view' },
    { v: 'list', glyph: '≣', label: 'list view' },
  ];
</script>

<div class="w-full h-full flex flex-col bg-paper overflow-hidden">
  <!-- Header -->
  <div class="pt-5 pb-4 px-7 border-b border-paper-edge flex items-center gap-5">
    <span class="kanji text-[28px] text-accent">場</span>
    <div>
      <p class="m-0 mb-0.5"><Eyebrow>Projects</Eyebrow></p>
      <h1 class="display text-[22px] font-normal m-0">All the places you work.</h1>
    </div>
    <span class="flex-1"></span>
    <span class="font-mono text-[11px] text-ink-soft py-1 px-2 border border-paper-edge rounded">
      ⌘K to jump
    </span>
  </div>

  <!-- Filter chips + view toggle + search -->
  <div class="py-3 px-7 gap-4 border-b border-paper-edge flex items-center">
    <div class="flex gap-1">
      {#each chips as c (c.v)}
        {@const on = projectsView.filter === c.v}
        {@const n = counts[c.v]}
        <button
          type="button"
          data-chip={c.v}
          data-active={on || undefined}
          onclick={() => projectsView.setFilter(c.v)}
          class="py-1 px-3 gap-2 text-[11px] rounded inline-flex items-center transition-colors"
          class:bg-ink={on}
          class:text-paper={on}
          class:text-ink-mute={!on}
        >
          <span class="kanji text-[11px]">{c.kanji}</span>
          {c.label}
          <span
            class="font-mono text-[11px]"
            class:text-paper={on}
            class:text-ink-faint={!on}
          >{n}</span>
        </button>
      {/each}
    </div>
    <span class="flex-1"></span>

    <!-- View toggle: grid 田 / list ≣ -->
    <div
      class="flex gap-1 p-1 bg-paper-soft border border-paper-edge rounded"
      role="group"
      aria-label="view"
    >
      {#each views as v (v.v)}
        {@const on = projectsView.view === v.v}
        <button
          type="button"
          data-view-toggle={v.v}
          aria-pressed={on}
          aria-label={v.label}
          onclick={() => projectsView.setView(v.v)}
          class="py-1 px-2 rounded-sm transition-colors"
          class:bg-paper={on}
          class:text-ink={on}
          class:text-ink-mute={!on}
        >
          <span class="kanji text-[12px]">{v.glyph}</span>
        </button>
      {/each}
    </div>

    <div class="flex items-center bg-paper-soft border border-paper-edge rounded gap-2 py-1 px-2 min-w-[260px]">
      <span class="kanji text-[11px] text-ink-soft">探</span>
      <input
        type="text"
        bind:value={projectsView.query}
        placeholder="search projects or clients…"
        aria-label="search projects"
        class="border-none outline-none bg-transparent text-[13px] flex-1 text-ink placeholder:text-ink-faint"
      />
      {#if projectsView.query}
        <button
          type="button"
          onclick={() => projectsView.clearQuery()}
          class="text-[11px] text-ink-faint hover:text-ink"
          aria-label="clear search"
        >×</button>
      {/if}
    </div>
    <span class="font-mono text-[11px] text-ink-soft whitespace-nowrap">
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
      <p class="text-[13px] text-ink-soft py-6 text-center">
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
