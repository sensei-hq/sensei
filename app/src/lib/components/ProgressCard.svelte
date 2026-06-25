<script lang="ts">
  import Eyebrow from './Eyebrow.svelte';
  import Spinner from './Spinner.svelte';

  /**
   * A reusable progress card: label + trailing estimate, a progress bar, an
   * optional left/right stat row, an optional live-activity line (with spinner),
   * and an optional footer note. Data-driven so it serves any progress update
   * (install/remedy, scan, backfill, …). Pure presentational; all copy is props.
   */
  interface Props {
    label: string;
    /** Right-aligned estimate beside the label, e.g. "≈ 2 min left". */
    trailing?: string;
    /** Fill percentage 0–100. */
    percent: number;
    /** Lower-left stat, e.g. "3 of 6 ready". */
    left?: string;
    /** Lower-right stat, e.g. "1.4 / 2.1 GB". */
    right?: string;
    /** Live activity line under a divider, e.g. a command being run. */
    activity?: string;
    /** Quiet footer note. */
    note?: string;
  }
  let { label, trailing, percent, left, right, activity, note }: Props = $props();

  const pct = $derived(Math.max(0, Math.min(100, percent)));
</script>

<div
  data-component="progress-card"
  class="rounded-lg border border-paper-edge bg-paper-soft p-4"
>
  <div class="flex items-baseline justify-between mb-2.5">
    <Eyebrow>{label}</Eyebrow>
    {#if trailing}
      <span class="font-mono text-xs text-ink-mute">{trailing}</span>
    {/if}
  </div>

  <div
    class="h-1 rounded-sm bg-paper-mute overflow-hidden"
    role="progressbar"
    aria-valuenow={pct}
    aria-valuemin={0}
    aria-valuemax={100}
  >
    <div class="h-full rounded-sm bg-accent" style="width: {pct}%;"></div>
  </div>

  {#if left || right}
    <div class="flex justify-between mt-1.5 font-mono text-xs text-ink-faint">
      <span>{left ?? ''}</span>
      <span>{right ?? ''}</span>
    </div>
  {/if}

  {#if activity}
    <div class="flex items-center gap-2 mt-3 pt-3 border-t border-paper-edge">
      <Spinner size={11} />
      <span class="font-mono text-xs text-ink-soft truncate">{activity}</span>
    </div>
  {/if}

  {#if note}
    <p class="text-xs text-ink-faint leading-relaxed mt-2.5">{note}</p>
  {/if}
</div>
