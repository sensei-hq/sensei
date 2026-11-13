// Relay · test fixtures (test-only: never exported from the package index; the app's
// `test-stubs/relay-fixtures.ts` re-exports them for its specs).
//
// Example data from the mockup (`coord/mockups/relay-lanes/Relay.dc.html`), shaped
// as the daemon's relay views answer it (`./types.ts`). Two crews, two
// workspaces and two of each row kind, so a screen that reads the wrong one, or
// only the first, fails.
import type { ApiResult } from '../result.js';
import type {
  RelayAction,
  RelayPresence,
  RelayActionKind,
  RelayActionProps,
  RelayBacklogItem,
  RelayCompletedTask,
  RelayCrew,
  RelayDecision,
  RelayFloatingReview,
  RelayLane,
  RelayNow,
  RelayPersonMessage,
  RelayQueue,
  RelayRunDetails,
  RelayRunSummary,
  RelayTaskView,
  RelayTriageItem,
} from './types.js';

type TrackerTriage = Extract<RelayTriageItem, { item_id: null }>;
type ManualTriage = Extract<RelayTriageItem, { ticket: null }>;

export const CREW_ID = '019a5e2c-4b1d-7c00-8d2e-00000000c7f3';
export const OTHER_CREW_ID = '019a5e2c-4b1d-7c00-8d2e-0000000099aa';
/** A crew on a repository that belongs to no workspace (67 of 157 repositories here). */
export const LOOSE_CREW_ID = '019a5e2c-4b1d-7c00-8d2e-00000000b10c';

export const REPO = 'github.com/sensei-hq/sensei';
export const LOOSE_REPO = 'github.com/jerrythomas/scratch';

/** Lane ids are uuids (L0 ruling 3); the screen shows the code. */
export const LANE = {
  L0: '019a5e2c-4b1d-7c00-8d2e-0000000000a0',
  L1: '019a5e2c-4b1d-7c00-8d2e-0000000000a1',
  L2: '019a5e2c-4b1d-7c00-8d2e-0000000000a2',
  L3: '019a5e2c-4b1d-7c00-8d2e-0000000000a3',
  L4: '019a5e2c-4b1d-7c00-8d2e-0000000000a4',
  L5: '019a5e2c-4b1d-7c00-8d2e-0000000000a5',
  L8: '019a5e2c-4b1d-7c00-8d2e-0000000000a8',
  L9: '019a5e2c-4b1d-7c00-8d2e-0000000000a9',
} as const;

export const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();

export const ok = <T>(data: T): ApiResult<T> => ({ ok: true, data });
export const fail = (status: number, message: string): ApiResult<never> => ({
  ok: false,
  error: { status, message },
});

export function lane(over: Partial<RelayLane> = {}): RelayLane {
  const row: Omit<RelayLane, 'presence'> = {
    id: LANE.L1,
    crew_id: CREW_ID,
    code: 'L1',
    title: 'Diagrams',
    role: 'contributor',
    family: 'claude-code',
    version: '2.1',
    model: 'claude-opus-5-5',
    lease_state: 'granted',
    lease_expires_at: null,
    limit_scope: null,
    limit_resumes_at: null,
    limit_note: null,
    suggested_name: null,
    reviewing: null,
    task: {
      id: 't-structure',
      title: 'Structure view on the shell',
      repo_key: REPO,
      workspace_id: 'w1',
      phase: 'tests',
      state: 'working',
      round: 1,
      state_changed_at: minutesAgo(38),
      current_step: 'Writing the failing specs',
      steps_done: 2,
      steps_total: 5,
      needs_answer: false,
    },
    ...over,
  };
  // The daemon's presence rule (relay.now, #419), unless a spec states one.
  const presence: RelayPresence =
    over.presence ?? (row.lease_state !== 'granted' ? 'not_responding' : row.task || row.reviewing ? 'active' : 'idle');
  return { ...row, presence };
}

