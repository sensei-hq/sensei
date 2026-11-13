<script lang="ts">
  // Relay · the crew screen (D90; CONTRACT §1a–§1d, §1f, §1g). One screen, two
  // places: the workspace window draws it pre-filtered to its workspace, the main
  // window draws every crew and offers the repository/workspace filters. The
  // difference is the data and the base of a task's link, never the layout.
  //
  // The loader reads; this screen draws what it read and hands every write to
  // `RelayState`, which is the only thing that reads again. The shell (title bar,
  // toolbar, info band, stats, footer) is `RelayShell`, fed by two pure
  // selectors; the body below is the view the toolbar shows. A failed read is a
  // Notice with a Retry — never an empty crew, never "No crew yet" (§1f).
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Loading, Notice } from '@sensei/ui';
  import { failureReason } from '$lib/api.js';
  import { useGlyphMode } from '$lib/glyphs';
  import type { RelayState } from './relay-state.svelte.js';
  import { toRelayShell, type RelayNarrowKey } from './relay-shell.js';
  import { toShellInput } from './relay-shell-input.js';
  import {
    crewGroups,
    crewRepoLabel,
    hhmm,
    laneState,
    providerLabel,
    since,
    taskHref,
  } from './relay-format.js';
  import DecisionRow from './DecisionRow.svelte';
  import MessageRow from './MessageRow.svelte';
  import ComposeBand from './ComposeBand.svelte';
  import CreateSprintForm from './CreateSprintForm.svelte';
  import FloatingReviewRow from './FloatingReviewRow.svelte';
  import QueueRow from './QueueRow.svelte';
  import RelayShell from './RelayShell.svelte';
  import RoleMark from './RoleMark.svelte';
  import SprintBand from './SprintBand.svelte';

  let {
    state,
    viewId,
    base,
    where,
    filtered = false,
  }: {
    state: RelayState;
    viewId: string;
    /** Where this window's Relay lives: `/workspace/<id>/relay`, or `/relay`. */
    base: string;
    /** The place the context line names: the workspace, or "all workspaces" (§1a). */
    where: string;
    /** Draw the repository/workspace filter band (the main window does). */
    filtered?: boolean;
  } = $props();

  const mode = useGlyphMode();

  // ─── The main window's filters (CONTRACT §1a) ─────────────────────
  // The band is a schema form the shell draws (`relay-shell.ts`); its values are
  // the URL's, so a narrowed view is a link the person can keep.
  const current = (key: RelayNarrowKey) => page.url.searchParams.get(key) ?? '';

  function narrow(key: RelayNarrowKey, value: string): void {
    const url = new URL(page.url);
    if (value) url.searchParams.set(key, value);
    else url.searchParams.delete(key);
    // The search change is this route's to answer: `goto` re-runs its load, and
    // nothing above it depends on the filter (N9).
    void goto(`${url.pathname}${url.search}`);
  }

  // The shell, as data: the read and the screen's state → what the header draws.
  const shell = $derived(
    toRelayShell(
      toShellInput({
        surface: 'desktop',
        where,
        now: state.now,
        failed: state.nowError != null,
        reading: state.reading,
        view: state.view,
        composing: state.composing,
        statFilter: state.statFilter,
        queueCount: state.queueCount,
        narrow: filtered ? { repo: current('repo'), workspace: current('workspace') } : null,
      }),
      {
        view: (id) => {
          if (id === 'now' || id === 'queue' || id === 'crew') state.view = id;
        },
        compose: () => (state.composing = !state.composing),
        filter: (id) => (state.statFilter = id),
        narrow,
      },
    ),
  );

  const triageTotal = $derived((state.now?.triage ?? []).reduce((n, t) => n + t.count, 0));
  const triageFrom = $derived(
    (state.now?.triage ?? [])
      .map((t) => (t.provider === 'manual' ? 'you added it' : `${providerLabel(t.provider)} ${t.sprint ?? ''}`.trim()))
      .join(' · '),
  );

  /** The notice a failed crew read draws: a fault, its reason, and a Retry. */
  const failure = $derived(
    state.nowError
      ? {
          detail: failureReason(state.nowError),
          code: state.nowError.status === 0 ? 'daemon_unreachable' : `http_${state.nowError.status}`,
        }
      : null,
  );

  /** The backlog rows the person picked for a sprint, in backlog order. */
  const selectedItems = $derived(
    [...(state.queue?.backlog ?? [])]
      .filter((item) => state.isSelected(item.item_id))
      .sort((a, b) => a.position - b.position),
  );

  /** The gate's defaults: the base branch and verify command this repository uses. The
   *  checkout is the person's to name — the daemon runs the gate on that path. */
  const sprintDefaults = { base_branch: 'develop', base_checkout: '', verify_cmd: 'make test-fast' };

  /** The lanes divided by crew in the main window, flat in a workspace. */
  const grouped = $derived(
    filtered && state.now
      ? crewGroups(state.now.crews, state.now.lanes)
      : state.now
        ? state.now.crews.slice(0, 1).map((crew) => ({ crew, lanes: state.now!.lanes }))
        : [],
  );

  /** The two lines a session pastes to register (CONTRACT §1h, until #424's AddMember). */
  const pasteLines = ['register with sensei as coordinator', 'register with sensei as contributor'];
  async function copy(text: string): Promise<void> {
    await navigator.clipboard?.writeText(text);
  }
