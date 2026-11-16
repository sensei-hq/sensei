<script lang="ts">
  // One task of a run (#422): its title, its priority while it waits, its state; then who
  // did or does it (codename · assistant · model, and who reviewed it once done) and its
  // step or its run time. The link row is the rokkit List's; this is its content.
  import { Pill } from '@rokkit/ui';
  import type { RelayRunTaskRowView } from '@sensei/state';

  let { row }: { row: RelayRunTaskRowView } = $props();
</script>

<span data-run-task={row.id} class="flex flex-col gap-0.5 min-w-0 w-full py-1.5">
  <span class="flex flex-wrap items-center gap-x-2 gap-y-1">
    <span data-run-task-title class="min-w-0 flex-[1_1_10rem] truncate text-sm text-ink">{row.title}</span>
    {#if row.priority}<span data-run-task-priority class="text-xs text-ink-mute">{row.priority}</span>{/if}
    <Pill value={row.state.label} tone={row.state.tone} size="sm" />
  </span>
  {#if row.who || row.detail}
    <span class="flex flex-wrap gap-x-2 gap-y-0.5 text-xs">
      {#if row.who}<span data-run-task-who class="text-ink-soft">{row.who}</span>{/if}
      {#if row.detail}<span data-run-task-detail class="text-ink-mute">{row.detail}</span>{/if}
    </span>
  {/if}
</span>
