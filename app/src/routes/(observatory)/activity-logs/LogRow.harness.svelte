<script lang="ts">
  import type { LogRow } from '$lib/types.js';
  import LogRowView from './LogRow.svelte';

  let {
    row,
    now = 0,
    expanded = false,
    onToggle = () => {},
  }: { row: LogRow; now?: number; expanded?: boolean; onToggle?: () => void } = $props();
</script>

<LogRowView {row} {now} {expanded} {onToggle} />
