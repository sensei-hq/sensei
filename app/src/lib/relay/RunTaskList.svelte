<script lang="ts">
  // A card of run task rows (#422): a display-only rokkit List whose rows link to each task.
  import { List } from '@rokkit/ui';
  import type { ProxyItem } from '@rokkit/states';
  import type { RelayRunTaskRowView } from '@sensei/state';
  import RunTaskRow from './RunTaskRow.svelte';

  let { rows, label }: { rows: RelayRunTaskRowView[]; label: string } = $props();
</script>

<List items={rows} fields={{ value: 'id' }} selectable={false} variant="card" {label}>
  {#snippet itemContent(proxy: ProxyItem)}<RunTaskRow row={proxy.original as RelayRunTaskRowView} />{/snippet}
</List>