export const coordinatorLane = (over: Partial<RelayLane> = {}) =>
  lane({
    id: LANE.L0,
    code: 'L0',
    title: 'Planning & review',
    role: 'coordinator',
    task: {
      id: 't-review-er',
      title: 'Review · ER sidebar on the shell',
      repo_key: REPO,
      workspace_id: 'w1',
      phase: 'impl',
      state: 'in_review',
      round: 1,
      state_changed_at: minutesAgo(24),
      current_step: null,
      steps_done: 0,
      steps_total: 0,
      needs_answer: false,
    },
    ...over,
  });

export function decision(over: Partial<RelayDecision> = {}): RelayDecision {
  return {
    id: 'ask-1',
    crew_id: CREW_ID,
    task_id: 't-logs',
    task_title: 'Activity logs',
    question: 'Where does the activity log live?',
    context: 'Activity logs is waiting on this.',
    options: [
      { id: 'db', label: "In the daemon's database", detail: null },
      { id: 'file', label: 'In a file per workspace', detail: null },
    ],
    recommended: 'db',
    asked_at: minutesAgo(5),
    ...over,
  };
}

export function personMessage(over: Partial<RelayPersonMessage> = {}): RelayPersonMessage {
  return {
    id: 'msg-1',
    crew_id: CREW_ID,
    task_id: 't-logs',
    task_title: 'Activity logs',
    from_lane_code: 'L2',
    text: 'The migration needs your eyes before I merge.',
    sent_at: minutesAgo(5),
    ...over,
  };
}

export function completed(over: Partial<RelayCompletedTask> = {}): RelayCompletedTask {
  return {
    task_id: 't-header',
    crew_id: CREW_ID,
    repo_key: REPO,
    title: 'Diagrams header on ScreenShell',
    lane_code: 'L1',
    lane_title: 'Diagrams',
    outcome: 'merged',
    completed_at: minutesAgo(60),
    ...over,
  };
}

export const RUN_ID = '019a5e2c-4b1d-7c00-8d2e-00000000ab10';
export const OTHER_RUN_ID = '019a5e2c-4b1d-7c00-8d2e-00000000ab11';

/** An open sprint run as `relay.now`'s `sprint_summary` counts it (queue cutover). */
export function sprint(over: Partial<RelayRunSummary> = {}): RelayRunSummary {
  return {
    run_id: RUN_ID,
    crew_id: CREW_ID,
    title: 'Sprint 14: Broker hardening',
    goal: 'Resolve the high-severity broker findings',
    state: 'active',
    created_at: minutesAgo(90),
    started_at: minutesAgo(60),
    total_tasks: 6,
    ready_tasks: 2,
    working_tasks: 1,
    in_review_tasks: 1,
    floating_reviews: 1,
    approved_tasks: 0,
    done_tasks: 2,
    blocked_tasks: 0,
    interrupted_tasks: 0,
    ...over,
  };
}

/** A report waiting for a reviewer, as `relay.now`'s `floating_review` row (queue cutover). */
export function floatingReview(over: Partial<RelayFloatingReview> = {}): RelayFloatingReview {
  return {
    task_id: 't-claim-handlers',
    crew_id: CREW_ID,
    run_id: RUN_ID,
    title: 'Add claim handlers',
    phase: 'impl',
    round: 1,
    repo_key: REPO,
    author_lane_code: 'L1',
    author_lane_title: 'Diagrams',
    reported_at: minutesAgo(17),
    ...over,
  };
}

/** Two runs (one planning, one active) and two floating reviews from different lanes, so a
 *  screen that reads only the first fails. */
export const sprintNow = (): RelayNow =>
  now({
    runs: [
      sprint({
        run_id: OTHER_RUN_ID,
        title: 'Sprint 15: Queue screen',
        goal: 'Run a sprint from the Relay screen',
        state: 'planning',
        created_at: minutesAgo(5),
        started_at: null,
        total_tasks: 1,
        ready_tasks: 1,
        working_tasks: 0,
        in_review_tasks: 0,
        floating_reviews: 0,
        done_tasks: 0,
      }),
      sprint(),
    ],
    floating_reviews: [
      floatingReview(),
      floatingReview({
        task_id: 't-ddl-deltas',
        title: 'Apply schema deltas',
        author_lane_code: 'L3',
        author_lane_title: null,
        reported_at: minutesAgo(3),
      }),
    ],
    lanes: [
      coordinatorLane({
        reviewing: { id: 't-review-er', title: 'ER sidebar on the shell', phase: 'impl', round: 1, claimed_at: minutesAgo(24) },
        task: null,
      }),
      lane({ suggested_name: 'worker-alpha' }),
    ],
  });

