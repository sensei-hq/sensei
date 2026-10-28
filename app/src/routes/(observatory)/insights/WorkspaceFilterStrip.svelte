<script lang="ts">
    import { KIND_ICONS } from '$lib/glyph-vocabulary';
    import Kanji from '$lib/components/Kanji.svelte';
  import type { InsightWorkspaceRef } from '$lib/types.js';
  import { chipWorkspaces, matchWorkspaces } from './workspace-filter.js';

  let {
    workspaces,
    selectedId,
    onSelect,
  }: {
    workspaces: InsightWorkspaceRef[];
    /** null = all workspaces. */
    selectedId: string | null;
    onSelect: (id: string | null) => void;
  } = $props();

  // The rail showed one chip per workspace — unusable at 100+ workspaces. Now: an
  // "All" chip, the 3 most-recent workspaces as chips (plus the active one if it's
  // not among them), and a search box to reach any other workspace.
  let query = $state('');
  let searchFocused = $state(false);

  const chips = $derived(chipWorkspaces(workspaces, selectedId));
  const matches = $derived(matchWorkspaces(workspaces, query));

  function pick(id: string | null): void {
    onSelect(id);
    query = '';
    searchFocused = false;
  }
</script>

<!-- One filter runs the whole page: pick a scope and all three columns narrow. -->
<div class="flex items-center gap-2" data-component="workspace-filter">
  <button
    type="button"
    data-chip="all"
    class="shrink-0 rounded-full border text-xs py-1 px-3 cursor-pointer"
    class:border-accent={selectedId === null}
    class:text-on-accent-soft={selectedId === null}
    class:bg-accent-soft={selectedId === null}
    class:border-paper-edge={selectedId !== null}
    class:text-ink-mute={selectedId !== null}
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
      class:text-on-accent-soft={active}
      class:bg-accent-soft={active}
      class:border-paper-edge={!active}
      class:text-ink-mute={!active}
      class:bg-paper-soft={!active}
      onclick={() => pick(p.id)}
    >
      <Kanji char={p.kanji} icon={KIND_ICONS.workspace} size="inherit" tone="inherit" class="text-xs {active ? 'text-on-accent-soft' : 'text-ink-soft'}" />
      <span>{p.name}</span>
    </button>
  {/each}

  <!-- Search — reach any workspace without a chip per workspace. The dropdown shows
       matches while typing; picking one selects it (and it appears as a chip via
       chipWorkspaces since it becomes the active selection). -->
  <div class="relative shrink-0 ml-1">
    <input
      type="text"
      data-workspace-search
      placeholder="Search workspaces…"
      bind:value={query}
      onfocus={() => (searchFocused = true)}
      onblur={() => setTimeout(() => (searchFocused = false), 120)}
      class="w-40 rounded-full border border-paper-edge bg-paper-soft text-xs text-ink py-1 px-3 outline-none focus:border-accent"
    />
    {#if searchFocused && matches.length > 0}
      <ul
        data-workspace-search-results
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
              <Kanji char={p.kanji} icon={KIND_ICONS.workspace} size="inherit" tone="inherit" class="text-xs text-ink-soft" />
              <span class="truncate">{p.name}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
