<script lang="ts">
  import type { ScheduledTask } from '$lib/types.js';
  import type { TaskLog } from './scheduled-tasks.svelte.js';
  import {
    taskLastRun,
    taskInterval,
    taskSchedule,
    taskHealth,
    taskNextRun,
    taskAvg,
    taskRunning,
  } from './scheduled-tasks.svelte.js';

  interface Props {
    task: ScheduledTask;
    /** `Date.now()` at render — kept a prop so relative time stays pure/testable. */
    now: number;
    /** The worker's log tail (#96); absent where no tail is offered. */
    log?: TaskLog;
  }
  let { task, now, log }: Props = $props();

  const lastRun = $derived(taskLastRun(now, task));
  const interval = $derived(taskInterval(task));
  const schedule = $derived(taskSchedule(task));
  const health = $derived(taskHealth(task));
  const nextRun = $derived(taskNextRun(now, task));
  const avg = $derived(taskAvg(task));
  const running = $derived(taskRunning(now, task));
</script>

<div
  class="grid grid-cols-[minmax(0,1fr)_96px_84px_84px_128px_56px_72px] items-baseline gap-3 py-2 px-6 border-b border-paper-edge"
  data-testid="scheduled-task"
  data-task-name={task.name}
>
  <div class="min-w-0">
    <div class="font-mono text-xs text-ink truncate">{task.name}</div>
    <div class="text-xs text-ink-mute truncate mt-[2px]" title={task.description}>
      {task.description}
    </div>
    {#if log}
      <button
        type="button"
        class="text-xs text-ink-faint hover:text-ink underline mt-[2px] bg-transparent border-0 p-0 cursor-pointer"
        data-task-log-toggle
        aria-expanded={log.open}
        onclick={() => log.toggle()}
      >{log.open ? 'hide log' : 'log'}</button>
    {/if}
    {#if running}
      <!-- A pass in progress, so a stall is not read as an old result (#283). -->
      <div class="text-xs text-ink-mute mt-[2px]" data-task-running>{running}</div>
    {/if}
    {#if task.last_ok === false && task.last_error}
      <!-- Why it is failing, where it is listed (#96). -->
      <div class="text-xs text-danger mt-[2px] line-clamp-2" data-task-error title={task.last_error}>
        {task.last_error}
      </div>
    {/if}
  </div>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-last-run>
    {lastRun}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-interval>
    {interval}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-next-run>
    {nextRun}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-schedule title={schedule}>
    {schedule}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-avg>
    {avg}
  </span>

  <span class="font-mono text-xs text-ink-mute truncate" data-task-health>
    {health}
  </span>
</div>

{#if log?.open}
  <div class="px-6 pb-3 border-b border-paper-edge" data-task-log>
    {#if log.loading}
      <p class="text-xs text-ink-faint m-0">Reading the log…</p>
    {:else if log.error}
      <p class="text-xs text-danger m-0" data-task-log-error>{log.error}</p>
    {:else if log.lines.length === 0}
      <p class="text-xs text-ink-faint m-0">Nothing logged by {task.name} today.</p>
    {:else}
      <pre class="font-mono text-xs text-ink-mute m-0 overflow-x-auto whitespace-pre max-h-64 overflow-y-auto">{log.lines.join('\n')}</pre>
    {/if}
  </div>
{/if}
