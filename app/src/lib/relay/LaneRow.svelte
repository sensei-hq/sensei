<script lang="ts">
  // One lane (COMPONENTS §3b LaneRow; CONTRACT §1c-2): role mark · objective over
  // codename · assistant · the one task · state pill · time in state · chevron. A
  // lane on a task is one link to it; an idle lane is no link and has no chevron.
  // On a phone: mark, name and pill, then task and time, no chevron.
  import type { RelayLaneRowView } from '@sensei/state';
  import Pill from './Pill.svelte';
  import RoleMark from './RoleMark.svelte';
  import { NOW_CARD_ROW, NOW_ROW } from './primitives.js';

  let { row }: { row: RelayLaneRowView } = $props();
</script>

{#snippet parts()}
  <span data-part="mark" class="flex-[0_0_24px]"><RoleMark role={row.role} /></span>
  <span data-part="name" class="order-1 md:order-none flex flex-col min-w-0 flex-[1_1_calc(100%-190px)] md:flex-[0_0_170px]">
    <strong class="text-sm font-semibold truncate">{row.name}</strong>
    <span class="text-xs text-ink-mute truncate">{row.who}</span>
  </span>
  <span
    data-part="task"
    class="order-4 md:order-none mt-0.5 md:mt-0 min-w-0 text-base flex-[1_1_calc(100%-100px)] md:flex-[1_1_200px] md:truncate"
  >{row.task}</span>
  <span data-part="state" data-tone={row.state.tone} class="order-2 md:order-none flex-[0_0_auto]">
    <Pill label={row.state.label} tone={row.state.tone} />
  </span>
  <span
    data-part="when"
    data-over={row.when.over}
    class={[
      'order-5 md:order-none flex-[0_0_auto] text-xs md:min-w-[70px] md:text-right tabular-nums',
      row.when.over ? 'text-warning font-semibold' : 'text-ink-mute',
    ]}
  >{row.when.text}</span>
  {#if row.href}
    <span data-part="chevron" aria-hidden="true" class="hidden md:inline text-ink-mute">›</span>
  {:else}
    <!-- No link, no chevron — but its room, so the pill and time stay in the column. -->
    <span data-chevron-space aria-hidden="true" class="hidden md:inline invisible">›</span>
  {/if}
{/snippet}

<li data-lane-row={row.code} class={NOW_CARD_ROW}>
  {#if row.href}
    <a href={row.href} class="{NOW_ROW} hover:bg-paper-soft">{@render parts()}</a>
  {:else}
    <div class={NOW_ROW}>{@render parts()}</div>
  {/if}
</li>
