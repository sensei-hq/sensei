<script lang="ts">
  // Relay · one task (D90; CONTRACT §1e, `relay.task_history`). Back link, title,
  // state and ticket; one facts line; Run details folded; the task's open decision;
  // Handed off; then the history, newest first, down to Assigned.
  //
  // Shared by both windows: the loader decides what was read, the base decides
  // where the back link and the view id point.
  import { Loading, Notice, ShellScreen } from '@sensei/ui';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import type { RelayRunDetails, RelayTaskView } from '$lib/relay-api.js';
  import type { RelayTaskState } from './relay-task-state.svelte.js';
  import { hhmm, providerLabel, roundsLabel, taskStateLabel } from './relay-format.js';
  import DecisionRow from './DecisionRow.svelte';
  import HistoryEntry from './HistoryEntry.svelte';

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

  const num = (n: number | null | undefined) => (n == null ? 'not reported' : new Intl.NumberFormat('en').format(n));
  const secs = (s: number | null | undefined) =>
    s == null ? 'not reported' : s < 60 ? `${s} s` : `${Math.round(s / 60)} min`;

  /** Run details, in the contract's order (§1e.3). */
  const runRows = $derived.by(() => {
    const r: RelayRunDetails | undefined = view?.run;
    if (!r) return [];
    return [
      { key: 'repository', label: 'Repository', value: r.repository ?? 'not reported' },
      { key: 'branch', label: 'Branch', value: r.branch ?? 'not reported' },
      { key: 'commits', label: 'Commits', value: r.commits?.join(', ') ?? 'not reported' },
      { key: 'model', label: 'Model', value: r.model ?? 'not reported' },
      { key: 'tokens_in', label: 'Tokens in', value: num(r.tokens_in) },
      { key: 'tokens_out', label: 'Tokens out', value: num(r.tokens_out) },
      { key: 'tokens_cache', label: 'Tokens cached', value: num(r.tokens_cache) },
      { key: 'context_tokens', label: 'Peak context', value: num(r.context_tokens) },
      { key: 'compactions', label: 'Compactions', value: num(r.compactions) },
      { key: 'peak_memory_mb', label: 'Peak memory', value: r.peak_memory_mb == null ? 'not reported' : `${r.peak_memory_mb} MB` },
      { key: 'subagents', label: 'Subagents', value: num(r.subagents) },
      { key: 'working_secs', label: 'Working', value: secs(r.working_secs) },
      { key: 'waiting_secs', label: 'Waiting for review', value: secs(r.waiting_secs) },
      { key: 'files_changed', label: 'Files changed', value: `${num(r.files_changed)} · ${num(r.files_outside_owned)} outside owned paths` },
      { key: 'gates_run', label: 'Gates run', value: num(r.gates_run) },
    ];
  });

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

<div data-screen="relay">
  <ShellScreen
    {viewId}
    glyphs={mode()}
    mark={{ kanji: '継', icon: KANJI_ICONS['継'] }}
    title={task?.title ?? 'Relay'}
    {context}
    search={false}
  >
    {#snippet footer()}
      <a class="text-xs underline" data-back href={backHref}>← Relay</a>
    {/snippet}

    <div class="flex flex-col gap-3 p-4">
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
        <header class="flex flex-wrap items-center gap-2">
          <h2 class="text-lg font-medium m-0" data-task-title>{task.title}</h2>
          <span class="text-xs rounded-sm bg-paper-soft px-2 py-0.5 text-ink-soft" data-task-state>
            {taskStateLabel(task.state)}
          </span>
          {#if task.ticket}
            <a
              class="ml-auto inline-flex items-center gap-1 text-xs underline"
              data-ticket
              href={task.ticket.url ?? undefined}
              target="_blank"
              rel="noopener noreferrer"
            >
              <span class="hidden sm:inline" data-ticket-provider>{providerLabel(task.ticket.provider)}</span>
              {task.ticket.key}
            </a>
          {/if}
        </header>

        <p class="text-xs text-ink-mute m-0" data-task-facts>
          {task.lane ? `${task.lane.title ?? task.lane.code} · ${task.lane.code}` : 'No lane'} ·
          {task.lane?.family ?? 'member not reported'}{task.lane?.version ? ` ${task.lane.version}` : ''} ·
          {task.lane?.model ?? 'model not reported'} ·
          {#if task.lane?.limit_scope}paused · {task.lane.limit_scope}
            {task.lane.limit_resumes_at ? `until ${hhmm(task.lane.limit_resumes_at)}` : 'resume unknown'} ·{/if}
          {hhmm(task.assigned_at)} → {task.completed_at ? hhmm(task.completed_at) : 'now'} ·
          {roundsLabel(task.rounds)}
        </p>

        <details class="rounded-sm border border-paper-edge p-3" data-run-details>
          <summary class="text-sm cursor-pointer">Run details</summary>
          <div class="mt-2 grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-1 text-xs text-ink-soft">
            {#each runRows as row (row.key)}
              <span class="flex justify-between gap-3" data-run-detail={row.key}>
                <span class="text-ink-mute">{row.label}</span>
                <span>{row.value}</span>
              </span>
            {/each}
          </div>
          <p class="text-xs text-ink-faint mt-2 m-0">
            Totals over the task's turns · {view.run.reported_turns} of {view.run.turns} reported
          </p>
        </details>

        {#if view.decision}
          <DecisionRow
            decision={view.decision}
            href={null}
            onanswer={(option) => state.answer(view.decision!, option)}
          />
        {/if}

        {#if view.handoff}
          <section class="rounded-sm border border-paper-edge p-3 flex flex-col gap-2" data-handoff>
            <h3 class="text-xs uppercase tracking-wide text-ink-mute m-0">Handed off</h3>
            <p class="text-sm text-ink-soft m-0">{view.handoff.body_md}</p>
            <div class="flex flex-col gap-1">
              {#each view.handoff.docs as doc (doc)}
                <code class="text-xs w-fit" data-handoff-doc>{doc}</code>
              {/each}
            </div>
          </section>
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
</div>
