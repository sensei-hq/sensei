<script lang="ts">
  import type { InsightProjectRef } from '$lib/types.js';
  import { chipProjects, matchProjects } from './project-filter.js';

  let {
    projects,
    selectedId,
    onSelect,
  }: {
    projects: InsightProjectRef[];
    /** null = all projects. */
    selectedId: string | null;
    onSelect: (id: string | null) => void;
  } = $props();

  // The rail showed one chip per project — unusable at 100+ projects. Now: an
  // "All" chip, the 3 most-recent projects as chips (plus the active one if it's
  // not among them), and a search box to reach any other project.
  let query = $state('');
  let searchFocused = $state(false);

  const chips = $derived(chipProjects(projects, selectedId));
  const matches = $derived(matchProjects(projects, query));

  function pick(id: string | null): void {
    onSelect(id);
    query = '';
    searchFocused = false;
  }
</script>

<!-- One filter runs the whole page: pick a scope and all three columns narrow. -->
<div class="flex items-center gap-2" data-component="project-filter">
  <button
    type="button"
    data-chip="all"
    class="shrink-0 rounded-full border text-xs py-1 px-3 cursor-pointer"
    class:border-accent={selectedId === null}
    class:text-accent={selectedId === null}
    class:bg-accent-soft={selectedId === null}
    class:border-paper-edge={selectedId !== null}
    class:text-ink-soft={selectedId !== null}
    class:bg-paper-soft={selectedId !== null}
    onclick={() => pick(null)}
  >
    All
  </button>

  {#each chips as p (p.id)}
    {@const active = selectedId === p.id}
    <button
      type="button"
      data-chip={p.id}
      class="shrink-0 inline-flex items-center gap-1 rounded-full border text-xs py-1 px-3 cursor-pointer"
      class:border-accent={active}
      class:text-accent={active}
      class:bg-accent-soft={active}
      class:border-paper-edge={!active}
      class:text-ink-soft={!active}
      class:bg-paper-soft={!active}
      onclick={() => pick(p.id)}
    >
      <span class="kanji text-xs" class:text-accent={active} class:text-ink-mute={!active}>{p.kanji}</span>
      <span>{p.name}</span>
    </button>
  {/each}

  <!-- Search — reach any project without a chip per project. The dropdown shows
       matches while typing; picking one selects it (and it appears as a chip via
       chipProjects since it becomes the active selection). -->
  <div class="relative shrink-0 ml-1">
    <input
      type="text"
      data-project-search
      placeholder="Search projects…"
      bind:value={query}
      onfocus={() => (searchFocused = true)}
      onblur={() => setTimeout(() => (searchFocused = false), 120)}
      class="w-40 rounded-full border border-paper-edge bg-paper-soft text-xs text-ink py-1 px-3 outline-none focus:border-accent"
    />
    {#if searchFocused && matches.length > 0}
      <ul
        data-project-search-results
        class="absolute z-10 mt-1 left-0 min-w-[200px] max-h-64 overflow-auto list-none m-0 p-1 rounded-md border border-paper-edge bg-paper shadow-lg"
      >
        {#each matches as p (p.id)}
          <li>
            <button
              type="button"
              data-search-result={p.id}
              class="w-full text-left flex items-center gap-2 rounded px-2 py-1.5 text-xs text-ink bg-transparent border-none cursor-pointer hover:bg-paper-mute"
              onclick={() => pick(p.id)}
            >
              <span class="kanji text-xs text-ink-mute">{p.kanji}</span>
              <span class="truncate">{p.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
