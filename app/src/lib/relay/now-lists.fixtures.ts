// The Now view's lists as the r1 mockup draws them (Relay.dc.html `NOW`, `DONE`):
// the bench renders these beside the artboard, so the rows are the mockup's own.
// Row data, the shape the @sensei/state selectors produce.
import type {
  RelayCompletedRowView,
  RelayLaneGroupView,
  RelayLaneRowView,
  RelayNeedsYouView,
  RelayWaitRowView,
} from '@sensei/state';
import { decision, personMessage } from '$lib/test-stubs/relay-fixtures.js';

const lane = (
  code: string,
  role: RelayLaneRowView['role'],
  name: string,
  who: string,
  task: string,
  state: RelayLaneRowView['state'],
  when: string,
  over = false,
  href: string | null = `/relay/c/${code}`,
  progress: RelayLaneRowView['progress'] = null,
): RelayLaneRowView => ({ id: code, code, role, name, who, task, state, when: { text: when, over }, href, progress });

/** The mockup's seven lanes, in its order. */
export const mockupLanes = (): RelayLaneGroupView[] => [
  {
    id: 'crew',
    label: null,
    rows: [
      lane('L0', 'coordinator', 'Planning & review', 'atlas-rigel · Claude Code 2.1', 'Review · Notice polish (impl r1)', { label: 'reviewing', tone: 'accent' }, '24 min'),
      lane('L4', 'contributor', 'ER diagrams', 'iris-deneb · Claude Code 2.1', 'ER sidebar on the shell (impl r1)', { label: 'approved, not merged', tone: 'warning' }, '52 min', true),
      lane('L1', 'contributor', 'Diagrams', 'cobalt-lyra · Claude Code 2.1', 'Structure view on the shell (tests r1)', { label: 'waiting · no reviewer', tone: 'warning' }, '26 min', true),
      lane('L2', 'contributor', 'Billing', 'amber-sirius · Cowork', 'Billing build (tests r3)', { label: 'lease lost', tone: 'danger' }, 'at 10:42'),
      lane('L3', 'contributor', 'Activity logs', 'jade-altair · Muse', 'Activity logs (tests r1)', { label: 'needs your answer', tone: 'warning' }, '55 min'),
      lane('L5', 'contributor', 'Docs pages', 'coral-spica · Freebuff 1.4', 'Libraries page (docs r4)', { label: 'verdict to pick up', tone: 'warning' }, '41 min', true),
      lane('L6', 'contributor', 'Sync hardening', 'slate-mira · Agent · in Cursor', 'Nothing assigned', { label: 'idle', tone: 'neutral' }, '', false, null),
    ],
  },
];

/** The main window: two crews, each under its repository. */
export const mainWindowLanes = (): RelayLaneGroupView[] => [
  { id: 'crew-a', label: 'github.com/sensei-hq/sensei', rows: mockupLanes()[0].rows.slice(0, 2) },
  {
    id: 'crew-b',
    label: 'github.com/jerrythomas/scratch',
    rows: [
      lane('L8', 'coordinator', 'Scratch planning', 'L8 · Codex', 'Nothing assigned', { label: 'idle', tone: 'neutral' }, '', false, null),
      lane('L9', 'contributor', 'Scratch parser', 'L9 · Claude Code 2.1', 'Prototype the scratch parser (impl r1)', { label: 'working', tone: 'accent' }, '12 min'),
    ],
  },
];

/** Step progress (#418) — not in the r1 mockup: what each holder reported, from both
 *  counts and the step down to the step alone; the last row reported nothing. */
export const progressLanes = (): RelayLaneGroupView[] => [
  {
    id: 'crew',
    label: null,
    rows: [
      lane('L1', 'contributor', 'Diagrams', 'cobalt-lyra · Claude Code 2.1', 'Structure view on the shell (tests r1)', { label: 'working', tone: 'accent' }, '38 min', false, '/relay/c/L1', { text: '2 of 5 steps done · Writing the failing specs', done: 2, total: 5 }),
      lane('L3', 'contributor', 'Activity logs', 'jade-altair · Muse', 'Activity logs (impl r2)', { label: 'working', tone: 'accent' }, '12 min', false, '/relay/c/L3', { text: '5 of 5 steps done', done: 5, total: 5 }),
      lane('L5', 'contributor', 'Docs pages', 'coral-spica · Freebuff 1.4', 'Libraries page (docs r4)', { label: 'working', tone: 'accent' }, '4 min', false, '/relay/c/L5', { text: 'Reading the brief', done: 0, total: 0 }),
      lane('L9', 'contributor', 'Scratch parser', 'L9 · Claude Code 2.1', 'Prototype the scratch parser (impl r1)', { label: 'working', tone: 'accent' }, '1 min'),
    ],
  },
];

