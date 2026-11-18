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

/** Table → the entity the daemon's stream names. The default for `fromPostgresChange`;
 *  the dōjō passes its own map where its table names differ. */
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

/** The daemon's event for one change; null for a table the map does not name (the caller reports it). */
export function fromPostgresChange(
  payload: PostgresChangePayload,
  tables: Readonly<Record<string, string>> = RELAY_TABLES,
): RelayStreamEvent | null {
  const entity = Object.hasOwn(tables, payload.table) ? tables[payload.table] : undefined;
  if (!entity) return null;
  const row = payload.eventType === 'DELETE' ? payload.old : payload.new;
  const data: Record<string, unknown> = { ...row };
  const key = ID_KEYS[entity];
  if (key && typeof row.id === 'string') data[key] = row.id;
  return { action: ACTIONS[payload.eventType], entity, data: data as RelayStreamEvent['data'] };
}
