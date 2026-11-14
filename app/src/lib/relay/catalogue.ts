// The relay component catalogue: every component with the fixture set it is
// verified on (COMPONENTS.md §0 — one object per state a component can be in).
// Two consumers, one list: the development preview (/preview) renders it for
// review against the mockup and for the capture matrix (/preview/all); a component's spec
// snapshots the same fixtures per theme × layout. Props are plain data — a
// component here has no reads of its own — so an entry is a component and the
// props it takes, nothing else.

import { createRawSnippet } from 'svelte';
import { Pill } from '@rokkit/ui';
import type { PreviewEntry } from '$lib/preview/catalogue.js';
import { decision, floatingReview, history, laneRefs, personMessage, sprint } from '$lib/test-stubs/relay-fixtures.js';
import { toRelayShell } from './relay-shell.js';
import { SHELL_FIXTURES } from './relay-shell.fixtures.js';
import RelayShell from './RelayShell.svelte';
import DecisionRow from './DecisionRow.svelte';
import FloatingReviewRow from './FloatingReviewRow.svelte';
import MessageRow from './MessageRow.svelte';
import QueueRow from './QueueRow.svelte';
import QueueSection from './QueueSection.svelte';
import AddItemForm from './AddItemForm.svelte';
import RunCard from './RunCard.svelte';
import TaskHeader from './TaskHeader.svelte';
import TaskHistory from './TaskHistory.svelte';
import RunDetails from './RunDetails.svelte';
import HandoffCard from './HandoffCard.svelte';
import { mockupHandoff, mockupRunDetails, mockupTaskHeader } from './task.fixtures.js';
import AddMember from './AddMember.svelte';
import { addContributor, mockupGroups, mockupRunCard, setupStep1, setupStep2 } from './crew.fixtures.js';
import { mockupBacklog, mockupTriage } from './queue.fixtures.js';
import RelayChecklist from './RelayChecklist.svelte';
import RoleMark from './RoleMark.svelte';
import SprintBand from './SprintBand.svelte';
import LaneList from './LaneList.svelte';
import CompletedList from './CompletedList.svelte';
import NeedsYou from './NeedsYou.svelte';
import WaitList from './WaitList.svelte';
import {
  edgeLanes,
  mainWindowLanes,
  mockupCompleted,
  mockupLanes,
  mockupNeedsYou,
  mockupWaits,
  mockupWaitsNudged,
  needsYouUnassigned,
  progressLanes,
} from './now-lists.fixtures.js';


const entry = (
  id: string,
  note: string,
  component: PreviewEntry['component'],
  props: Record<string, unknown>,
  frame?: PreviewEntry['frame'],
): PreviewEntry => ({ id, note, component, props, frame });

/** The preview never answers a decision or adds to a backlog: a fixture's
 *  callbacks resolve to "no refusal" so the component can be exercised. */
const accepted = async () => null;
/** A shell fixture's actions go nowhere: the bench shows one state at a time. */
const still = { view: () => {}, compose: () => {}, filter: () => {}, narrow: () => {} };
/** A stand-in body, so the shell's own regions are what the picture shows. */
/** Rows enough that every shell's body overflows at both widths: the scroll test
 *  (e2e/preview/scroll.spec.ts) scrolls each one to its last row. Numbered, so a
 *  person reviewing sees where the scroll ends. */
const SCROLL_ROWS = 60;
const body = createRawSnippet(() => ({
  render: () =>
    `<ol class="m-0 list-none p-0" data-scroll-body>${Array.from(
      { length: SCROLL_ROWS },
      (_, i) =>
        `<li data-scroll-row class="border-b border-paper-mute px-5 py-2 text-sm text-ink-soft">row ${i + 1} of ${SCROLL_ROWS}</li>`,
    ).join('')}</ol>`,
}));
const band = createRawSnippet(() => ({
  render: () =>
    '<div class="border-b border-paper-edge bg-paper-soft px-5 py-3 text-sm text-ink-mute">Message the crew · the compose band (its own fixture)</div>',
}));

