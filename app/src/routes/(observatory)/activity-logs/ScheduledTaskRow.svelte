<script lang="ts">
  import type { ScheduledTask } from '$lib/types.js';
  import {
    taskLastRun,
    taskInterval,
    taskSchedule,
    taskHealth,
  } from './scheduled-tasks.svelte.js';

  interface Props {
    task: ScheduledTask;
    /** `Date.now()` at render — kept a prop so relative time stays pure/testable. */
    now: number;
  }
  let { task, now }: Props = $props();

  const lastRun = $derived(taskLastRun(now, task));
  const interval = $derived(taskInterval(task));
  const schedule = $derived(taskSchedule(task));
  const health = $derived(taskHealth(task));
</script>

<div
  class="grid grid-cols-[minmax(0,1fr)_96px_84px_128px_72px] items-baseline gap-3 py-2 px-6 border-b border-paper-edge"
  data-testid="scheduled-task"
  data-task-name={task.name}
>
  <div class="min-w-0">
    <div class="font-mono text-xs text-ink truncate">{task.name}</div>
    <div class="text-xs text-ink-mute truncate mt-[2px]" title={task.description}>
      {task.description}
    </div>
  </div>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-last-run>
    {lastRun}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-interval>
    {interval}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-schedule title={schedule}>
    {schedule}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-health>
    {health}
  </span>
</div>
