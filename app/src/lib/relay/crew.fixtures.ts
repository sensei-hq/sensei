// The Crew view as the r1 mockup draws it (Relay.dc.html, view "crew"): the bench renders
// these beside the artboard. Data in the shape the crew selectors produce.
import type { RelayAddMemberView, RelayMemberGroupView, RelayMemberView, RelayRunCardView } from '@sensei/state';

const m = (
  laneCode: string,
  codename: string,
  lane: string,
  role: RelayMemberView['role'],
  status: RelayMemberView['status'],
  family: string,
  model: string,
  alert: string | null = null,
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
      m('L6', 'slate-mira', 'Sync hardening', 'contributor', 'idle', 'Agent', 'claude-sonnet-4-5'),
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
