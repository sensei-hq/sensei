// The Queue view's selectors (#423; CONTRACT §1d, §3h; COMPONENTS §3c QueueRow): from the
// queue read to the rows QueueRow draws. Pure.

import { providerLabel } from './format.js';
import type { RelayTone } from './now-selectors.js';
import type { RelayAddToBacklog, RelayBacklogItem, RelayItemKind, RelayQueue, RelayTriageItem } from './types.js';

/** One queue row, as QueueRow draws it. */
export interface RelayQueueRowView {
  /** The item's id, or `provider:key` for a tracker task with no item yet. */
  id: string;
  title: string;
  kind: { label: string; tone: RelayTone };
  /** In the contract's order: sprint or milestone, added by you, the repository, its own tags. */
  tags: string[];
  ticket: { provider: string; key: string; url: string | null } | null;
  /** Triage only: + adds it to the backlog. */
  canAdd: boolean;
  /** What + sends: the row's own identity (an item's id, or a tracker task's ticket). */
  add: RelayAddToBacklog | null;
  /** Backlog only: picked for a run. */
  selectable: boolean;
}

const KIND: Record<RelayItemKind, { label: string; tone: RelayTone }> = {
  bug: { label: 'bug', tone: 'danger' },
  gap: { label: 'gap', tone: 'danger' },
  feature: { label: 'feature', tone: 'success' },
  finding: { label: 'review finding', tone: 'warning' },
  issue: { label: 'issue', tone: 'warning' },
};

/** A repository key as people say it: its last segment ("github.com/sensei-hq/sensei" → "sensei"). */
const repoName = (repo: string | null) => (repo ? (repo.split('/').pop() ?? repo) : null);

function tagsOf(item: { sprint: string | null; added_by_you: boolean; repo_key: string | null; tags: string[] }): string[] {
  const out: string[] = [];
  if (item.sprint) out.push(item.sprint);
  if (item.added_by_you) out.push('added by you');
  const repo = repoName(item.repo_key);
  if (repo) out.push(repo);
  for (const t of item.tags) if (!out.includes(t)) out.push(t);
  return out;
}

const ticketOf = (t: RelayTriageItem['ticket'] | RelayBacklogItem['ticket']) =>
  t ? { provider: providerLabel(t.provider), key: t.key, url: t.url ?? null } : null;

function triageRow(item: RelayTriageItem): RelayQueueRowView {
  const id = item.item_id ?? `${item.ticket.provider}:${item.ticket.key}`;
  return {
    id,
    title: item.title,
    kind: KIND[item.kind] ?? { label: item.kind, tone: 'neutral' },
    tags: tagsOf(item),
    ticket: ticketOf(item.ticket),
    canAdd: true,
    add: item.item_id !== null ? { item_id: item.item_id } : { ticket_provider: item.ticket.provider, ticket_key: item.ticket.key },
    selectable: false,
  };
}

function backlogRow(item: RelayBacklogItem): RelayQueueRowView {
  return {
    id: item.item_id,
    title: item.title,
    kind: KIND[item.kind] ?? { label: item.kind, tone: 'neutral' },
    tags: tagsOf(item),
    ticket: ticketOf(item.ticket),
    canAdd: false,
    add: null,
    selectable: true,
  };
}

/** The Queue's two lists: Triage as read, the backlog in its order. */
export function toQueueRows(queue: RelayQueue | null): { triage: RelayQueueRowView[]; backlog: RelayQueueRowView[] } {
  if (!queue) return { triage: [], backlog: [] };
  return {
    triage: queue.triage.map(triageRow),
    backlog: [...queue.backlog].sort((a, b) => a.position - b.position).map(backlogRow),
  };
}
