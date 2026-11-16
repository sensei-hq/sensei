// The Queue view's selectors (#423; CONTRACT §1d, §3h; COMPONENTS §3c QueueRow): from the
// queue read to the rows QueueRow draws — title, kind first as a toned pill, then the
// tags in the contract's order, the ticket, and whether + adds it.
import { describe, it, expect } from 'vitest';
import { toQueueRows } from './queue-selectors.js';
import * as F from './fixtures.js';

describe('toQueueRows · Triage', () => {
  it('a tracker task: kind, sprint, the repository, its tags; the ticket by provider and key; + adds it by its ticket', () => {
    const [row] = toQueueRows(F.queue()).triage;
    expect(row).toEqual({
      id: 'jira:SEN-58',
      title: 'Sync shows "synced" for a refused repository',
      kind: { label: 'bug', tone: 'danger' },
      tags: ['sprint 47', 'sensei'],
      ticket: { provider: 'Jira', key: 'SEN-58', url: 'https://acme.atlassian.net/browse/SEN-58' },
      canAdd: true,
      add: { ticket_provider: 'jira', ticket_key: 'SEN-58' },
      selectable: false,
    });
  });

  it('an item the person added: kind, "added by you", the repository, its own tags; no ticket; + adds it by its id', () => {
    const manual = toQueueRows(F.queue()).triage[1];
    expect(manual.tags[0]).toBe('added by you');
    expect(manual.ticket).toBeNull();
    expect(manual.add).toEqual({ item_id: manual.id });
  });

  it('each kind its tone: a bug or a gap danger, a feature success, a finding (a review finding) or an issue warning', () => {
    const kinds = (['bug', 'gap', 'feature', 'finding', 'issue'] as const).map(
      (kind) => toQueueRows(F.queue({ triage: [F.manualItem({ kind })] })).triage[0].kind,
    );
    expect(kinds).toEqual([
      { label: 'bug', tone: 'danger' },
      { label: 'gap', tone: 'danger' },
      { label: 'feature', tone: 'success' },
      { label: 'review finding', tone: 'warning' },
      { label: 'issue', tone: 'warning' },
    ]);
  });
});

describe('toQueueRows · Backlog', () => {
  it('in backlog order, no +, each selectable for a run', () => {
    const queue = F.queue({ backlog: [F.backlogItem({ item_id: 'b2', position: 2 }), F.backlogItem({ item_id: 'b1', position: 1 })] });
    const rows = toQueueRows(queue).backlog;
    expect(rows.map((r) => r.id)).toEqual(['b1', 'b2']);
    expect(rows.every((r) => !r.canAdd && r.add === null && r.selectable)).toBe(true);
  });

  it('no read, no rows', () => {
    expect(toQueueRows(null)).toEqual({ triage: [], backlog: [] });
  });
});
