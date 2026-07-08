<script lang="ts">
  import MemoryRow from './MemoryRow.svelte';
  import type { MemoryRowVm } from './insights-board.svelte.js';

  let {
    memory,
    variant = 'settled',
  }: {
    memory: MemoryRowVm;
    variant?: 'settled' | 'proposed';
  } = $props();
</script>

<MemoryRow {memory} {variant} />
