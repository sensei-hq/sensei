// The Supabase mapping (#414, design D3): a `postgres_changes` payload on a relay table,
// as the daemon would stream the same change. INSERT is add, UPDATE update, DELETE
// remove (the row from `old`), and the row carries the id key the daemon names its
// entity by, so RelayState and RelayTaskState cannot tell the two planes apart.

import type { RelayStreamEvent } from '../types.js';

/** One change as supabase-js delivers it to a `postgres_changes` listener. */
export interface PostgresChangePayload {
  schema: string;
  table: string;
  eventType: 'INSERT' | 'UPDATE' | 'DELETE';
  new: Record<string, unknown>;
  old: Record<string, unknown>;
}

/** One table's entry: the entity it holds, and whether its rows are the screen's rows.
 *  A bare entity name is a table row (today's `relay` schema): its columns are not what
 *  the screen holds (`relay.items` has `state`, not `stage`; `relay.runs` has no task
 *  counts; `relay.lanes` carries `token_hash`), so the change names the row's id and the
 *  state re-reads it. `{entity, screenRow: true}` is a table a publisher writes in the
 *  screen's shape: its row is applied as it is. */
export type RelayTableEntry = string | { entity: string; screenRow: true };

/** Table → entry. */
export type RelayTableMap = Readonly<Record<string, RelayTableEntry>>;

/** Table → the entity the daemon's stream names, for today's `relay` schema: table rows,
 *  so each change re-reads. The default for `fromPostgresChange`; the dōjō passes its own
 *  map where its table names or row shapes differ. */
export const RELAY_TABLES: Readonly<Record<string, string>> = {
  lanes: 'lane',
  tasks: 'task',
  runs: 'run',
  items: 'item',
  decisions: 'decision',
  messages: 'message',
};

/** The id key the daemon names an entity by, beside the row's `id`. */
const ID_KEYS: Readonly<Record<string, string>> = { task: 'task_id', lane: 'lane_id', run: 'run_id' };

const ACTIONS = { INSERT: 'add', UPDATE: 'update', DELETE: 'remove' } as const;

/** The daemon's event for one change; null for a table the map does not name (the caller
 *  reports it). A table row becomes `{op, <entity>_id}` with no `id`, which the states
 *  read as "this changed, re-read it"; a screen-shaped row is carried whole. */
export function fromPostgresChange(payload: PostgresChangePayload, tables: RelayTableMap = RELAY_TABLES): RelayStreamEvent | null {
  const entry = Object.hasOwn(tables, payload.table) ? tables[payload.table] : undefined;
  if (!entry) return null;
  const entity = typeof entry === 'string' ? entry : entry.entity;
  const row = payload.eventType === 'DELETE' ? payload.old : payload.new;
  const action = ACTIONS[payload.eventType];
  if (typeof entry === 'string') {
    const data: Record<string, unknown> = { op: payload.eventType.toLowerCase() };
    if (typeof row.id === 'string') data[ID_KEYS[entity] ?? `${entity}_id`] = row.id;
    return { action, entity, data: data as RelayStreamEvent['data'] };
  }
  const data: Record<string, unknown> = { ...row };
  const key = ID_KEYS[entity];
  if (key && typeof row.id === 'string') data[key] = row.id;
  return { action, entity, data: data as RelayStreamEvent['data'] };
}
