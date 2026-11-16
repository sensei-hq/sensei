<script lang="ts">
  // A queue list (COMPONENTS §3c; CONTRACT §1d): its marked heading with the count, a
  // line on what it holds, and one card of QueueRows. Triage rows carry +; backlog
  // rows a checkbox for a run.
  import type { Snippet } from 'svelte';
  import type { RelayQueueRowView } from '@sensei/state';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import QueueRow from './QueueRow.svelte';
  import { NOW_CARD } from './primitives.js';

  let {
    id,
    label,
    kanji,
    description,
    rows,
    onadd = null,
    isSelected = () => false,
    ontoggle = null,
    children,
  }: {
    id: string;
    label: string;
    kanji: string;
    description: string;
    rows: RelayQueueRowView[];
    onadd?: ((row: RelayQueueRowView) => Promise<string | null>) | null;
    isSelected?: (id: string) => boolean;
    ontoggle?: ((row: RelayQueueRowView) => void) | null;
    /** Below the card: a notice, or the add-an-item form. */
    children?: Snippet;
  } = $props();

  const mode = useGlyphMode();
</script>

<section aria-label={label} data-relay-section={id} class="flex flex-col gap-1.5">
  <h3 data-section-heading class="m-0 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.12em] text-ink-mute">
    {#if mode() === 'icon'}
      <span aria-hidden="true" class="{KANJI_ICONS[kanji]} text-accent text-sm"></span>
    {:else}
      <span aria-hidden="true" class="font-kanji text-accent text-sm">{kanji}</span>
    {/if}
    {label}&nbsp;·&nbsp;{rows.length}
  </h3>
  <p data-section-description class="m-0 text-sm text-ink-soft">{description}</p>
  {#if rows.length}
    <ul data-queue-card class={NOW_CARD}>
      {#each rows as row (row.id)}
        <QueueRow
          {row}
          onadd={onadd && row.canAdd ? () => onadd(row) : null}
          selected={isSelected(row.id)}
          ontoggle={ontoggle ? () => ontoggle(row) : null}
        />
      {/each}
    </ul>
  {/if}
  {@render children?.()}
</section>
