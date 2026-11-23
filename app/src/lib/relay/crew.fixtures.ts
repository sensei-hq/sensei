// The Crew view as the r1 mockup draws it (Relay.dc.html, view "crew"): the bench renders
// these beside the artboard. Data in the shape the crew selectors produce.
import { editorTag, toAddMember, type RelayAddMemberView, type RelayCatalogue, type RelayMemberGroupView, type RelayMemberView, type RelayRunCardView } from '@sensei/state';

const m = (
  laneCode: string,
  codename: string,
  lane: string,
  role: RelayMemberView['role'],
  status: RelayMemberView['status'],
  family: string,
  model: string,
  alert: string | null = null,
  editor: RelayMemberView['editor'] = null,
): RelayMemberView => ({
  id: laneCode,
  codename,
  lane: `${lane} · ${laneCode}`,
  laneCode,
  role,
  status,
  statusText: { active: 'Active', idle: 'Idle', not_responding: 'Not responding' }[status],
  family,
  model,
  alert,
  editor,
});

export const mockupRunCard = (): RelayRunCardView => ({
  title: 'Run 7f3a',
  state: { label: 'running', tone: 'success' },
  meta: 'sensei · started 06:00 · coordinator atlas-rigel · completes, and is archived, when its last assigned task is done',
  facts: [
    { label: 'Members', value: 7 },
    { label: 'Tasks', value: 12 },
    { label: 'Done', value: 5 },
  ],
  draft: null,
});

/** The mockup's crew: the coordinator, six contributors — one not responding, one idle. */
export const mockupGroups = (): RelayMemberGroupView[] => [
  { label: 'Coordinator', members: [m('L0', 'atlas-rigel', 'Planning & review', 'coordinator', 'active', 'Claude Code 2.1', 'claude-opus-5-5')] },
  {
    label: 'Contributors · 6',
    members: [
      m('L1', 'cobalt-lyra', 'Diagrams', 'contributor', 'active', 'Claude Code 2.1', 'claude-opus-5-5'),
      m('L2', 'amber-sirius', 'Billing', 'contributor', 'not_responding', 'Cowork', 'claude-opus-5-5', 'No answer since 10:42 · lease lost.'),
      m('L3', 'jade-altair', 'Activity logs', 'contributor', 'active', 'Muse', 'model not reported'),
      m('L4', 'iris-deneb', 'ER diagrams', 'contributor', 'active', 'Claude Code 2.1', 'claude-opus-5-5'),
      m('L5', 'coral-spica', 'Docs pages', 'contributor', 'active', 'Freebuff 1.4', 'model not reported'),
      m('L6', 'slate-mira', 'Sync hardening', 'contributor', 'idle', 'Agent', 'claude-sonnet-4-5', null, editorTag('Cursor', 'claude-sonnet-4-5')),
    ],
  },
];

/** No crew yet: step 1 of 2, the coordinator. */
export const setupStep1 = (): RelayAddMemberView => ({
  mode: 'setup',
  title: 'Start a run: set up your crew',
  lead: 'Relay runs one crew at a time. Two sessions are enough: one coordinator, then one or more contributors. Each registers with a single line, gets a codename, and the run starts with the second.',
  role: 'coordinator',
  roleLocked: false,
  command: {
    step: 'Step 1 of 2.',
    text: 'register with sensei as coordinator',
    note: "Paste it into an assistant session open in this workspace's repository. It registers, gets a codename, and shows above.",
  },
  listening: 'Listening · 0 of 2 to start a run',
  catalogue: null,
  catalogueError: null,
  launch: null,
});

/** The crew's id on the lines that name it (#419). */
const CREW = '019a5e2c-4b1d-7c00-8d2e-00000000c7f3';

/** The coordinator registered: step 2 of 2, a contributor into this crew (#419: the line names it). */
export const setupStep2 = (): RelayAddMemberView => ({
  ...setupStep1(),
  role: 'contributor',
  roleLocked: true,
  command: { ...setupStep1().command, step: 'Step 2 of 2.', text: `register with sensei as contributor in crew ${CREW}` },
  listening: 'Listening · 1 of 2 to start a run',
});

/** A crew at work: Add a member, contributors only. */
export const addContributor = (): RelayAddMemberView => ({
  ...setupStep1(),
  mode: 'add',
  title: 'Add a member',
  lead: 'Another session joins as a contributor and takes the next piece of work.',
  role: 'contributor',
  roleLocked: true,
  command: { ...setupStep1().command, step: '', text: `register with sensei as contributor in crew ${CREW}` },
  listening: 'Listening · a new member shows above as it registers',
});

const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();

/** What this machine has seen (#455), as the daemon serves it: two assistants, two editors,
 *  and a word the catalogue does not list (never offered). */
export const mockupCatalogue = (): RelayCatalogue => ({
  assistants: [
    { id: 'claude', name: 'Claude Code', kind: 'assistant', installed: true, last_used_at: minutesAgo(120), sessions_week: 12, launch: 'cd "/w/sensei" && claude' },
    { id: 'codex', name: 'Codex', kind: 'assistant', installed: null, last_used_at: minutesAgo(60 * 24 * 9), sessions_week: 0, launch: null },
  ],
  editors: [
    { id: 'cursor', name: 'Cursor', kind: 'editor', installed: true, last_used_at: minutesAgo(5), models: ['claude-sonnet-4-5'], mcp_config: '~/.cursor/mcp.json', launch: null },
    { id: 'zed', name: 'Zed', kind: 'editor', installed: true, last_used_at: null, models: [], mcp_config: null, launch: null },
  ],
  unknown: [{ id: 'cowork', name: 'Cowork', kind: 'unknown', last_used_at: minutesAgo(30) }],
});

/** Add a member with the catalogue picker, nothing picked. */
export const addWithCatalogue = (picked: string | null = null): RelayAddMemberView =>
  toAddMember(null, 'coordinator', { data: mockupCatalogue(), error: null, picked });

/** The catalogue could not be read: the error Notice, the line still there. */
export const addCatalogueError = (): RelayAddMemberView =>
  toAddMember(null, 'coordinator', { data: null, error: 'The daemon answered 500: internal error', picked: null });
