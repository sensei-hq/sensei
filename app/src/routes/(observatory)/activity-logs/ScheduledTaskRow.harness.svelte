<script lang="ts">
  import type { ScheduledTask } from '$lib/types.js';
  import ScheduledTaskRowView from './ScheduledTaskRow.svelte';

  let { task, now = 0 }: { task: ScheduledTask; now?: number } = $props();
</script>

<ScheduledTaskRowView {task} {now} />
