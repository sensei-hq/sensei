<script lang="ts">
  import type { InsightProjectRef } from '$lib/types.js';

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
</script>

<!-- One filter runs the whole page: pick a scope and all three columns narrow. -->
<div class="flex items-center gap-2 overflow-x-auto" data-component="project-filter">
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
    onclick={() => onSelect(null)}
  >
    All
  </button>
  {#each projects as p (p.id)}
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
      onclick={() => onSelect(p.id)}
    >
      <span class="kanji text-xs" class:text-accent={active} class:text-ink-mute={!active}>{p.kanji}</span>
      <span>{p.name}</span>
    </button>
  {/each}
</div>
