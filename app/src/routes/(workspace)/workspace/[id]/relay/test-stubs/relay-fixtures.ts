// Relay · test fixtures (test-only; `test-stubs/` is outside coverage).
//
// Example data from the mockup (`coord/mockups/relay-lanes/Relay.dc.html`), shaped
// as the daemon's relay views answer it (`$lib/relay-api`). Two crews, two
// workspaces and two of each row kind, so a screen that reads the wrong one, or
// only the first, fails.
import { vi } from 'vitest';
import type { ApiResult } from '$lib/api.js';
import type {
  RelayAction,
  RelayApi,
  RelayBacklogItem,
  RelayCompletedTask,
  RelayDecision,
  RelayLane,
  RelayNow,
  RelayQueue,
  RelayRunDetails,
  RelayTaskView,
  RelayTriageItem,
} from '$lib/relay-api.js';

export const CREW_ID = '019a5e2c-4b1d-7c00-8d2e-00000000c7f3';
export const OTHER_CREW_ID = '019a5e2c-4b1d-7c00-8d2e-0000000099aa';

export const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();

export const ok = <T>(data: T): ApiResult<T> => ({ ok: true, data });
export const fail = (status: number, message: string): ApiResult<never> => ({
  ok: false,
  error: { status, message },
});

export function lane(over: Partial<RelayLane> = {}): RelayLane {
  return {
    id: 'lane-1',
    code: 'L1',
    title: 'Diagrams',
    role: 'contributor',
    family: 'claude-code',
    version: '2.1',
    model: 'claude-opus-5-5',
    lease_state: 'granted',
    lease_expires_at: null,
    task: {
      id: 't-structure',
      title: 'Structure view on the shell',
      phase: 'tests',
      state: 'working',
      round: 1,
      state_changed_at: minutesAgo(38),
      needs_answer: false,
    },
    ...over,
  };
}

export const coordinatorLane = (over: Partial<RelayLane> = {}) =>
  lane({
    id: 'lane-0',
    code: 'L0',
    title: 'Planning & review',
    role: 'coordinator',
    task: {
      id: 't-review-er',
      title: 'Review · ER sidebar on the shell',
      phase: 'impl',
      state: 'in_review',
      round: 1,
      state_changed_at: minutesAgo(24),
      needs_answer: false,
    },
    ...over,
  });

