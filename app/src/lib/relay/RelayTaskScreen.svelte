<script lang="ts">
  // Relay · one task (D90; CONTRACT §1e, `relay.task_history`). Back link, title,
  // state and ticket; one facts line; the timeline (F8, oldest first, entries
  // expand); Run details folded; the task's open decision; Handed off; then the
  // history, newest first, down to Assigned.
  //
  // Shared by both windows: the loader decides what was read, the base decides
  // where the back link and the view id point.
  import { Loading, Notice, ShellScreen } from '@sensei/ui';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import { toTaskView, type RelayTaskState } from '@sensei/state';
  import DecisionRow from './DecisionRow.svelte';
  import HandoffCard from './HandoffCard.svelte';
  import RunDetails from './RunDetails.svelte';
  import TaskHeader from './TaskHeader.svelte';
  import HistoryEntry from './HistoryEntry.svelte';
  import RelayChecklist from './RelayChecklist.svelte';
  import RelayTimeline from './RelayTimeline.svelte';

  let {
    state,
    viewId,
    backHref,
    context,
  }: {
    state: RelayTaskState;
    viewId: string;
    backHref: string;
    context: string;
  } = $props();

  const mode = useGlyphMode();
  const view = $derived(state.view);
  const task = $derived(view?.task ?? null);

  // The header, Run details and Handed off, from the @sensei/state selector (#425).
  const parts = $derived(view ? toTaskView(view, { back: { label: 'Relay', href: backHref } }) : null);

  /** A message's receipts, folded by the message they reply to (§1e.6, F5). */
  const receipts = $derived.by(() => {
    const out: Record<string, 'sent' | 'delivered' | 'read'> = {};
    for (const a of view?.history ?? []) {
      if (!a.reply_to) continue;
      if (a.kind === 'read') out[a.reply_to] = 'read';
      else if (a.kind === 'delivered' && out[a.reply_to] !== 'read') out[a.reply_to] = 'delivered';
    }
    return out;
  });

  /** Receipts are not entries of their own: they fold into their message. */
  const entries = $derived((view?.history ?? []).filter((a) => a.kind !== 'delivered' && a.kind !== 'read'));
</script>

<ShellScreen
    data-screen="relay"
    {viewId}
    glyphs={mode()}
    mark={{ kanji: '継', icon: KANJI_ICONS['継'] }}
    title={task?.title ?? 'Relay'}
    {context}
    search={false}
  >
    <div class="flex flex-col gap-3 p-4">
      {#if !parts || state.reading || state.notFound || state.error}
        <!-- No task drawn (reading, not found, a failed read): the header's back link is
             not there, so the way back is. -->
        <a data-back href={backHref} class="w-fit text-sm text-accent no-underline hover:underline"><span aria-hidden="true">‹&nbsp;</span>Relay</a>
      {/if}
      {#if state.reading}
        <Loading />
      {:else if state.notFound}
        <Notice
          data-relay-not-found
          tone="refusal"
          size="panel"
          title="This task isn't here."
          detail="It may belong to another crew, another person, or have been removed."
          code="http_404"
        />
      {:else if state.error}
        <!-- A failed re-read keeps the old view; drawing it would pass stale data off as
             fresh, so the fault comes first and Retry reads again (D83, N8). -->
        <Notice
          data-relay-error
          tone="fault"
          size="panel"
          title="Couldn't read this task"
          detail={state.reason ?? ''}
          code={state.error.status === 0 ? 'daemon_unreachable' : `http_${state.error.status}`}
          primary={{ label: 'Retry', name: 'retry', onclick: () => void state.retry() }}
        />
      {:else if task && view}
        <TaskHeader header={parts!.header} />

        <RelayChecklist steps={view.steps} />

        <RelayTimeline {view} />

        <RunDetails run={parts!.run} />

        {#if view.decision}
          <DecisionRow
            decision={view.decision}
            href={null}
            onanswer={(option) => state.answer(view.decision!, option)}
          />
        {/if}

        {#if parts?.handoff}
          <HandoffCard handoff={parts.handoff} />
        {/if}

        <section class="flex flex-col gap-2" data-relay-section="history">
          <h3 class="text-xs uppercase tracking-wide text-ink-mute m-0">History</h3>
          {#each entries as action (action.id)}
            <HistoryEntry {action} lanes={view.lanes} messageState={receipts[action.id] ?? (action.kind === 'message' ? 'sent' : null)} />
          {/each}
        </section>
      {/if}
    </div>
  </ShellScreen>
