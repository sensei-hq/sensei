<script lang="ts">
  // Waits, oldest first (COMPONENTS §3b WaitList): the rows of the long-waits card in
  // Needs you, and of Waiting's filter on Now.
  import type { RelayWaitRowView } from '@sensei/state';
  import WaitRow from './WaitRow.svelte';

  let {
    rows,
    onnudge,
    nudging = [],
    errors = {},
  }: {
    rows: RelayWaitRowView[];
    onnudge?: (row: RelayWaitRowView) => void;
    /** Rows whose nudge is on its way. */
    nudging?: string[];
    /** Why a row's last nudge was refused, by row id. */
    errors?: Record<string, string>;
  } = $props();
</script>

<ul data-wait-list class="m-0 p-0 list-none flex flex-col">
  {#each rows as row (row.id)}
    <WaitRow {row} {onnudge} busy={nudging.includes(row.id)} error={errors[row.id] ?? null} />
  {/each}
</ul>
