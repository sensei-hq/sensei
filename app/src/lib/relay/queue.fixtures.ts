// The Queue as the r1 mockup draws it (Relay.dc.html, view "queue"): the bench renders
// these beside the artboard. Row data, the shape toQueueRows produces.
import type { RelayQueueRowView } from '@sensei/state';

const row = (
  id: string,
  title: string,
  kind: RelayQueueRowView['kind'],
  tags: string[],
  ticket: RelayQueueRowView['ticket'],
  triage: boolean,
): RelayQueueRowView => ({
  id,
  title,
  kind,
  tags,
  ticket,
  canAdd: triage,
  add: triage ? (ticket ? { ticket_provider: 'jira', ticket_key: ticket.key } : { item_id: id }) : null,
  selectable: !triage,
});

const BUG = { label: 'bug', tone: 'danger' } as const;
const GAP = { label: 'gap', tone: 'danger' } as const;
const FEATURE = { label: 'feature', tone: 'success' } as const;
const FINDING = { label: 'review finding', tone: 'warning' } as const;

/** The mockup's three triage rows. */
export const mockupTriage = (): RelayQueueRowView[] => [
  row('jira:SEN-58', 'Sync shows "synced" for a refused repository', BUG, ['sprint 47', 'sensei'], { provider: 'Jira', key: 'SEN-58', url: 'https://example.atlassian.net/browse/SEN-58' }, true),
  row('github:#231', 'Export a crew’s history', FEATURE, ['milestone 11', 'relay'], { provider: 'GitHub', key: '#231', url: 'https://github.com/sensei-hq/sensei/issues/231' }, true),
  row('item-3', 'The pre-commit hook doesn’t run the dōjō tests', FINDING, ['added by you', 'dōjō'], null, true),
];

/** The mockup's four backlog rows, in order. */
export const mockupBacklog = (): RelayQueueRowView[] => [
  row('b1', 'Settings · Dōjō: "sign in again" when a session lapses', GAP, ['added by you', 'sensei', 'sync hardening'], null, false),
  row('b2', 'Repositories tab · org repositories read', FEATURE, ['sprint 47', 'dōjō', 'D46'], { provider: 'Jira', key: 'SEN-61', url: 'https://example.atlassian.net/browse/SEN-61' }, false),
  row('b3', 'Sync status for refused repositories', BUG, ['milestone 11', 'sensei'], { provider: 'GitHub', key: '#218', url: 'https://github.com/sensei-hq/sensei/issues/218' }, false),
  row('b4', 'Workspace hub', FEATURE, ['sprint 22', 'dōjō', 'roadmap A4'], { provider: 'Azure DevOps', key: '4512', url: 'https://dev.azure.com/x/_workitems/edit/4512' }, false),
];