export function crew(over: Partial<RelayCrew> = {}): RelayCrew {
  return {
    id: CREW_ID,
    repo_key: REPO,
    workspace_id: 'w1',
    goal: 'Ship relay slice 1',
    status: 'running',
    started_at: minutesAgo(300),
    heartbeat_at: minutesAgo(1),
    ...over,
  };
}

export function now(over: Partial<RelayNow> = {}): RelayNow {
  return {
    crews: [crew()],
    facets: { repos: [REPO], workspaces: [{ id: 'w1', name: 'Sensei' }] },
    lanes: [coordinatorLane(), lane()],
    runs: [],
    floating_reviews: [],
    decisions: [],
    person_messages: [],
    // Agrees with queue().triage: one Jira sprint-47 task, one manual item (r2 L6).
    triage: [
      { provider: 'jira', sprint: 'sprint 47', count: 1 },
      { provider: 'manual', sprint: null, count: 1 },
    ],
    completed_today: [
      completed(),
      completed({ task_id: 't-er-tests', title: 'ER sidebar on the shell — tests', outcome: 'approved' }),
    ],
    today: { completed: 2, reviewed: 5, first_pass_approved: 1 },
    resources: {
      build_lease: { resource: 'rust-build', lane_code: 'L1', granted_at: minutesAgo(4) },
      disk_free_gb: 46,
      disk_floor_gb: 25,
      swap_free_gb: 1.2,
      swap_floor_gb: 1,
    },
    ...over,
  };
}

/** Five lanes, in the view's order: the coordinator, a working lane, one blocked on the
 *  person's answer, one whose lease expired, and an idle one. */
export const fiveLaneCrew = () =>
  now({
    lanes: [
      coordinatorLane(),
      lane(),
      lane({
        id: LANE.L3,
        crew_id: CREW_ID,
        code: 'L3',
        title: 'Activity logs',
        task: {
          id: 't-logs',
          title: 'Activity logs',
          repo_key: REPO,
          workspace_id: 'w1',
          phase: 'tests',
          state: 'blocked',
          round: 1,
          state_changed_at: minutesAgo(55),
          current_step: null,
          steps_done: 0,
          steps_total: 0,
          needs_answer: true,
        },
      }),
      lane({
        id: LANE.L2,
        code: 'L2',
        title: 'Billing',
        family: 'cowork',
        lease_state: 'expired',
        task: {
          id: 't-billing',
          title: 'Billing build',
          repo_key: REPO,
          workspace_id: 'w1',
          phase: 'tests',
          state: 'interrupted',
          round: 3,
          state_changed_at: minutesAgo(70),
          current_step: null,
          steps_done: 0,
          steps_total: 0,
          needs_answer: false,
        },
      }),
      lane({ id: LANE.L5, code: 'L5', title: 'Docs pages', family: 'freebuff', version: '1.4', task: null }),
    ],
  });

/** The main window's answer (no filter): the workspace crew, and a crew on a repository that
 *  belongs to no workspace. Lane codes differ so each row is addressable. */
