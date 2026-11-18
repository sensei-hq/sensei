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
 *  A table row (a bare entity name, or `{entity, key?, crewScoped?}`) is today's `relay`
 *  schema: its columns are not what the screen holds (`relay.items` has `state`, not
 *  `stage`; `relay.runs` has no task counts; `relay.lanes` carries `token_hash`), so the
 *  change names the entity's id and the state re-reads it. `key` is the column that holds
 *  that id (default `id`: a lease names its lane by `lane_id`). `{entity, screenRow: true}`
 *  is a table a publisher writes in the screen's shape: its row is applied as it is.
 *  `crewScoped` says every row names its crew in `crew_id`, so a crew's channel asks the
 *  server for that crew's rows only. */
export type RelayTableEntry =
  | string
  | { entity: string; key?: string; crewScoped?: boolean; screenRow?: never }
  | { entity: string; screenRow: true; crewScoped?: boolean };

/** Table → entry. */
export type RelayTableMap = Readonly<Record<string, RelayTableEntry>>;

/** Table → entry for today's `relay` schema (`database/ddl/table/relay/`): table rows, so
 *  each change re-reads. Only tables the DDL has: realtime joins every binding at once and
 *  the server refuses the join when one names a missing table. Decisions and messages are
 *  `relay.actions` kinds, so an action names its task (none: a crew-level message, which
 *  re-reads Now); a lease is a lane's presence, so it names its lane. A draft run has no
 *  crew yet and the queue is the person's across crews: neither is crew-scoped. The
 *  default for `fromPostgresChange` and `realtimeChannel`; the dōjō passes its own map
 *  where its table names or row shapes differ. */
export const RELAY_TABLES: RelayTableMap = {
  lanes: { entity: 'lane', crewScoped: true },
  leases: { entity: 'lane', key: 'lane_id', crewScoped: true },
  tasks: { entity: 'task', crewScoped: true },
  actions: { entity: 'task', key: 'task_id', crewScoped: true },
  runs: 'run',
  items: 'item',
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
  if (typeof entry === 'string' || !entry.screenRow) {
    const data: Record<string, unknown> = { op: payload.eventType.toLowerCase() };
    const id = row[(typeof entry === 'string' ? undefined : entry.key) ?? 'id'];
    if (typeof id === 'string') data[ID_KEYS[entity] ?? `${entity}_id`] = id;
    return { action, entity, data: data as RelayStreamEvent['data'] };
  }
  const data: Record<string, unknown> = { ...row };
  const key = ID_KEYS[entity];
  if (key && typeof row.id === 'string') data[key] = row.id;
  return { action, entity, data: data as RelayStreamEvent['data'] };
}

/** True when the table's rows all name their crew in `crew_id`. */
export function isCrewScoped(entry: RelayTableEntry | undefined): boolean {
  return typeof entry === 'object' && entry.crewScoped === true;
}
