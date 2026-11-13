// The Crew view's selectors (#424; CONTRACT §1h; COMPONENTS §3e): the run card, the
// members grouped by role with their presence, and AddMember's setup or add. Pure. What
// the daemon does not report — a heartbeat, the editor a session runs in, the catalogue
// of assistants seen on this machine — is not invented (D83).

import { crewTail, hhmm } from './format.js';
import { assistantLabel, type RelayTone } from './now-selectors.js';
import type { RelayLane, RelayNow, RelayPresence, RelayRole } from './types.js';

export type { RelayPresence };

/** The run, or the crew before its first run (COMPONENTS §3e RunCard). */
export interface RelayRunCardView {
  title: string;
  state: { label: string; tone: RelayTone };
  meta: string;
  facts: Array<{ label: string; value: number }>;
}

/** One member (COMPONENTS §3e MemberCard). */
export interface RelayMemberView {
  id: string;
  codename: string;
  /** "objective · code". */
  lane: string;
  laneCode: string;
  role: RelayRole;
  status: RelayPresence;
  statusText: string;
  family: string;
  model: string;
  /** The not-responding line under the card. */
  alert: string | null;
}

export interface RelayMemberGroupView {
  label: string;
  members: RelayMemberView[];
}

/** AddMember (COMPONENTS §3e): setting the crew up, or adding to it. */
export interface RelayAddMemberView {
  mode: 'setup' | 'add';
  title: string;
  lead: string;
  role: RelayRole;
  /** The coordinator is taken: contributors only. */
  roleLocked: boolean;
  command: { step: string; text: string; note: string };
  listening: string;
}

const codename = (lane: RelayLane) => lane.suggested_name ?? lane.code;
const repoName = (repo: string | null) => (repo ? (repo.split('/').pop() ?? repo) : 'no repository');

/** The run card: the open run when there is one, else the crew itself; null without a crew. */
export function toRunCard(now: RelayNow | null): RelayRunCardView | null {
  const crew = now?.crews[0];
  if (!now || !crew) return null;
  const coordinator = now.lanes.find((l) => l.crew_id === crew.id && l.role === 'coordinator');
  const members = now.lanes.filter((l) => l.crew_id === crew.id).length;
  const who = coordinator ? ` · coordinator ${codename(coordinator)}` : '';
  const run = now.runs.find((r) => r.crew_id === crew.id && (r.state === 'planning' || r.state === 'active'));
  if (!run) {
    return {
      title: `Crew ${crewTail(crew.id)}`,
      state: { label: 'no run yet', tone: 'neutral' },
      meta: `${repoName(crew.repo_key)} · started ${hhmm(crew.started_at)}${who}`,
      facts: [{ label: 'Members', value: members }],
    };
  }
  const started = run.started_at ?? run.created_at;
  return {
    title: `Run ${run.run_id.slice(-4)}`,
    state: run.state === 'active' ? { label: 'running', tone: 'success' } : { label: 'planning', tone: 'accent' },
    meta: `${repoName(crew.repo_key)} · started ${hhmm(started)}${who} · completes, and is archived, when its last assigned task is done`,
    facts: [
      { label: 'Members', value: members },
      { label: 'Tasks', value: run.total_tasks },
      { label: 'Done', value: run.done_tasks },
    ],
  };
}

const STATUS_TEXT: Record<RelayPresence, string> = { active: 'Active', idle: 'Idle', not_responding: 'Not responding' };

function member(lane: RelayLane): RelayMemberView {
  const status = lane.presence;
  return {
    id: lane.id,
    codename: codename(lane),
    lane: `${lane.title ?? lane.code} · ${lane.code}`,
    laneCode: lane.code,
    role: lane.role,
    status,
    statusText: STATUS_TEXT[status],
    family: assistantLabel(lane.family, lane.version),
    model: lane.model ?? 'model not reported',
    alert:
      status === 'not_responding'
        ? `No answer since ${lane.lease_expires_at ? hhmm(lane.lease_expires_at) : 'a time not reported'} · lease lost.`
        : null,
  };
}

/** The coordinator, then the contributors with their count. */
export function toMemberGroups(now: RelayNow | null, _at: number): RelayMemberGroupView[] {
  if (!now) return [];
  const coordinators = now.lanes.filter((l) => l.role === 'coordinator').map(member);
  const contributors = now.lanes.filter((l) => l.role === 'contributor').map(member);
  const groups: RelayMemberGroupView[] = [];
  if (coordinators.length) groups.push({ label: 'Coordinator', members: coordinators });
  if (contributors.length) groups.push({ label: `Contributors · ${contributors.length}`, members: contributors });
  return groups;
}

const SETUP_LEAD =
  'Relay runs one crew at a time. Two sessions are enough: one coordinator, then one or more contributors. Each registers with a single line, gets a codename, and the run starts with the second.';

/** AddMember: set the crew up (no run and fewer than two members), or add to it. */
export function toAddMember(now: RelayNow | null, picked: RelayRole): RelayAddMemberView {
  const lanes = now?.lanes ?? [];
  const hasCoordinator = lanes.some((l) => l.role === 'coordinator');
  const hasRun = (now?.runs ?? []).some((r) => r.state === 'planning' || r.state === 'active');
  const setup = !hasRun && lanes.length < 2;
  const role: RelayRole = hasCoordinator ? 'contributor' : picked;
  const line = `register with sensei as ${role}`;
  const command = {
    step: setup ? (role === 'coordinator' ? 'Step 1 of 2.' : 'Step 2 of 2.') : '',
    text: line,
    note: `Paste it into an assistant session open in this workspace's repository. It registers, gets a codename, and shows above.`,
  };
  if (setup) {
    return {
      mode: 'setup',
      title: 'Start a run: set up your crew',
      lead: SETUP_LEAD,
      role,
      roleLocked: hasCoordinator,
      command,
      listening: `Listening · ${lanes.length} of 2 to start a run`,
    };
  }
  return {
    mode: 'add',
    title: 'Add a member',
    lead: 'Another session joins as a contributor and takes the next piece of work.',
    role,
    roleLocked: hasCoordinator,
    command,
    listening: 'Listening · a new member shows above as it registers',
  };
}