export const mainNow = (): RelayNow =>
  now({
    crews: [crew(), crew({ id: LOOSE_CREW_ID, repo_key: LOOSE_REPO, workspace_id: null, goal: 'Try the scratch idea' })],
    facets: { repos: [REPO, LOOSE_REPO], workspaces: [{ id: 'w1', name: 'Sensei' }] },
    lanes: [
      coordinatorLane(),
      lane(),
      // Lane codes are unique per crew only; these differ so each row is addressable.
      coordinatorLane({ id: LANE.L8, crew_id: LOOSE_CREW_ID, code: 'L8', title: 'Scratch planning', task: null }),
      lane({
        id: LANE.L9,
        crew_id: LOOSE_CREW_ID,
        code: 'L9',
        title: 'Scratch',
        task: {
          id: 't-scratch',
          title: 'Prototype the scratch parser',
          repo_key: LOOSE_REPO,
          workspace_id: null,
          phase: 'impl',
          state: 'working',
          round: 1,
          state_changed_at: minutesAgo(12),
          current_step: null,
          steps_done: 0,
          steps_total: 0,
          needs_answer: false,
        },
      }),
    ],
  });

/** No crew yet: nothing registered in this workspace. Not a failure. */
export const noCrew = (): RelayNow =>
  now({ crews: [], lanes: [], decisions: [], triage: [], completed_today: [], today: { completed: 0, reviewed: null, first_pass_approved: null } });

/** A tracker task in an active sprint, no item yet: `+` sends its ticket identity. */
export function triageItem(over: Partial<TrackerTriage> = {}): TrackerTriage {
  return {
    item_id: null,
    repo_key: REPO,
    workspace_id: 'w1',
    kind: 'bug',
    title: 'Sync shows "synced" for a refused repository',
    tags: [],
    sprint: 'sprint 47',
    ticket: { provider: 'jira', key: 'SEN-58', url: 'https://acme.atlassian.net/browse/SEN-58' },
    added_by_you: false,
    ...over,
  };
}

/** A manual triage item: `+` sends its id. */
export function manualItem(over: Partial<ManualTriage> = {}): ManualTriage {
  return {
    item_id: 'item-manual',
    repo_key: null,
    workspace_id: 'w1',
    kind: 'finding',
    title: "The pre-commit hook doesn't run the dōjō tests",
    tags: ['dōjō'],
    sprint: null,
    ticket: null,
    added_by_you: true,
    ...over,
  };
}

export function backlogItem(over: Partial<RelayBacklogItem> = {}): RelayBacklogItem {
  return {
    item_id: 'item-61',
    repo_key: REPO,
    workspace_id: 'w1',
    kind: 'feature',
    title: 'Repositories tab · org repositories read',
    tags: [],
    sprint: 'sprint 47',
    ticket: { provider: 'jira', key: 'SEN-61', url: 'https://acme.atlassian.net/browse/SEN-61' },
    added_by_you: false,
    position: 1,
    ...over,
  };
}

export function queue(over: Partial<RelayQueue> = {}): RelayQueue {
  return {
    triage: [triageItem(), manualItem()],
    backlog: [
      backlogItem(),
      backlogItem({
        item_id: 'item-218',
        kind: 'bug',
        title: 'Sync status for refused repositories',
        sprint: 'milestone 11',
        ticket: { provider: 'github', key: '#218', url: 'https://github.com/sensei-hq/sensei/issues/218' },
        position: 2,
      }),
    ],
    ...over,
  };
}

export function run(over: Partial<RelayRunDetails> = {}): RelayRunDetails {
  return {
    repository: 'sensei-hq/sensei',
    branch: 'lane/L1',
    commits: ['3f2a91c', '8be04d7'],
    model: 'claude-opus-5-5',
    tokens_in: 3_100_000,
    tokens_out: 112_000,
    tokens_cache: null,
    context_tokens: 118_000,
    compactions: 0,
    peak_memory_mb: null,
    subagents: 1,
    working_secs: 8520,
    waiting_secs: 1800,
    files_changed: 6,
    files_outside_owned: 1,
    gates_run: 6,
    reported_turns: 2,
    turns: 4,
    ...over,
  };
}

type ActionBase = Omit<RelayAction, 'kind' | 'props'>;

/** One action row, typed by its kind's props (lanes-data table). */
export function action<K extends RelayActionKind>(
  kind: K,
  props: RelayActionProps[K],
  over: Partial<ActionBase> & { id: string },
): RelayAction {
  return {
    lane_code: 'L0',
    round: null,
    phase: null,
    body_md: null,
    reply_to: null,
    created_at: minutesAgo(240),
    ...over,
    kind,
    props,
  } as RelayAction;
}