/** A lane paused on a quota, and one whose objective and task run long (the ellipsis). */
export const edgeLanes = (): RelayLaneGroupView[] => [
  {
    id: 'crew',
    label: null,
    rows: [
      lane('L7', 'contributor', 'Release notes', 'L7 · Claude Code 2.1', 'Draft the release notes (docs r1)', { label: 'paused', tone: 'warning' }, 'until 14:00'),
      lane(
        'L8',
        'contributor',
        'A lane objective long enough to need the ellipsis',
        'a-very-long-codename · Claude Code 2.1',
        'A task title that runs well past the width of the row so that it has to end in an ellipsis (impl r2)',
        { label: 'working', tone: 'accent' },
        '2 h 5 min',
      ),
    ],
  },
];

const done = (id: string, at: string, title: string, lane: string, outcome: string): RelayCompletedRowView => ({
  id,
  at,
  title,
  lane,
  outcome: { label: outcome, tone: 'success' },
  href: `/relay/c/${id}`,
});

/** The mockup's five completed tasks. */
export const mockupCompleted = (): RelayCompletedRowView[] => [
  done('x1', '10:02', 'Diagrams header on ScreenShell', 'Diagrams · cobalt-lyra', 'merged'),
  done('x2', '09:40', 'Notice family', 'Billing · amber-sirius', 'merged'),
  done('x4', '08:16', 'ER sidebar on the shell — tests', 'Diagrams · cobalt-lyra', 'approved'),
  done('x5', '07:55', 'Libraries page', 'Docs pages · coral-spica', 'merged'),
  done('x6', '06:55', 'Pack-rule pull', 'Activity logs · jade-altair', 'merged'),
];

// ─── Needs you and Waiting (the mockup's `WAITS`, its decision, its triage line) ─────

const waitRow = (
  id: string,
  minutes: number | null,
  task: string,
  why: string,
  nudge: string,
  href: string | null = `/relay/c/${id}`,
  said: { canNudge?: boolean; nudgedText?: string | null } = {},
): RelayWaitRowView => ({
  id,
  minutes,
  minutesText: minutes == null ? '' : `${minutes} min`,
  over: minutes != null && minutes >= 15,
  task,
  why,
  href,
  nudge: {
    label: nudge,
    canNudge: said.canNudge ?? true,
    nudgedText: said.nudgedText ?? null,
    message: { crew_id: 'c', to: { kind: 'coordinator' }, task_id: href ? id : null, text: task, nudge: true, reason: 'merge_pending' },
  },
});

/** The mockup's waits, oldest first: three past the threshold — each with Sensei's last
 *  automatic nudge beside its button — then the idle lane. */
export const mockupWaits = (): RelayWaitRowView[] => [
  waitRow('w1', 52, 'ER sidebar on the shell (impl r1)', 'ER diagrams · approved, not merged · waiting on the coordinator', 'Nudge the coordinator', '/relay/c/w1', { nudgedText: 'Sensei nudged at 10:23' }),
  waitRow('w2', 41, 'Libraries page (docs r4)', 'Docs pages · verdict filed, not picked up · waiting on Docs pages (coral-spica)', 'Nudge coral-spica', '/relay/c/w2', { nudgedText: 'Sensei nudged at 10:34' }),
  waitRow('d1', 26, 'Structure view on the shell (tests r1)', 'Diagrams · report has no reviewer · waiting on the coordinator', 'Nudge the coordinator', '/relay/c/d1', { nudgedText: 'Sensei nudged at 10:31' }),
  waitRow('L6', null, 'Nothing assigned', 'Sync hardening · lane idle, nothing assigned · waiting on the coordinator', 'Nudge the coordinator', null),
];

/** The mockup's waits after you nudged the oldest: its button has given way to when. */
export const mockupWaitsNudged = (): RelayWaitRowView[] =>
  mockupWaits().map((r, i) => (i === 0 ? { ...r, nudge: { ...r.nudge, canNudge: false, nudgedText: 'Nudged just now · by you' } } : r));

/** Needs you as the mockup draws it with a decision pending. */
export const mockupNeedsYou = (): RelayNeedsYouView => ({
  count: 3,
  decisions: [decision()],
  stale: { title: '3 tasks have waited 15 min or more — oldest first', rows: mockupWaits().slice(0, 3) },
  triage: { count: 3, from: 'From Jira sprint 47, GitHub milestone 11, and an open item' },
  unassigned: null,
  messages: [],
});

/** The tasks no lane has taken, beside a message from a lane. */
export const needsYouUnassigned = (): RelayNeedsYouView => ({
  count: 2,
  decisions: [],
  stale: null,
  triage: null,
  unassigned: { count: 4, from: 'From GitHub milestone 11 and an open item' },
  messages: [personMessage()],
});
