<script lang="ts">
  // One wait (COMPONENTS §3b WaitRow; CONTRACT §1c-0, §3f-bis): how long — amber and
  // bold at the threshold — the task with why and on whom, a link to the task, and
  // Nudge {party} (#419). Once you nudged, the button gives way to when; past the
  // threshold the last automatic nudge shows beside the button.
  import type { RelayWaitRowView } from '@sensei/state';
  import { Notice } from '@sensei/ui';

  let {
    row,
    onnudge,
    busy = false,
    error = null,
  }: {
    row: RelayWaitRowView;
    onnudge?: (row: RelayWaitRowView) => void;
    /** The nudge is on its way. */
    busy?: boolean;
    /** Why the last nudge was refused. */
    error?: string | null;
  } = $props();
</script>

{#snippet body()}
  <span data-part="task" class="text-base">{row.task}</span>
  <span data-part="why" class="text-xs text-ink-soft">{row.why}</span>
{/snippet}

<li data-wait-row={row.id} class="flex flex-wrap items-center gap-x-3 gap-y-1 py-2 border-t border-solid border-paper-mute">
  <span
    data-part="minutes"
    data-over={row.over}
    class={['flex-[0_0_56px] tabular-nums text-sm', row.over ? 'text-warning font-semibold' : 'text-ink-mute']}
  >{row.minutesText}</span>
  {#if row.href}
    <a href={row.href} class="flex-[1_1_calc(100%-80px)] md:flex-[1_1_260px] min-w-0 flex flex-col gap-px text-ink no-underline">{@render body()}</a>
  {:else}
    <span class="flex-[1_1_calc(100%-80px)] md:flex-[1_1_260px] min-w-0 flex flex-col gap-px">{@render body()}</span>
  {/if}
  {#if row.nudge.canNudge && onnudge}
    <button
      type="button"
      data-nudge
      disabled={busy}
      aria-busy={busy}
      onclick={() => onnudge(row)}
      class="min-h-[30px] px-2.5 rounded-[7px] border border-solid border-paper-edge bg-paper text-ink text-xs cursor-pointer disabled:cursor-wait disabled:text-ink-mute"
    >{row.nudge.label}</button>
  {/if}
  {#if row.nudge.nudgedText}
    <span data-nudged class="text-xs text-ink-mute">{row.nudge.nudgedText}</span>
  {/if}
  {#if error}
    <div class="basis-full"><Notice size="inline" tone="fault" title={error} data-nudge-error /></div>
  {/if}
</li>