const none = {} as Record<string, never>;

/** A task's history, newest first, as `relay.task_history` orders it. Two messages from the
 *  person: `act-msg` was delivered and read, `act-msg-2` only delivered (r2 F5). */
export function history(): RelayAction[] {
  return [
    action(
      'merged',
      { commits: ['3f2a91c', '8be04d7'], into: 'develop', gates: [{ cmd: 'make test-fast', exit: 0, counts: 'passed' }] },
      { id: 'act-merged', body_md: 'Merged the two reviewed commits into develop.', created_at: minutesAgo(10) },
    ),
    action('delivered', none, { id: 'act-delivered-2', lane_code: 'L1', reply_to: 'act-msg-2', created_at: minutesAgo(20) }),
    action(
      'message',
      { to: { kind: 'lane', lane_id: LANE.L1 }, surface: 'desktop' },
      { id: 'act-msg-2', lane_code: null, body_md: 'Merge when the gates are green.', created_at: minutesAgo(21) },
    ),
    action(
      'reviewed',
      { verdict: 'approved', findings: [], probes: 8 },
      {
        id: 'act-review-2',
        round: 2,
        phase: 'impl',
        body_md: 'Both comments from r1 are answered with tests that fail without the fix.',
        created_at: minutesAgo(36),
      },
    ),
    action('read', none, { id: 'act-read', lane_code: 'L1', reply_to: 'act-msg', created_at: minutesAgo(50) }),
    action('delivered', none, { id: 'act-delivered', lane_code: 'L1', reply_to: 'act-msg', created_at: minutesAgo(51) }),
    action(
      'message',
      { to: { kind: 'lane', lane_id: LANE.L1 }, surface: 'desktop' },
      {
        id: 'act-msg',
        lane_code: null,
        body_md: 'Keep the old keyboard shortcut for the toolbar working.',
        created_at: minutesAgo(52),
      },
    ),
    action(
      'reported',
      {
        look_at: 'diagram-header.svelte.ts:42',
        flags: 'One file outside my paths (view-states) needed a one-line export.',
        files: [
          { path: 'app/src/lib/diagrams/DiagramHeader.svelte', added: 8, removed: 3, outside_owned: false },
          { path: 'app/src/lib/stores/view-states.svelte.ts', added: 6, removed: 1, outside_owned: true },
        ],
        gates: [{ cmd: 'bun run check', exit: 0, counts: '0 errors' }],
        tokens_in: 900_000,
        tokens_out: 38_000,
        tokens_cache: null,
      },
      {
        id: 'act-report-2',
        lane_code: 'L1',
        round: 2,
        phase: 'impl',
        body_md: 'A failed read now shows the error notice instead of an empty header.',
        created_at: minutesAgo(66),
      },
    ),
    action(
      'reviewed',
      {
        verdict: 'changes_requested',
        findings: [
          {
            severity: 'high',
            class: 'missing-failure-path',
            text: 'a failed read renders an empty header',
            file: 'diagram-header.svelte.ts',
            line: 42,
          },
        ],
      },
      {
        id: 'act-review-1',
        round: 1,
        phase: 'impl',
        body_md: 'The move is clean, but a failed read is silent.',
        created_at: minutesAgo(96),
      },
    ),
    action(
      'reported',
      { files: [], gates: [] },
      {
        id: 'act-report-1',
        lane_code: 'L1',
        round: 1,
        phase: 'impl',
        body_md: 'The header now sits on ScreenShell.',
        created_at: minutesAgo(126),
      },
    ),
    action(
      'assigned',
      {
        docs: ['coord/briefs/L1-diagrams.md', 'docs/prd/modules/diagrams.md'],
        owned_paths: ['app/src/lib/diagrams/**'],
        acceptance: ['every existing spec passes unedited'],
      },
      { id: 'act-assigned', body_md: 'Move the diagrams header onto ScreenShell; behaviour unchanged.' },
    ),
  ];
}

