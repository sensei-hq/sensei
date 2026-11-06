// Relay · the pure readings its screens share (D90; CONTRACT §1).
//
// Nothing here fetches or holds state: a lane's pill, a time on a task, a ticket's
// label, a crew's grouping. Both windows draw the same rows, so the wording lives
// in one place; and every one of them says "not reported" rather than inventing a
// zero (D83).
import type {
  RelayCrew,
  RelayLane,
  RelayLaneRef,
  RelayTicket,
  RelayTicketProvider,
} from '$lib/relay-api.js';

/** The tracker's own name, as the screen writes it. */
export function providerLabel(provider: RelayTicketProvider): string {
  if (provider === 'jira') return 'Jira';
  if (provider === 'ado') return 'Azure DevOps';
  return 'GitHub';
}

/** The ticket button's label: provider and key (CONTRACT §1d: "provider + key"). */
export const ticketLabel = (ticket: RelayTicket): string => `${providerLabel(ticket.provider)} ${ticket.key}`;

/** A local `HH:MM`, as the reader's clock shows it. */
export function hhmm(iso: string): string {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** How long something has been where it is: "38 min", "2 h 5 min". */
export function since(iso: string, now = Date.now()): string {
  const mins = Math.max(0, Math.round((now - new Date(iso).getTime()) / 60_000));
  if (mins < 60) return `${mins} min`;
  const hours = Math.floor(mins / 60);
  const rest = mins % 60;
  return rest === 0 ? `${hours} h` : `${hours} h ${rest} min`;
}

/** A crew's id, by the part that tells it apart (a uuid v7 starts with its clock). */
export const crewTail = (id: string): string => id.slice(-4);

/** One lane's state pill: the lease first, then the person's open question, then the task. */
export function laneState(lane: RelayLane): string {
  if (lane.lease_state === 'expired') return 'lease lost';
  if (!lane.task) return 'idle';
  if (lane.task.needs_answer) return 'needs your answer';
  return taskStateLabel(lane.task.state);
}

/** A task state as a person reads it. */
export function taskStateLabel(state: string): string {
  switch (state) {
    case 'in_review': return 'in review';
    case 'assigned': return 'assigned';
    case 'working': return 'working';
    case 'rework': return 'rework';
    case 'approved': return 'approved';
    case 'done': return 'done';
    case 'blocked': return 'blocked';
    case 'interrupted': return 'interrupted';
    case 'cancelled': return 'cancelled';
    default: return state.replace(/_/g, ' ');
  }
}

/** The rounds a task has had, by phase ("tests 1 · impl 2"). */
export const roundsLabel = (rounds: { phase: string; count: number }[]): string =>
  rounds.map((r) => `${r.phase} ${r.count}`).join(' · ');

/** A task's route, under whichever window's Relay is drawing it. */
export const taskHref = (base: string, crewId: string, taskId: string): string =>
  `${base}/${encodeURIComponent(crewId)}/${encodeURIComponent(taskId)}`;

/** A lane's members by code, so a uuid in props is read by its code. */
export function codeOf(lanes: RelayLaneRef[], id: string | null | undefined): string {
  if (!id) return '—';
  return lanes.find((l) => l.id === id)?.code ?? '—';
}

/** The crews' lane rows in the order the read gave them, each under its own crew. */
export function crewGroups(
  crews: RelayCrew[],
  lanes: RelayLane[],
): { crew: RelayCrew; lanes: RelayLane[] }[] {
  return crews
    .map((crew) => ({ crew, lanes: lanes.filter((l) => l.crew_id === crew.id) }))
    .filter((g) => g.lanes.length > 0);
}

/** "N completed", and first-pass approval only when a review reported it. */
export function todayLine(today: { completed: number; reviewed: number | null; first_pass_approved: number | null }): {
  value: string;
  text: string;
} {
  const value = `${today.completed} completed`;
  if (today.first_pass_approved == null || today.reviewed == null) {
    return { value, text: 'First-pass approval not reported' };
  }
  return { value, text: `${today.first_pass_approved} of ${today.reviewed} passed review first time` };
}
