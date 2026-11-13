// The relay component catalogue: every component with the fixture set it is
// verified on (COMPONENTS.md §0 — one object per state a component can be in).
// Two consumers, one list: the development gallery (/gallery) renders it for
// review against the mockup and for the capture matrix; a component's spec
// snapshots the same fixtures per theme × layout. Props are plain data — a
// component here has no reads of its own — so an entry is a component and the
// props it takes, nothing else.

import { createRawSnippet, type Component } from 'svelte';
import { decision, floatingReview, personMessage, sprint } from '$lib/test-stubs/relay-fixtures.js';
import { toRelayShell } from './relay-shell.js';
import { SHELL_FIXTURES } from './relay-shell.fixtures.js';
import RelayShell from './RelayShell.svelte';
import DecisionRow from './DecisionRow.svelte';
import FloatingReviewRow from './FloatingReviewRow.svelte';
import MessageRow from './MessageRow.svelte';
import Pill from './Pill.svelte';
import QueueRow from './QueueRow.svelte';
import RelayChecklist from './RelayChecklist.svelte';
import RoleMark from './RoleMark.svelte';
import SprintBand from './SprintBand.svelte';
import Tag from './Tag.svelte';

export interface GalleryEntry {
  /** `Component/fixture`, stable: the capture's folder and the mockup map's key. */
  id: string;
  /** One line on what this fixture shows, for the person reviewing. */
  note: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  component: Component<any>;
  props: Record<string, unknown>;
  /** A fixed sample height, for a component that fills its container (the shell). */
  frame?: { height: number };
}

const entry = (
  id: string,
  note: string,
  component: GalleryEntry['component'],
  props: Record<string, unknown>,
  frame?: GalleryEntry['frame'],
): GalleryEntry => ({ id, note, component, props, frame });

/** The gallery never answers a decision or adds to a backlog: a fixture's
 *  callbacks resolve to "no refusal" so the component can be exercised. */
const accepted = async () => null;
/** A shell fixture's actions go nowhere: the bench shows one state at a time. */
const still = { view: () => {}, compose: () => {}, filter: () => {} };
/** A stand-in body, so the shell's own regions are what the picture shows. */
const body = createRawSnippet(() => ({
  render: () => '<div class="p-5 text-sm text-ink-mute">the view’s body goes here</div>',
}));
const band = createRawSnippet(() => ({
  render: () =>
    '<div class="border-b border-paper-edge bg-paper-soft px-5 py-3 text-sm text-ink-mute">Message the crew · the compose band (its own fixture)</div>',
}));

export const GALLERY: GalleryEntry[] = [
  ...SHELL_FIXTURES.map((f) =>
    entry(`RelayShell/${f.name}`, f.note, RelayShell, { shell: toRelayShell(f.input(), still), viewId: `gallery.relay.${f.name}`, children: body, compose: band }, { height: 560 }),
  ),

  entry('Pill/neutral', 'a state pill, neutral tone', Pill, { label: 'idle', tone: 'neutral' }),
  entry('Pill/accent', 'working', Pill, { label: 'reviewing', tone: 'accent' }),
  entry('Pill/warning', 'a wait past the threshold', Pill, { label: 'approved, not merged', tone: 'warning' }),
  entry('Pill/danger', 'a lost lease', Pill, { label: 'lease lost', tone: 'danger' }),
  entry('Pill/success', 'merged', Pill, { label: 'merged', tone: 'success' }),

  entry('Tag/family', 'the assistant family', Tag, { kind: 'family', label: 'Claude Code 2.1' }),
  entry('Tag/model', 'the model, mono', Tag, { kind: 'model', label: 'claude-opus-5-5' }),
  entry('Tag/host', 'an assistant inside an editor', Tag, { kind: 'host', editor: 'Cursor' }),

  entry('RoleMark/coordinator-24', 'lane row size, coordinator', RoleMark, { role: 'coordinator', size: 24 }),
  entry('RoleMark/contributor-24', 'lane row size, contributor', RoleMark, { role: 'contributor', size: 24 }),
  entry('RoleMark/coordinator-30-active', 'member card size, active', RoleMark, { role: 'coordinator', status: 'active', size: 30 }),
  entry('RoleMark/contributor-30-idle', 'member card size, idle', RoleMark, { role: 'contributor', status: 'idle', size: 30 }),
  entry('RoleMark/contributor-30-not_responding', 'member card size, not responding', RoleMark, {
    role: 'contributor',
    status: 'not_responding',
    size: 30,
  }),

  entry('QueueRow/triage-ticket', 'a triage item from a sprint, with its ticket and the add button', QueueRow, {
    id: 'i1',
    title: 'Sync shows "synced" for a refused repository',
    kind: 'bug',
    tags: ['sprint 47', 'sensei'],
    sprint: '47',
    ticket: { provider: 'jira', key: 'SEN-58', url: 'https://example.atlassian.net/browse/SEN-58' },
    onadd: accepted,
  }),
  entry('QueueRow/triage-manual', 'an item the person added, no ticket', QueueRow, {
    id: 'i3',
    title: 'The pre-commit hook does not run the dōjō tests',
    kind: 'finding',
    tags: ['dōjō'],
    addedByYou: true,
    onadd: accepted,
  }),
  entry('QueueRow/backlog', 'a backlog row: no add button, selectable for a run', QueueRow, {
    id: 'b1',
    title: 'Repositories tab · org repositories read',
    kind: 'feature',
    tags: ['sprint 47', 'dōjō', 'D46'],
    ticket: { provider: 'jira', key: 'SEN-61', url: 'https://example.atlassian.net/browse/SEN-61' },
    selectable: true,
    selected: false,
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
