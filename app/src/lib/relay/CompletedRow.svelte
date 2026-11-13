<script lang="ts">
  // One task finished today (COMPONENTS §3b CompletedRow; CONTRACT §1c-3): time ·
  // title · objective and codename · outcome · chevron, one link to the task. On a
  // phone: title and outcome, then time and lane.
  import type { RelayCompletedRowView } from '@sensei/state';
  import Pill from './Pill.svelte';
  import { NOW_CARD_ROW, NOW_ROW } from './primitives.js';

  let { row }: { row: RelayCompletedRowView } = $props();
</script>

<li data-completed-row={row.id} class={NOW_CARD_ROW}>
  <a href={row.href} class="{NOW_ROW} hover:bg-paper-soft">
    <span data-part="at" class="order-3 md:order-none flex-[0_0_auto] md:flex-[0_0_44px] font-mono text-xs text-ink-mute">{row.at}</span>
    <span data-part="title" class="order-1 md:order-none min-w-0 text-base flex-[1_1_calc(100%-110px)] md:flex-[1_1_200px] md:truncate">{row.title}</span>
    <span data-part="lane" class="order-4 md:order-none flex-[1_1_auto] md:flex-[0_0_auto] text-xs text-ink-mute">{row.lane}</span>
    <span data-part="outcome" data-tone={row.outcome.tone} class="order-2 md:order-none flex-[0_0_auto]">
      <Pill label={row.outcome.label} tone={row.outcome.tone} />
    </span>
    <span data-part="chevron" aria-hidden="true" class="hidden md:inline text-ink-mute">›</span>
  </a>
</li>
