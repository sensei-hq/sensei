<script lang="ts">
  import type { ScheduledTask } from '$lib/types.js';
  import type { TaskLog } from './scheduled-tasks.svelte.js';
  import ScheduledTaskRowView from './ScheduledTaskRow.svelte';

  let { task, now = 0, log }: { task: ScheduledTask; now?: number; log?: TaskLog } = $props();
</script>

<ScheduledTaskRowView {task} {now} {log} />