/** A history with the steps CONTRACT §2 names beyond the turns (r2 F8): a brief update, a lost
 *  lease and its re-offer, and the kinds the screen gives no turn of their own. Newest first. */
export function historyWithSteps(): RelayAction[] {
  return [
    action('reoffered', { to_lane: LANE.L3 }, { id: 'act-reoffered', lane_code: 'L0', created_at: minutesAgo(5) }),
    action('interrupted', { checkpoint: null }, { id: 'act-interrupted', lane_code: 'L1', created_at: minutesAgo(35) }),
    action(
      'compacted',
      { context_tokens_before: 180_000, context_tokens_after: 40_000 },
      { id: 'act-compacted', lane_code: 'L1', created_at: minutesAgo(60) },
    ),
    action('answered', { option: 'db', surface: 'desktop' }, { id: 'act-answered', lane_code: null, reply_to: 'act-asked', created_at: minutesAgo(80) }),
    action(
      'asked',
      {
        context: 'Activity logs is waiting on this.',
        options: [
          { id: 'db', label: "In the daemon's database", detail: null },
          { id: 'file', label: 'In a file per workspace', detail: null },
        ],
        recommended: 'db',
        blocks: ['t-header'],
        asked_of: 'person',
      },
      { id: 'act-asked', lane_code: 'L0', body_md: 'Where does the activity log live?', created_at: minutesAgo(90) },
    ),
    action(
      'brief_updated',
      { docs: ['coord/briefs/L1-diagrams-r2.md'] },
      { id: 'act-brief-2', lane_code: 'L0', body_md: 'Also keep the old keyboard shortcut working.', created_at: minutesAgo(120) },
    ),
    action('started', none, { id: 'act-started', lane_code: 'L1', created_at: minutesAgo(200) }),
    action(
      'assigned',
      { docs: ['coord/briefs/L1-diagrams.md'], owned_paths: ['app/src/lib/diagrams/**'], acceptance: [] },
      { id: 'act-assigned', body_md: 'Move the diagrams header onto ScreenShell; behaviour unchanged.' },
    ),
  ];
}

/** The crew's lanes by id, for codes behind uuids in props. */
export const laneRefs = () => [
  { id: LANE.L0, code: 'L0', title: 'Planning & review', suggested_name: 'atlas-rigel' },
  { id: LANE.L1, code: 'L1', title: 'Diagrams', suggested_name: 'cobalt-lyra' },
  { id: LANE.L3, code: 'L3', title: 'Activity logs', suggested_name: 'coral-spica' },
];

export function taskView(over: Partial<RelayTaskView> = {}): RelayTaskView {
  return {
    task: {
      id: 't-header',
      crew_id: CREW_ID,
      title: 'Diagrams header on ScreenShell',
      repo_key: REPO,
      workspace_id: 'w1',
      phase: 'impl',
      state: 'done',
      round: 2,
      ticket: { provider: 'jira', key: 'SEN-52', url: 'https://acme.atlassian.net/browse/SEN-52' },
      lane: { id: LANE.L1, code: 'L1', title: 'Diagrams', family: 'claude-code', version: '2.1', model: 'claude-opus-5-5', limit_scope: null, limit_resumes_at: null, limit_note: null, suggested_name: 'cobalt-lyra' },
      run_id: null,
      deps_status: 'satisfied',
      unmet_deps: [],
      assigned_at: minutesAgo(240),
      completed_at: minutesAgo(10),
      state_changed_at: minutesAgo(10),
      rounds: [
        { phase: 'tests', count: 1 },
        { phase: 'impl', count: 2 },
      ],
      needs_answer: false,
    },
    run: run(),
    decision: null,
    handoff: {
      body_md: 'Move the diagrams header onto ScreenShell; behaviour unchanged.',
      docs: ['coord/briefs/L1-diagrams.md', 'docs/prd/modules/diagrams.md'],
      at: minutesAgo(240),
    },
    history: history(),
    steps: [],
    lanes: laneRefs(),
    ...over,
  };
}

/** A promise the test resolves by hand, for in-flight and single-flight tests. */
export function deferred<T>() {
  let resolve: (v: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
