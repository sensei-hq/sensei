<script lang="ts">
  // The task view's header (COMPONENTS §3d TaskHeader; CONTRACT §1e): back to where the
  // task was opened from, the title in the display face with its state, the ticket as a
  // pill linking out (#381), and one line of facts — key muted, value in ink.
  import type { RelayTaskHeaderView } from '@sensei/state';
  import Pill from './Pill.svelte';

  let { header }: { header: RelayTaskHeaderView } = $props();
</script>

<header data-task-header class="flex flex-col gap-2">
  <a data-back href={header.back.href} class="w-fit text-sm text-accent no-underline hover:underline">
    <span aria-hidden="true">‹&nbsp;</span>{header.back.label}
  </a>
  <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
    <h2 data-task-title class="m-0 display text-xl font-medium">{header.title}</h2>
    <span data-task-state><Pill label={header.state.label} tone={header.state.tone} /></span>
    {#if header.ticket}
      <a
        data-ticket
        href={header.ticket.url ?? undefined}
        target="_blank"
        rel="noopener noreferrer"
        class="ml-auto inline-flex items-center gap-1.5 min-h-8 px-3 rounded-full border border-solid border-paper-edge bg-paper text-sm text-ink no-underline"
      >
        <span data-ticket-provider class="text-ink-soft">{header.ticket.provider}</span>
        <span class="font-mono">{header.ticket.key}</span>
        <span aria-hidden="true" class="i-solar-arrow-right-up-linear text-sm text-ink-soft"></span>
      </a>
    {/if}
  </div>
  <dl data-task-facts class="m-0 flex flex-wrap gap-x-6 gap-y-1 text-sm">
    {#each header.facts as fact (fact.k)}
      <div data-task-fact class="flex gap-1.5">
        <dt class="text-ink-mute">{fact.k}</dt>
        <dd class="m-0 text-ink">{fact.v}</dd>
      </div>
    {/each}
  </dl>
</header>
