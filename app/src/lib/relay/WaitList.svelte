<script lang="ts">
  // Waits, oldest first (COMPONENTS §3b WaitList): the rows of the long-waits card in
  // Needs you, and of Waiting's filter on Now.
  import type { RelayWaitRowView } from '@sensei/state';
  import WaitRow from './WaitRow.svelte';

  let { rows, onnudge }: { rows: RelayWaitRowView[]; onnudge?: (row: RelayWaitRowView) => void } = $props();
</script>

<ul data-wait-list class="m-0 p-0 list-none flex flex-col">
  {#each rows as row (row.id)}
    <WaitRow {row} {onnudge} />
  {/each}
</ul>