export function decision(over: Partial<RelayDecision> = {}): RelayDecision {
  return {
    id: 'ask-1',
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

export function completed(over: Partial<RelayCompletedTask> = {}): RelayCompletedTask {
  return {
    task_id: 't-header',
    title: 'Diagrams header on ScreenShell',
    lane_code: 'L1',
    lane_title: 'Diagrams',
    outcome: 'merged',
    completed_at: minutesAgo(60),
    ...over,
  };
}

export function now(over: Partial<RelayNow> = {}): RelayNow {
  return {
    crew: {
      id: CREW_ID,
      workspace_id: 'w1',
      goal: 'Ship relay slice 1',
      status: 'running',
      started_at: minutesAgo(300),
      heartbeat_at: minutesAgo(1),
    },
    lanes: [coordinatorLane(), lane()],
    decisions: [],
    triage: [],
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

/** No crew yet: nothing registered in this workspace. Not a failure. */
export const noCrew = (): RelayNow =>
  now({ crew: null, lanes: [], decisions: [], triage: [], completed_today: [], today: { completed: 0, reviewed: null, first_pass_approved: null } });

export function triageItem(over: Partial<RelayTriageItem> = {}): RelayTriageItem {
  return {
    item_id: null,
    kind: 'bug',
    title: 'Sync shows "synced" for a refused repository',
    tags: [],
    sprint: 'sprint 47',
    ticket: { provider: 'jira', key: 'SEN-58', url: 'https://acme.atlassian.net/browse/SEN-58' },
    added_by_you: false,
    ...over,
  };
}

export function backlogItem(over: Partial<RelayBacklogItem> = {}): RelayBacklogItem {
  return {
    item_id: 'item-61',
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
    triage: [
      triageItem(),
      triageItem({
        item_id: 'item-manual',
        kind: 'finding',
        title: "The pre-commit hook doesn't run the dōjō tests",
        tags: ['dōjō'],
        sprint: null,
        ticket: null,
        added_by_you: true,
      }),
    ],
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

export function action(over: Partial<RelayAction> = {}): RelayAction {
  return {
    id: 'act-1',
    kind: 'assigned',
    lane_code: 'L0',
    round: null,
    phase: null,
    body_md: null,
    props: {},
    reply_to: null,
    created_at: minutesAgo(240),
    ...over,
  };
}

/** A task's history, newest first, as `relay.task_history` orders it. */
export function history(): RelayAction[] {
  return [
    action({
      id: 'act-merged',
      kind: 'merged',
      lane_code: 'L0',
      body_md: 'Merged the two reviewed commits into develop.',
      props: { commits: ['3f2a91c', '8be04d7'], into: 'develop', gates: [{ cmd: 'make test-fast', exit: 0, counts: 'passed' }] },
      created_at: minutesAgo(10),
    }),
    action({
      id: 'act-review-2',
      kind: 'reviewed',
      lane_code: 'L0',
      round: 2,
      phase: 'impl',
      body_md: 'Both comments from r1 are answered with tests that fail without the fix.',
      props: { verdict: 'approved', findings: [], probes: 8 },
      created_at: minutesAgo(36),
    }),
    action({
      id: 'act-read',
      kind: 'read',
      lane_code: 'L1',
      reply_to: 'act-msg',
      created_at: minutesAgo(50),
    }),
    action({
      id: 'act-delivered',
      kind: 'delivered',
      lane_code: 'L1',
      reply_to: 'act-msg',
      created_at: minutesAgo(51),
    }),
    action({
      id: 'act-msg',
      kind: 'message',
      lane_code: null,
      body_md: 'Keep the old keyboard shortcut for the toolbar working.',
      props: { to: 'L1', surface: 'desktop' },
      created_at: minutesAgo(52),
    }),
    action({
      id: 'act-report-2',
      kind: 'reported',
      lane_code: 'L1',
      round: 2,
      phase: 'impl',
      body_md: 'A failed read now shows the error notice instead of an empty header.',
      props: {
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
      created_at: minutesAgo(66),
    }),
    action({
      id: 'act-review-1',
      kind: 'reviewed',
      lane_code: 'L0',
      round: 1,
      phase: 'impl',
      body_md: 'The move is clean, but a failed read is silent.',
      props: {
        verdict: 'changes_requested',
        findings: [
          { severity: 'high', class: 'missing-failure-path', text: 'a failed read renders an empty header', file: 'diagram-header.svelte.ts', line: 42 },
        ],
      },
      created_at: minutesAgo(96),
    }),
    action({
      id: 'act-report-1',
      kind: 'reported',
      lane_code: 'L1',
      round: 1,
      phase: 'impl',
      body_md: 'The header now sits on ScreenShell.',
      props: { files: [], gates: [] },
      created_at: minutesAgo(126),
    }),
    action({
      id: 'act-assigned',
      kind: 'assigned',
      lane_code: 'L0',
      body_md: 'Move the diagrams header onto ScreenShell; behaviour unchanged.',
      props: { docs: ['coord/briefs/L1-diagrams.md', 'docs/prd/modules/diagrams.md'] },
      created_at: minutesAgo(240),
    }),
  ];
}

export function taskView(over: Partial<RelayTaskView> = {}): RelayTaskView {
  return {
    task: {
      id: 't-header',
      title: 'Diagrams header on ScreenShell',
      phase: 'impl',
      state: 'done',
      round: 2,
      ticket: { provider: 'jira', key: 'SEN-52', url: 'https://acme.atlassian.net/browse/SEN-52' },
      lane: { id: 'lane-1', code: 'L1', title: 'Diagrams', family: 'claude-code', version: '2.1', model: 'claude-opus-5-5' },
      assigned_at: minutesAgo(240),
      completed_at: minutesAgo(10),
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
    ...over,
  };
}

/** A relay client whose every call is a spy; each test sets the answers it needs. */
export type FakeRelayApi = { [K in keyof RelayApi]: ReturnType<typeof vi.fn> & RelayApi[K] };

export function fakeRelayApi(): FakeRelayApi {
  return {
    tryGetNow: vi.fn(async () => ok(now())),
    tryGetQueue: vi.fn(async () => ok(queue())),
    tryGetTask: vi.fn(async () => ok(taskView())),
    trySendMessage: vi.fn(async () => ok({ message_id: 'msg-1' })),
    tryAnswerDecision: vi.fn(async () => ok({ action_id: 'act-answer' })),
    tryAddToBacklog: vi.fn(async () => ok({ item_id: 'item-new' })),
  } as FakeRelayApi;
}

/** A promise the test resolves by hand, for in-flight and single-flight tests. */
export function deferred<T>() {
  let resolve: (v: T) => void = () => {};
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
