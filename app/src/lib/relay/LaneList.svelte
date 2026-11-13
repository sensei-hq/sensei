<script lang="ts">
  // Lanes · N (COMPONENTS §3b LaneList; CONTRACT §1c-2): one card of lane rows. The
  // workspace window draws one crew flat; the main window one card per crew, under
  // its repository, because a lane code is unique per crew only.
  import type { RelayLaneGroupView } from '@sensei/state';
  import LaneRow from './LaneRow.svelte';
  import SectionHeading from './SectionHeading.svelte';
  import { NOW_CARD } from './primitives.js';

  let { groups }: { groups: RelayLaneGroupView[] } = $props();
  const count = $derived(groups.reduce((n, g) => n + g.rows.length, 0));
</script>

<section aria-label="Lanes" data-relay-section="lanes" class="flex flex-col gap-1.5">
  <SectionHeading label="Lanes" {count} />
  {#each groups as group (group.id)}
    <div data-crew-group={group.id} class="flex flex-col gap-1.5">
      {#if group.label}<p data-crew-heading class="m-0 text-xs text-ink-mute">{group.label}</p>{/if}
      <ul data-lanes-card class={NOW_CARD}>
        {#each group.rows as row (row.id)}
          <LaneRow {row} />
        {/each}
      </ul>
    </div>
  {/each}
</section>