export const RELAY_CATALOGUE: PreviewEntry[] = [
  entry('LaneList/mockup', 'the mockup’s seven lanes: working, waits past the threshold, lease lost, a question, idle', LaneList, { groups: mockupLanes() }),
  entry('LaneList/main-window', 'the main window: a card per crew under its repository', LaneList, { groups: mainWindowLanes() }),
  entry('LaneList/edges', 'a quota pause, and an objective and task that need the ellipsis', LaneList, { groups: edgeLanes() }),
  entry('LaneList/progress', 'NOT IN THE MOCKUP (#418): step progress under the task — counts and step, counts only, step only, nothing reported', LaneList, { groups: progressLanes() }),
  entry('NeedsYou/mockup', 'a decision, three long waits, the triage line', NeedsYou, {
    view: mockupNeedsYou(),
    href: (crew: string, task: string) => `/relay/${crew}/${task}`,
    onanswer: accepted,
    onopenqueue: () => {},
    onnudge: () => {},
  }),
  entry('NeedsYou/unassigned', 'tasks no lane has taken (read-only), and a lane’s message', NeedsYou, {
    view: needsYouUnassigned(),
    href: (crew: string, task: string) => `/relay/${crew}/${task}`,
    onanswer: accepted,
    onopenqueue: () => {},
    onnudge: () => {},
  }),
  entry('WaitList/waiting', 'Waiting’s filter: every wait, oldest first, the idle lane with no time; Nudge on each, Sensei’s last nudge beside the long ones', WaitList, { rows: mockupWaits(), onnudge: () => {} }),
  entry('WaitList/nudged-by-you', 'after you nudge the oldest: its button gives way to “Nudged just now · by you”', WaitList, { rows: mockupWaitsNudged(), onnudge: () => {} }),
  entry('CompletedList/mockup', 'the mockup’s five completed tasks, merged and approved', CompletedList, { rows: mockupCompleted() }),
  ...SHELL_FIXTURES.map((f) =>
    entry(`RelayShell/${f.name}`, f.note, RelayShell, { shell: toRelayShell(f.input(), still), viewId: `preview.relay.${f.name}`, children: body, compose: band }, { height: 560, phoneHeight: 900 }),
  ),

  entry('Pill/neutral', 'rokkit Pill, small: a state, neutral tone', Pill, { value: 'idle', tone: 'neutral', size: 'sm' }),
  entry('Pill/accent', 'working', Pill, { value: 'reviewing', tone: 'accent', size: 'sm' }),
  entry('Pill/warning', 'a wait past the threshold', Pill, { value: 'approved, not merged', tone: 'warning', size: 'sm' }),
  entry('Pill/danger', 'a lost lease', Pill, { value: 'lease lost', tone: 'danger', size: 'sm' }),
  entry('Pill/success', 'merged', Pill, { value: 'merged', tone: 'success', size: 'sm' }),
  entry('Pill/family', 'the assistant family: the plain chip', Pill, { value: 'Claude Code 2.1', size: 'sm' }),
  entry('Pill/model', 'the model: the outline chip, in mono', Pill, { value: 'claude-opus-5-5', variant: 'outline', size: 'sm', class: 'font-mono' }),

  entry('RoleMark/coordinator-24', 'lane row size, coordinator', RoleMark, { role: 'coordinator', size: 24 }),
  entry('RoleMark/contributor-24', 'lane row size, contributor', RoleMark, { role: 'contributor', size: 24 }),
  entry('RoleMark/coordinator-30-active', 'member card size, active', RoleMark, { role: 'coordinator', status: 'active', size: 30 }),
  entry('RoleMark/contributor-30-idle', 'member card size, idle', RoleMark, { role: 'contributor', status: 'idle', size: 30 }),
  entry('RoleMark/contributor-30-not_responding', 'member card size, not responding', RoleMark, {
    role: 'contributor',
    status: 'not_responding',
    size: 30,
  }),

  entry('QueueRow/triage-ticket', 'a triage item from a sprint, with its ticket and the add button', QueueRow, { row: mockupTriage()[0], onadd: accepted }),
  entry('QueueRow/triage-manual', 'an item the person added, no ticket', QueueRow, { row: mockupTriage()[2], onadd: accepted }),
  entry('QueueRow/backlog', 'a backlog row: no add button, selectable for a run', QueueRow, { row: mockupBacklog()[1], selected: false, ontoggle: () => {} }),
  entry('QueueSection/triage', 'Triage: the mockup’s three rows', QueueSection, {
    id: 'triage',
    label: 'Triage',
    kanji: '選',
    description: 'From your active sprints in Jira, Azure DevOps and GitHub, and open items you add.',
    rows: mockupTriage(),
    onadd: accepted,
  }),
  entry('AddItemForm/empty', 'add an open item to triage — not in the r1 mockup: the smallest form the contract needs, for Design', AddItemForm, { onsubmit: accepted }),
  entry('RunCard/running', 'the mockup’s run: the coordinator, six contributors — one not responding, one idle', RunCard, {
    card: mockupRunCard(),
    groups: mockupGroups(),
    onmessage: () => {},
  }),
  entry('AddMember/setup', 'no crew yet: step 1 of 2, the coordinator', AddMember, { view: setupStep1(), oncopy: () => {}, onrole: () => {} }),
  entry('AddMember/setup-step2', 'the coordinator in: step 2 of 2, a contributor; the line names the crew (#419, beyond the mockup line)', AddMember, { view: setupStep2(), oncopy: () => {}, onrole: () => {} }),
  entry('AddMember/add', 'a crew at work: Add a member, contributors only', AddMember, { view: addContributor(), oncopy: () => {}, onrole: () => {} }),
  entry('TaskHeader/merged', 'the task header: back, title and state, the ticket pill, the facts', TaskHeader, { header: mockupTaskHeader() }),
  entry('RunDetails/folded', 'Run details, folded to its line (open it: a value nobody reported reads so)', RunDetails, { run: mockupRunDetails() }),
  entry('HandoffCard/brief', 'Handed off: when, the brief, its docs', HandoffCard, { handoff: mockupHandoff() }),
  entry('TaskHistory/merged', 'a merged task’s history, newest first: merge, messages, reviews, turns', TaskHistory, { history: history(), lanes: laneRefs() }),
  entry('QueueSection/backlog', 'Backlog: the mockup’s four rows, in order', QueueSection, {
    id: 'backlog',
    label: 'Backlog',
    kanji: '列',
    description: 'Your tasks, in order. The crew’s coordinator takes the top one next and puts it on a lane.',
    rows: mockupBacklog(),
    ontoggle: () => {},
  }),

  entry('DecisionRow/two-options', 'a decision with the recommended option marked', DecisionRow, {
    decision: decision(),
    href: '/relay/crew/task',
    onanswer: accepted,
  }),

  entry('FloatingReviewRow/floating', 'a report waiting for a reviewer', FloatingReviewRow, {
    review: floatingReview(),
    href: '/relay/crew/task',
  }),

  entry('SprintBand/active', 'a run in progress', SprintBand, { run: sprint() }),

  entry('MessageRow/unread', 'a message from a lane to the person', MessageRow, {
    message: personMessage(),
    href: '/relay/crew/task',
  }),

  entry('RelayChecklist/mixed', 'a task checklist with done, open and skipped steps', RelayChecklist, {
    steps: [
      { key: 'tests', title: 'Red tests pin the behaviour', state: 'done', reason: null, updated_at: '2026-11-12T10:02:00Z' },
      { key: 'impl', title: 'Implementation', state: 'open', reason: null, updated_at: '2026-11-12T10:20:00Z' },
      { key: 'docs', title: 'Docs', state: 'skipped', reason: 'no user-facing change', updated_at: '2026-11-12T10:21:00Z' },
    ],
  }),
];
