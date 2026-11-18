// The realtime channel adapter (#414, design D4): a RelayRealtimeTransport's `subscribe`
// over any client with the supabase-js channel shape. The package takes no dependency
// on supabase-js: the dōjō hands in its client, and these interfaces are all it uses.

import type { ChannelStatus } from '../../source.js';
import type { RelayStreamEvent } from '../types.js';
import { fromPostgresChange, isCrewScoped, RELAY_TABLES, type PostgresChangePayload, type RelayTableMap } from './postgres-change.js';

/** What the adapter needs of a channel: a `postgres_changes` listener and a subscribe. */
export interface RealtimeChannelLike {
  on(
    type: 'postgres_changes',
    filter: { event: '*'; schema: string; table: string; filter?: string },
    callback: (payload: PostgresChangePayload) => void,
  ): RealtimeChannelLike;
  subscribe(callback?: (status: string, err?: Error) => void): RealtimeChannelLike;
}

/** What the adapter needs of a client: open a channel, remove it. */
export interface RealtimeClientLike {
  channel(topic: string): RealtimeChannelLike;
  removeChannel(channel: RealtimeChannelLike): unknown;
}

export interface RealtimeChannelSpec {
  /** The channel's topic; each subscription opens it under its own suffix. */
  topic: string;
  /** The schema the relay tables live in. */
  schema: string;
  /** The crew this channel carries, or null for every crew the person can see. The topic
   *  scopes nothing (postgres_changes ignore it): a crew's channel asks the server for
   *  the crew's rows on each crew-scoped table, and drops any row that names another
   *  crew. Required, so a caller says which it means. */
  crewId: string | null;
  /** Table → entry (see `RelayTableEntry`); `RELAY_TABLES` when not given. */
  tables?: RelayTableMap;
}

const STATUSES: readonly string[] = ['SUBSCRIBED', 'TIMED_OUT', 'CLOSED', 'CHANNEL_ERROR'] satisfies ChannelStatus[];

/** False when the change's row names a crew other than `crewId`. A row with no crew (a
 *  draft run, a queue item, a DELETE's key-only `old`) is the person's and passes. */
function inCrew(payload: PostgresChangePayload, crewId: string | null): boolean {
  if (crewId === null) return true;
  const row = payload.eventType === 'DELETE' ? payload.old : payload.new;
  return typeof row.crew_id !== 'string' || row.crew_id === crewId;
}

/** Subscriptions opened by this module; the suffix that keeps their topics apart. */
let opened = 0;

/** A transport's `subscribe`: one channel per subscription, a listener per table, each
 *  change mapped to the daemon's event and each status passed on. The teardown removes
 *  the channel; nothing reaches the handlers after it.
 *
 *  Each subscription gets its own topic (`<topic>:<n>`). supabase-js hands back the
 *  channel it already holds for a topic, and adding a listener to a subscribed channel
 *  throws, so two views on one client and crew, or a re-subscribe while the last
 *  removal is in flight, would otherwise break. postgres_changes filter by schema,
 *  table and row filter, not by topic, so the suffix changes nothing the server sends;
 *  `crewId` is what scopes the channel. */
export function realtimeChannel(client: RealtimeClientLike, spec: RealtimeChannelSpec) {
  const tables = spec.tables ?? RELAY_TABLES;
  return {
    subscribe(
      on: (event: RelayStreamEvent) => void,
      onStatus: (status: ChannelStatus, err?: Error) => void,
    ): () => void {
      let open = true;
      opened += 1;
      let channel = client.channel(`${spec.topic}:${opened}`);
      for (const table of Object.keys(tables)) {
        const filter: { event: '*'; schema: string; table: string; filter?: string } = { event: '*', schema: spec.schema, table };
        if (spec.crewId !== null && isCrewScoped(tables[table])) filter.filter = `crew_id=eq.${spec.crewId}`;
        channel = channel.on('postgres_changes', filter, (payload) => {
          if (!open || !inCrew(payload, spec.crewId)) return;
          const event = fromPostgresChange(payload, tables);
          if (event) on(event);
          else console.warn('relay realtime: no entity for the table; change skipped', `${payload.schema}.${payload.table}`);
        });
      }
      channel = channel.subscribe((status, err) => {
        if (!open) return;
        if (!STATUSES.includes(status)) {
          console.warn('relay realtime: unknown channel status', status, spec.topic, err);
          return;
        }
        // The channel is down: say so, with what supabase-js said (RLS, JWT, publication).
        if (status !== 'SUBSCRIBED') console.warn('relay realtime: channel', status, spec.topic, err);
        onStatus(status as ChannelStatus, err);
      });
      return () => {
        if (!open) return;
        open = false;
        Promise.resolve(client.removeChannel(channel)).catch((err: unknown) =>
          console.warn('relay realtime: removing the channel failed', spec.topic, err),
        );
      };
    },
  };
}
