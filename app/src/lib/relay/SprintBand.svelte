<script lang="ts">
  // One open sprint run (queue cutover, phase 4): the band above the lanes —
  // title, state, goal, done over total, and the tasks counted by state. The
  // counts are the daemon's (`relay.now` sprint_summary); the band adds nothing up.
  import type { RelayRunSummary } from '$lib/relay-api.js';

  let { run }: { run: RelayRunSummary } = $props();

  const fraction = $derived(run.total_tasks > 0 ? run.done_tasks / run.total_tasks : 0);
  /** "0", "0.33", "1": a plain number, never a division by nothing. */
  const fractionText = $derived(String(Number(fraction.toFixed(2))));
  const stateLabel = $derived(
    run.state === 'planning'
      ? run.total_tasks === 1 && run.done_tasks === 0
        ? 'planning · Task 0 in the queue'
        : 'planning'
      : run.state,
  );
  const counts = $derived(
    [
      { key: 'ready', n: run.ready_tasks, label: `${run.ready_tasks} ready` },
      { key: 'working', n: run.working_tasks, label: `${run.working_tasks} working` },
      {
        key: 'in_review',
        n: run.in_review_tasks,
        label:
          run.floating_reviews > 0
            ? `${run.in_review_tasks} in review · ${run.floating_reviews} floating`
            : `${run.in_review_tasks} in review`,
      },
      { key: 'approved', n: run.approved_tasks, label: `${run.approved_tasks} approved` },
      { key: 'done', n: run.done_tasks, label: `${run.done_tasks} done` },
      { key: 'blocked', n: run.blocked_tasks, label: `${run.blocked_tasks} blocked` },
      { key: 'interrupted', n: run.interrupted_tasks, label: `${run.interrupted_tasks} interrupted` },
    ].filter((c) => c.n > 0),
  );
</script>

<div
  class="rounded-sm border border-paper-edge bg-paper-soft p-3 flex flex-col gap-2"
  data-sprint-band
  data-run-id={run.run_id}
  data-run-state={run.state}
>
  <div class="flex flex-wrap items-center gap-2" data-row-line="1">
    <span class="text-sm font-medium" data-sprint-title>{run.title}</span>
    <span class="text-xs rounded-sm px-2 py-0.5 bg-paper text-ink-soft" data-state>{stateLabel}</span>
    <span
      class="ml-auto text-xs text-ink-faint tabular-nums"
      data-relay-progress
      data-done={run.done_tasks}
      data-total={run.total_tasks}
      data-fraction={fractionText}
    >
      {run.done_tasks} of {run.total_tasks} done
    </span>
  </div>
  <p class="text-xs text-ink-mute m-0" data-goal>{run.goal}</p>
  <div class="h-1 w-full rounded-sm bg-paper-edge overflow-hidden" aria-hidden="true">
    <div class="h-full bg-accent" style:width={`${Math.round(fraction * 100)}%`}></div>
  </div>
  {#if counts.length > 0}
    <div class="flex flex-wrap gap-2">
      {#each counts as c (c.key)}
        <span class="text-xs rounded-sm bg-paper px-2 py-0.5 text-ink-mute" data-count={c.key}>{c.label}</span>
      {/each}
    </div>
  {/if}
</div>
