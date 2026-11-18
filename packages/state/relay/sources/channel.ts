// The realtime channel adapter (#414, design D4): a RelayRealtimeTransport's `subscribe`
// over any client with the supabase-js channel shape. The package takes no dependency
// on supabase-js: the dōjō hands in its client, and these interfaces are all it uses.

import type { ChannelStatus } from '../../source.js';
import type { RelayStreamEvent } from '../types.js';
import { fromPostgresChange, RELAY_TABLES, type PostgresChangePayload } from './postgres-change.js';

/** What the adapter needs of a channel: a `postgres_changes` listener and a subscribe. */
export interface RealtimeChannelLike {
  on(
    type: 'postgres_changes',
    filter: { event: '*'; schema: string; table: string },
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
  /** The channel's topic. */
  topic: string;
  /** The schema the relay tables live in. */
  schema: string;
  /** Table → entity; `RELAY_TABLES` when not given. */
  tables?: Readonly<Record<string, string>>;
}

const STATUSES: readonly string[] = ['SUBSCRIBED', 'TIMED_OUT', 'CLOSED', 'CHANNEL_ERROR'] satisfies ChannelStatus[];

/** A transport's `subscribe`: one channel per subscription, a listener per table, each
 *  change mapped to the daemon's event and each status passed on. The teardown removes
 *  the channel; nothing reaches the handlers after it. */
export function realtimeChannel(client: RealtimeClientLike, spec: RealtimeChannelSpec) {
  const tables = spec.tables ?? RELAY_TABLES;
  return {
    subscribe(on: (event: RelayStreamEvent) => void, onStatus: (status: ChannelStatus) => void): () => void {
      let open = true;
      let channel = client.channel(spec.topic);
      for (const table of Object.keys(tables)) {
        channel = channel.on('postgres_changes', { event: '*', schema: spec.schema, table }, (payload) => {
          if (!open) return;
          const event = fromPostgresChange(payload, tables);
          if (event) on(event);
          else console.warn('relay realtime: no entity for the table; change skipped', `${payload.schema}.${payload.table}`);
        });
      }
      channel = channel.subscribe((status, err) => {
        if (!open) return;
        if (STATUSES.includes(status)) onStatus(status as ChannelStatus);
        else console.warn('relay realtime: unknown channel status', status, spec.topic, err);
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
