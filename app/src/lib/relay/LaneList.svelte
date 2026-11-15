<script lang="ts">
  // Lanes · N (COMPONENTS §3b LaneList; CONTRACT §1c-2): one card of lane rows. The
  // workspace window draws one crew flat; the main window one card per crew, under
  // its repository, because a lane code is unique per crew only.
  // Each card is a display-only rokkit List: a lane on a task is a link row, an idle
  // lane a plain row (no button, never focused).
  import { List } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { RelayLaneGroupView, RelayLaneRowView } from '@sensei/state';
  import LaneRow from './LaneRow.svelte';
  import SectionHeading from './SectionHeading.svelte';

  let {
    groups,
    showing = null,
  }: { groups: RelayLaneGroupView[]; showing?: { label: string; onclear: () => void } | null } = $props();
  const count = $derived(groups.reduce((n, g) => n + g.rows.length, 0));
</script>

<section aria-label="Lanes" data-relay-section="lanes" class="flex flex-col gap-1.5">
  <SectionHeading label="Lanes" {count} {showing} />
  {#each groups as group (group.id)}
    <div data-crew-group={group.id} class="flex flex-col gap-1.5">
      {#if group.label}<p data-crew-heading class="m-0 text-xs text-ink-mute">{group.label}</p>{/if}
      <List items={group.rows} fields={{ value: 'id' }} selectable={false} variant="card" label={group.label ?? 'Lanes'}>
        {#snippet itemContent(proxy: ProxyItem)}<LaneRow row={proxy.original as RelayLaneRowView} />{/snippet}
      </List>
    </div>
  {/each}
</section>
