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
  import type { RelayState } from '@sensei/state';
  import { toCompletedRows, toLaneGroups, toNeedsYou, toQueueRows, toWaitRows, type NowRowOptions } from '@sensei/state';
  import { WAIT_THRESHOLD_MIN, toRelayShell, type RelayNarrowKey } from './relay-shell.js';
  import { toShellInput } from './relay-shell-input.js';
  import { taskHref } from './relay-format.js';
  import ComposeBand from './ComposeBand.svelte';
  import CreateSprintForm from './CreateSprintForm.svelte';
  import FloatingReviewRow from './FloatingReviewRow.svelte';
  import QueueSection from './QueueSection.svelte';
  import LaneList from './LaneList.svelte';
  import CompletedList from './CompletedList.svelte';
  import NeedsYou from './NeedsYou.svelte';
  import SectionHeading from './SectionHeading.svelte';
  import WaitList from './WaitList.svelte';
  import { NOW_CARD } from './primitives.js';
  import RelayShell from './RelayShell.svelte';
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

  // Now's lists, from the @sensei/state selectors (#422): the main window divides the
  // lanes by crew, because a lane code is unique per crew only (§1c); a workspace
  // draws its crew flat.
  const rowOptions = $derived<NowRowOptions>({
    at: Date.now(),
    threshold: WAIT_THRESHOLD_MIN,
    href: (crewId, taskId) => taskHref(base, crewId, taskId),
    byCrew: filtered,
  });
  /** The pressed stat, choosing what Now shows (§1c-0). */
  const filter = $derived(state.statFilter);
  const FILTER_LABEL = { working: 'Working', waiting: 'Waiting', blocked: 'Blocked', done: 'Completed today' } as const;
  const showing = $derived(filter ? { label: FILTER_LABEL[filter], onclear: () => (state.statFilter = null) } : null);
  const laneGroups = $derived(
    toLaneGroups(state.now, { ...rowOptions, only: filter === 'working' || filter === 'blocked' ? filter : undefined }),
  );
  const waitRows = $derived(toWaitRows(state.now, rowOptions));
  const needsYou = $derived(toNeedsYou(state.now, state.queue, rowOptions));

  // The Queue's rows (#423): Triage as read, the backlog in its order.
  const queueRows = $derived(toQueueRows(state.queue));

  /** Add a triage row to the backlog, found by its row id in the read. */
  function addRow(id: string): Promise<string | null> {
    const item = state.queue?.triage.find((t) => (t.item_id ?? `${t.ticket.provider}:${t.ticket.key}`) === id);
    return item ? state.addToBacklog(item) : Promise.resolve('This item is no longer in triage.');
  }

  /** Answer a decision by its id (Needs you calls back with it). */
  function answer(id: string, option: string): Promise<string | null> {
    const decision = state.now?.decisions.find((d) => d.id === id);
    return decision ? state.answer(decision, option) : Promise.resolve('This decision is no longer open.');
  }
  const completedRows = $derived(toCompletedRows(state.now, rowOptions));

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
    <!-- Now (CONTRACT §1c): what needs the person, then the lanes, then today's
         completions. A pressed stat chooses what shows (§1c-0); Needs you hides
         while one is on. -->
    {#if needsYou && !filter}
      <div class="p-4 pb-0">
        <NeedsYou
          view={needsYou}
          href={(crewId, taskId) => taskHref(base, crewId, taskId)}
          onanswer={(id, option) => answer(id, option)}
          onopenqueue={() => state.openQueue()}
        />
      </div>
    {/if}

    {#if !filter && state.now.floating_reviews.length > 0}
      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="reviews">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">
          Needs a reviewer · {state.now.floating_reviews.length}
        </h2>
        {#each state.now.floating_reviews as review (review.task_id)}
          <FloatingReviewRow {review} href={taskHref(base, review.crew_id, review.task_id)} />
        {/each}
      </section>
    {/if}

    {#if !filter && state.now.runs.length > 0}
      <section class="flex flex-col gap-2 p-4 pb-0" data-relay-section="sprints">
        <h2 class="text-xs uppercase tracking-wide text-ink-mute m-0">Sprints · {state.now.runs.length}</h2>
        {#each state.now.runs as run (run.run_id)}
          <SprintBand {run} />
        {/each}
      </section>
    {/if}

    {#if filter === 'waiting'}
      <section aria-label="Waiting · oldest first" data-relay-section="waiting" class="flex flex-col gap-1.5 p-4">
        <SectionHeading label="Waiting · oldest first" count={waitRows.length} {showing} />
        <div class="{NOW_CARD} px-3.5"><WaitList rows={waitRows} /></div>
      </section>
    {:else if filter !== 'done'}
      <div class="p-4 pb-0">
        <LaneList groups={laneGroups} {showing} />
      </div>
    {/if}

    {#if (!filter || filter === 'done') && completedRows.length > 0}
      <div class="p-4">
        <CompletedList rows={completedRows} showing={filter === 'done' ? showing : null} />
      </div>
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
      <div class="flex flex-col gap-5 p-4">
        <QueueSection
          id="triage"
          label="Triage"
          kanji="選"
          description="From your active sprints in Jira, Azure DevOps and GitHub, and open items you add."
          rows={queueRows.triage}
          onadd={(row) => addRow(row.id)}
        >
          {#if state.addError}
            <Notice size="inline" tone="fault" title={state.addError} data-add-error />
          {/if}
        </QueueSection>
        <QueueSection
          id="backlog"
          label="Backlog"
          kanji="列"
          description="Your tasks, in order. The crew's coordinator takes the top one next and puts it on a lane."
          rows={queueRows.backlog}
          isSelected={(id) => state.isSelected(id)}
          ontoggle={(row) => state.toggleSelected(row.id)}
        />
      </div>
    {/if}
  {/if}
</RelayShell>
