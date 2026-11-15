<script lang="ts">
  // Completed today · N (COMPONENTS §3b CompletedList): the same card as the lanes.
  // A display-only rokkit List card: every row is a link to its task.
  import { List } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { RelayCompletedRowView } from '@sensei/state';
  import CompletedRow from './CompletedRow.svelte';
  import SectionHeading from './SectionHeading.svelte';

  let {
    rows,
    showing = null,
  }: { rows: RelayCompletedRowView[]; showing?: { label: string; onclear: () => void } | null } = $props();
</script>

<section aria-label="Completed today" data-relay-section="completed" class="flex flex-col gap-1.5">
  <SectionHeading label="Completed today" count={rows.length} {showing} />
  <List items={rows} fields={{ value: 'id' }} selectable={false} variant="card" label="Completed today">
    {#snippet itemContent(proxy: ProxyItem)}<CompletedRow row={proxy.original as RelayCompletedRowView} />{/snippet}
  </List>
</section>
