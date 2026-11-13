<script lang="ts">
  // Completed today · N (COMPONENTS §3b CompletedList): the same card as the lanes.
  import type { RelayCompletedRowView } from '@sensei/state';
  import CompletedRow from './CompletedRow.svelte';
  import SectionHeading from './SectionHeading.svelte';
  import { NOW_CARD } from './primitives.js';

  let {
    rows,
    showing = null,
  }: { rows: RelayCompletedRowView[]; showing?: { label: string; onclear: () => void } | null } = $props();
</script>

<section aria-label="Completed today" data-relay-section="completed" class="flex flex-col gap-1.5">
  <SectionHeading label="Completed today" count={rows.length} {showing} />
  <ul data-completed-card class={NOW_CARD}>
    {#each rows as row (row.id)}
      <CompletedRow {row} />
    {/each}
  </ul>
</section>
