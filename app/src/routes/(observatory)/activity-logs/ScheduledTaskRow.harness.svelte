<script lang="ts">
  import type { ScheduledTask } from '$lib/types.js';
  import type { TaskLog } from './scheduled-tasks.svelte.js';
  import ScheduledTaskRowView from './ScheduledTaskRow.svelte';

  let { task, now = 0, log, layout = 'row' }: {
    task: ScheduledTask;
    now?: number;
    log?: TaskLog;
    layout?: 'row' | 'stack';
  } = $props();
</script>

<ScheduledTaskRowView {task} {now} {log} {layout} />