</script>

{#snippet composeBand()}
  {#if state.now}
    <ComposeBand
      crews={state.now.crews}
      lanes={state.now.lanes}
      send={(body) => state.send(body)}
      onclose={() => (state.composing = false)}
    />
  {/if}
{/snippet}

<RelayShell
  {shell}
  {viewId}
  glyphs={mode()}
  compose={composeBand}
>
  {#if state.reading}
    <Loading />
  {:else if failure}
    <div class="p-4">
      <Notice
        data-relay-error
        tone="fault"
        size="panel"
        title="Couldn't read the crew"
        detail={failure.detail}
        code={failure.code}
        primary={{ label: 'Retry', name: 'retry', onclick: () => void state.retryNow() }}
      />
    </div>
  {:else if state.now && shell.view === 'crew'}
    <!-- The Crew view (CONTRACT §1h): the roster, the run card and Add a member arrive
         with #424; until then it is the setup — the line a session pastes to join. -->
    <section class="flex flex-col gap-2 p-4" data-relay-section="crew">
      <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">
        {state.hasCrew ? 'Add a lane' : 'Start a run: set up your crew'}
      </h2>
      {#if !state.hasCrew}
        <p class="text-sm font-medium m-0" data-no-crew>No crew yet</p>
      {/if}
      <p class="text-sm text-ink-soft m-0">
        {state.hasCrew
          ? 'Paste the contributor line into a new session in this repository; it registers and takes the next piece of work.'
          : 'Register a session in this repository as the coordinator, then the contributors it invites.'}
      </p>
      {#each pasteLines as line (line)}
        <div class="flex items-center gap-2" data-paste-line>
          <code class="text-xs rounded-sm px-2 py-1 bg-paper-soft" data-paste-text>{line}</code>
          <button type="button" class="text-xs underline" data-paste-copy onclick={() => void copy(line)}>Copy</button>
        </div>
      {/each}
      {#if !state.hasCrew}
        <p class="text-xs text-ink-faint m-0">Listening · 0 of 2 to start</p>
      {/if}
    </section>
  {:else if state.now && state.hasCrew && shell.view === 'now'}
    {#if state.now.decisions.length > 0 || triageTotal > 0 || state.now.person_messages.length > 0}
      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="needs-you">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Needs you</h2>
        {#each state.now.decisions as decision (decision.id)}
          <DecisionRow
            {decision}
            href={decision.task_id ? taskHref(base, decision.crew_id, decision.task_id) : null}
            onanswer={(option) => state.answer(decision, option)}
          />
        {/each}
        {#each state.now.person_messages as message (message.id)}
          <MessageRow
            {message}
            href={message.task_id ? taskHref(base, message.crew_id, message.task_id) : null}
          />
        {/each}
        {#if triageTotal > 0}
          <button
            type="button"
            class="text-left text-sm text-ink-soft hover:text-ink"
            data-triage-line
            onclick={() => state.openQueue()}
          >
            {triageTotal} to triage — from {triageFrom}
          </button>
        {/if}
      </section>
    {/if}

    {#if state.now.floating_reviews.length > 0}
      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="reviews">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">
          Needs a reviewer · {state.now.floating_reviews.length}
        </h2>
        {#each state.now.floating_reviews as review (review.task_id)}
          <FloatingReviewRow {review} href={taskHref(base, review.crew_id, review.task_id)} />
        {/each}
      </section>
    {/if}

    {#if state.now.runs.length > 0}
      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="sprints">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Sprints · {state.now.runs.length}</h2>
        {#each state.now.runs as run (run.run_id)}
          <SprintBand {run} />
        {/each}
      </section>
    {/if}

    <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="lanes">
      <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Lanes · {state.now.lanes.length}</h2>
      <!-- The workspace window draws one crew's lanes flat; the main window divides
           them by crew, because a lane code is unique per crew only (§1c). -->
      {#each grouped as group (group.crew.id)}
        <div data-crew-group={group.crew.id}>
          <p class="text-xs text-ink-faint m-0" data-crew-heading>{crewRepoLabel(group.crew)}</p>
          {#each group.lanes as lane (lane.id)}
            {@const lost = laneState(lane)}
            <div class="border-t border-paper-edge py-2" data-lane-row={lane.code}>
              <div class="flex items-center gap-2" data-row-line="1">
                <RoleMark role={lane.role} />
                <span class="font-medium text-sm">{lane.title ?? lane.code}</span>
                <span class="text-xs text-ink-mute">{lane.code}</span>
                <span class="text-xs text-ink-faint">
                  {lane.family}{lane.version ? ` ${lane.version}` : ''} · {lane.model ?? 'model not reported'}
                </span>
                <span class="ml-auto text-xs rounded-sm px-2 py-1 bg-paper-soft text-ink-soft" data-state>{lost}</span>
                {#if lost === 'lease lost' && lane.lease_expires_at}
                  <span class="text-xs text-warning">at {hhmm(lane.lease_expires_at)}</span>
                {/if}
                {#if lost === 'paused'}
                  <span class="text-xs text-warning"
                    >{lane.limit_scope} · {lane.limit_resumes_at
                      ? `until ${hhmm(lane.limit_resumes_at)}`
                      : 'resume unknown'}</span
                  >
                {/if}
              </div>
              <div class="text-sm text-ink-soft" data-row-line="2">
                {#if lane.task}
                  <a class="hover:underline" href={taskHref(base, lane.crew_id, lane.task.id)}>{lane.task.title}</a>
                  <span class="text-xs text-ink-faint"> · {since(lane.task.state_changed_at)}</span>
                {:else if !lane.reviewing}
                  Nothing claimed
                {/if}
                {#if lane.reviewing}
                  <span data-reviewing={lane.reviewing.id}>
                    {#if lane.task}<span class="text-xs text-ink-faint"> · </span>{/if}
                    <span class="text-xs text-ink-mute">reviewing</span>
                    <a class="hover:underline" href={taskHref(base, lane.crew_id, lane.reviewing.id)}>{lane.reviewing.title}</a>
                    <span class="text-xs text-ink-faint"> · {since(lane.reviewing.claimed_at)}</span>
                  </span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/each}
    </section>

    {#if state.now.completed_today.length > 0}
      <section class="flex flex-col gap-2 p-4" data-relay-section="completed">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">
          Completed today · {state.now.completed_today.length}
        </h2>
        {#each state.now.completed_today as row (row.task_id)}
          <div class="flex items-center gap-2 text-sm" data-completed-row>
            <span class="text-xs text-ink-faint tabular-nums">{hhmm(row.completed_at)}</span>
            <a class="hover:underline" href={taskHref(base, row.crew_id, row.task_id)}>{row.title}</a>
            <span class="text-xs text-ink-mute">{row.lane_title ?? row.lane_code}</span>
            <span class="ml-auto text-xs rounded-sm px-2 py-1 bg-paper-soft text-ink-soft" data-state>{row.outcome}</span>
          </div>
        {/each}
      </section>
    {/if}
  {:else if state.now && shell.view === 'queue'}
    {#if state.readingQueue}
      <Loading />
    {:else if state.queueError}
      <div class="p-4">
        <Notice
          data-relay-queue-error
          tone="fault"
          size="panel"
          title="Couldn't read the queue"
          detail={failureReason(state.queueError)}
          code={state.queueError.status === 0 ? 'daemon_unreachable' : `http_${state.queueError.status}`}
          primary={{ label: 'Retry', name: 'retry', onclick: () => void state.retryQueue() }}
        />
      </div>
    {:else if state.queue}
      {#if state.creatingSprint}
        <CreateSprintForm
          selected={selectedItems}
          defaults={sprintDefaults}
          error={state.sprintError}
          onsubmit={(draft) => state.createSprint(draft)}
          oncancel={() => {
            state.creatingSprint = false;
            state.sprintError = null;
          }}
        />
      {/if}
      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="triage">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Triage · {state.queue.triage.length}</h2>
        {#if state.addError}
          <Notice size="inline" tone="fault" title={state.addError} data-add-error />
        {/if}
        {#each state.queue.triage as item (item.item_id ?? item.ticket?.key)}
          <QueueRow
            id={item.item_id ?? item.ticket?.key ?? ''}
            title={item.title}
            kind={item.kind}
            tags={item.tags}
            sprint={item.sprint}
            addedByYou={item.added_by_you}
            ticket={item.ticket}
            onadd={() => state.addToBacklog(item)}
          />
        {/each}
      </section>

      <section class="flex flex-col gap-2 p-4" data-relay-section="backlog">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Backlog · {state.queue.backlog.length}</h2>
        {#each [...state.queue.backlog].sort((a, b) => a.position - b.position) as item (item.item_id)}
          <QueueRow
            id={item.item_id}
            title={item.title}
            kind={item.kind}
            tags={item.tags}
            sprint={item.sprint}
            addedByYou={item.added_by_you}
            ticket={item.ticket}
            selectable
            selected={state.isSelected(item.item_id)}
            ontoggle={() => state.toggleSelected(item.item_id)}
          />
        {/each}
      </section>
    {/if}
  {/if}
</RelayShell>
