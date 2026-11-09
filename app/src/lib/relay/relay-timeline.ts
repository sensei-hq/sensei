// Relay · the task timeline (F8): one thin row per broker-known action,
// oldest first (the lifecycle story; History keeps the newest-first scan).
// Every kind maps — an unmapped kind is a bug, so unknown kinds render
// honestly as unknown, never silently. A cancel writes no action (F3),
// so a cancelled task gains a derived row stamped from the header.
import type { RelayAction, RelayLaneRef, RelayTaskView } from '$lib/relay-api.js';
import { codeOf } from './relay-format.js';

export interface TimelineRow {
  /** Stable: the action id, or 'terminal-cancelled' for the derived row. */
  id: string;
  /** The action's stamp (derived rows: the header's state_changed_at). */
  at: string;
  /** Who acted, by code; 'you' when the person did. */
  actor: string;
  /** One-line gist. */
  title: string;
  /** The action for the expanded detail; null for derived rows. */
  action: RelayAction | null;
}

const turn = (a: Extract<RelayAction, { kind: 'reported' }>): string =>
  `${(a.round ?? 1) > 1 ? 'Rework' : 'Work'} · ${a.phase ?? 'work'} r${a.round ?? 1}`;

function titleFor(action: RelayAction, lanes: RelayLaneRef[]): string | null {
  switch (action.kind) {
    case 'assigned':
      return `Assigned to ${action.lane_code ?? '—'}`;
    case 'brief_updated':
      return 'Brief updated';
    case 'started':
      return 'Started';
    case 'reported':
      return `${action.props.partial ? 'Partial report' : 'Reported'} — ${turn(action)}`;
    case 'reviewed':
      return action.props.verdict === 'approved' ? 'Review approved' : 'Changes requested';
    case 'asked':
      return action.props.asked_of === 'person' ? 'Asked you' : `Asked ${codeOf(lanes, action.props.asked_of)}`;
    case 'answered':
      return `Answered: ${action.props.option}`;
    case 'message':
      return `Message to ${action.props.to.kind === 'lane' ? codeOf(lanes, action.props.to.lane_id) : action.props.to.kind}`;
    case 'merged':
      return `Merged into ${action.props.into}`;
    case 'interrupted':
      return 'Lease lost';
    case 'reoffered':
      return action.props.to_lane ? `Re-offered to ${codeOf(lanes, action.props.to_lane)}` : 'Returned to the queue';
    case 'compacted':
      return 'Compacted';
    case 'handed_over':
      return `Handed over ${codeOf(lanes, action.props.from_lane)} → ${codeOf(lanes, action.props.to_lane)}`;
    case 'delivered':
    case 'read':
      // Receipts fold into their message (History's precedent), never rows.
      return null;
    default:
      return `Unknown event (${(action as { kind: string }).kind})`;
  }
}

export function buildTimeline(view: RelayTaskView): TimelineRow[] {
  const rows: TimelineRow[] = [];
  // History reads newest-first; the timeline tells it oldest-first.
  for (const action of [...view.history].reverse()) {
    const title = titleFor(action, view.lanes);
    if (title === null) continue;
    rows.push({
      id: action.id,
      at: action.created_at,
      actor: action.lane_code ?? 'you',
      title,
      action,
    });
  }
  if (view.task.state === 'cancelled') {
    rows.push({
      id: 'terminal-cancelled',
      at: view.task.state_changed_at,
      actor: 'coordinator',
      title: 'Cancelled',
      action: null,
    });
  }
  return rows;
}

/** A message's receipts, folded by the message they reply to (§1e.6, F5). */
export function foldReceipts(history: RelayAction[]): Record<string, 'sent' | 'delivered' | 'read'> {
  const out: Record<string, 'sent' | 'delivered' | 'read'> = {};
  for (const a of history) {
    if (!a.reply_to) continue;
    if (a.kind === 'read') out[a.reply_to] = 'read';
    else if (a.kind === 'delivered' && out[a.reply_to] !== 'read') out[a.reply_to] = 'delivered';
  }
  return out;
}
