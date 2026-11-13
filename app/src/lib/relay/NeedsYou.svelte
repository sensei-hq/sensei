<script lang="ts">
  // Needs you (CONTRACT §1c-1; COMPONENTS §3b): only when something does — the
  // decisions, the waits at the threshold as one card, the lanes' messages, the
  // triage line (it opens the Queue), and the tasks no lane has taken (read-only:
  // placing them is the coordinator's). It draws toNeedsYou's data; writes call back.
  import type { RelayNeedsYouView, RelayWaitRowView } from '@sensei/state';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import DecisionRow from './DecisionRow.svelte';
  import MessageRow from './MessageRow.svelte';
  import SectionHeading from './SectionHeading.svelte';
  import WaitList from './WaitList.svelte';

  let {
    view,
    href,
    onanswer,
    onopenqueue,
    onnudge,
  }: {
    view: RelayNeedsYouView;
    href: (crewId: string, taskId: string) => string;
    onanswer: (decisionId: string, option: string) => Promise<string | null>;
    onopenqueue: () => void;
    onnudge?: (row: RelayWaitRowView) => void;
  } = $props();

  const mode = useGlyphMode();
</script>

{#snippet mark(kanji: string, tone: string, size = 'text-sm')}
  {#if mode() === 'icon'}
    <span aria-hidden="true" class="{KANJI_ICONS[kanji]} {tone} {size}"></span>
  {:else}
    <span aria-hidden="true" class="font-kanji {tone} {size}">{kanji}</span>
  {/if}
{/snippet}

<section aria-label="Needs you" data-relay-section="needs-you" class="flex flex-col gap-2">
  <h3 data-section-heading class="m-0 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.12em] text-ink-mute">
    {@render mark('問', 'text-warning')}Needs you&nbsp;·&nbsp;{view.count}
  </h3>
  {#each view.decisions as decision (decision.id)}
    <DecisionRow
      {decision}
      href={decision.task_id ? href(decision.crew_id, decision.task_id) : null}
      onanswer={(option) => onanswer(decision.id, option)}
    />
  {/each}
  {#if view.stale}
    <article
      data-stale-waits
      class="flex flex-col gap-2 p-3.5 rounded-xl border border-solid border-warning bg-[color-mix(in_oklch,var(--warning)_8%,var(--paper))]"
    >
      <strong data-stale-title class="flex items-center gap-1.5 text-base font-semibold">
        {@render mark('待', 'text-warning', 'text-base')}{view.stale.title}
      </strong>
      <WaitList rows={view.stale.rows} {onnudge} />
    </article>
  {/if}
  {#each view.messages as message (message.id)}
    <MessageRow {message} href={message.task_id ? href(message.crew_id, message.task_id) : null} />
  {/each}
  {#if view.triage}
    <button
      type="button"
      data-triage-line
      onclick={onopenqueue}
      class="flex flex-wrap items-center gap-x-3 gap-y-1.5 py-3 px-3.5 rounded-xl border border-solid border-paper-edge bg-paper text-ink text-left cursor-pointer"
    >
      {@render mark('選', 'text-accent')}
      <strong class="text-base font-semibold">{view.triage.count} to triage</strong>
      <span class="flex-[1_1_260px] text-sm text-ink-soft">{view.triage.from}</span>
      <span aria-hidden="true" class="text-ink-mute">›</span>
    </button>
  {/if}
  {#if view.unassigned}
    <div
      data-unassigned
      class="flex flex-wrap items-center gap-x-3 gap-y-1.5 py-3 px-3.5 rounded-xl border border-solid border-paper-edge bg-paper-soft text-ink"
    >
      <strong class="text-base font-semibold">Unassigned · {view.unassigned.count}</strong>
      <span class="flex-[1_1_260px] text-sm text-ink-soft">{view.unassigned.from} — the coordinator puts these on a lane.</span>
    </div>
  {/if}
</section>
