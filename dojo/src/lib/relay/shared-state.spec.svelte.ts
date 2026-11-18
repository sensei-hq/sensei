// The dōjō hosts the shared relay state (#414): RelayState and RelayTaskState from
// @sensei/state, built over the realtime source on a transport whose channel is
// `realtimeChannel` over a supabase-js-shaped client. Canned postgres_changes payloads
// in, data out — no DOM, no daemon, no Supabase.
import { describe, it, expect, vi } from 'vitest';
import type { SupabaseClient } from '@supabase/supabase-js';
import {
	RelayState,
	RelayTaskState,
	cannedRealtime,
	realtimeChannel,
	relayRealtimeSource,
	relayRealtimeTaskSource,
	type PostgresChangePayload,
	type RealtimeClientLike,
	type RelayRealtimeTransport
} from '@sensei/state';
import * as F from '../../../../packages/state/relay/fixtures.js';

// The dōjō hands in its supabase-js client: it must satisfy the package's structural client.
export const asRealtimeClient = (client: SupabaseClient): RealtimeClientLike => client;

type Listener = { table: string; cb: (p: PostgresChangePayload) => void };

/** A supabase-js-shaped client: the channels it opened, their listeners and status callbacks. */
function fakeSupabase() {
	const channels: Array<{ listeners: Listener[]; status: (s: string) => void }> = [];
	const client: RealtimeClientLike = {
		channel() {
			const record = { listeners: [] as Listener[], status: (_s: string) => {} };
			channels.push(record);
			const channel = {
				on(_type: 'postgres_changes', filter: { table: string }, cb: Listener['cb']) {
					record.listeners.push({ table: filter.table, cb });
					return channel;
				},
				subscribe(cb?: (s: string) => void) {
					if (cb) record.status = cb;
					return channel;
				}
			};
			return channel;
		},
		removeChannel: vi.fn()
	};
	const last = () => channels[channels.length - 1];
	return {
		client,
		/** The server reports a status on the latest channel. */
		status: (s: string) => last().status(s),
		/** The server delivers a change on the latest channel. */
		change: (table: string, eventType: PostgresChangePayload['eventType'], row: Record<string, unknown>) => {
			const p: PostgresChangePayload = {
				schema: 'relay',
				table,
				eventType,
				new: eventType === 'DELETE' ? {} : row,
				old: eventType === 'DELETE' ? row : {}
			};
			for (const l of last().listeners) if (l.table === table) l.cb(p);
		}
	};
}

/** The dōjō's transport: canned reads (spied), recorded writes, and the realtime channel. */
function transport() {
	const sb = fakeSupabase();
	const canned = cannedRealtime({
		now: F.now({ decisions: [F.decision()] }),
		queue: F.queue(),
		task: F.taskView()
	});
	const t = {
		...canned,
		readNow: vi.fn(canned.readNow),
		readQueue: vi.fn(canned.readQueue),
		readTask: vi.fn(canned.readTask),
		subscribe: realtimeChannel(sb.client, { topic: `relay:${F.CREW_ID}`, schema: 'relay' }).subscribe
	} satisfies RelayRealtimeTransport;
	return { t, sb, writes: canned.writes };
}

const settle = () => new Promise((r) => setTimeout(r, 0));
const reads = () => ({ now: F.now({ decisions: [F.decision()] }), nowError: null, queue: F.queue(), queueError: null });

describe('RelayState in the dōjō, over the realtime source', () => {
	it('moves lanes and decisions on canned postgres_changes, with no re-read', async () => {
		const { t, sb } = transport();
		const s = new RelayState(relayRealtimeSource(t), {}, reads(), { surface: 'dojo' });
		s.connect();
		const lane = s.lanes.items[0];
		sb.change('lanes', 'UPDATE', { ...lane, title: 'Live from the dōjō' });
		sb.change('decisions', 'DELETE', { id: 'ask-1' });
		await settle();
		expect(s.now?.lanes.find((l) => l.id === lane.id)?.title).toBe('Live from the dōjō');
		expect(s.now?.decisions).toEqual([]);
		expect(t.readNow).not.toHaveBeenCalled();
		expect(t.readQueue).not.toHaveBeenCalled();
	});

	it('re-reads once on each SUBSCRIBED, the first and after a drop', async () => {
		const { t, sb } = transport();
		const s = new RelayState(relayRealtimeSource(t), {}, reads(), { surface: 'dojo' });
		s.connect();
		sb.status('SUBSCRIBED');
		await settle();
		expect(t.readNow).toHaveBeenCalledTimes(1);
		expect(t.readQueue).toHaveBeenCalledTimes(1);
		sb.status('CHANNEL_ERROR');
		await settle();
		expect(t.readNow).toHaveBeenCalledTimes(1);
		sb.status('SUBSCRIBED');
		await settle();
		expect(t.readNow).toHaveBeenCalledTimes(2);
	});

	it('answers a decision as the dōjō', async () => {
		const { t, writes } = transport();
		const s = new RelayState(relayRealtimeSource(t), {}, reads(), { surface: 'dojo' });
		expect(await s.answer(F.decision(), 'db')).toBeNull();
		expect(writes).toEqual([
			{ kind: 'answer', body: { decision_id: 'ask-1', option: 'db', note: null, surface: 'dojo' } }
		]);
	});
});

describe('RelayTaskState in the dōjō, over the realtime task source', () => {
	const taskId = F.taskView().task.id;
	const build = () => {
		const x = transport();
		const s = new RelayTaskState(
			{ view: F.taskView(), error: null },
			relayRealtimeTaskSource(x.t),
			F.CREW_ID,
			taskId,
			{ surface: 'dojo' }
		);
		return { ...x, s };
	};

	it('ignores another task’s change, and re-reads only the viewed task when it changes', async () => {
		const { t, sb, s } = build();
		s.connect();
		sb.change('tasks', 'UPDATE', { id: 't-other', state: 'working' });
		sb.change('lanes', 'UPDATE', { id: 'l-1', title: 'Elsewhere' });
		await settle();
		expect(t.readTask).not.toHaveBeenCalled();
		sb.change('tasks', 'UPDATE', { id: taskId, state: 'in_review' });
		await settle();
		expect(t.readTask).toHaveBeenCalledTimes(1);
		expect(t.readTask).toHaveBeenCalledWith(F.CREW_ID, taskId);
	});

	it('re-reads once on SUBSCRIBED', async () => {
		const { t, sb, s } = build();
		s.connect();
		sb.status('SUBSCRIBED');
		await settle();
		expect(t.readTask).toHaveBeenCalledTimes(1);
	});

	it('answers as the dōjō, then re-reads the task', async () => {
		const { t, writes, s } = build();
		expect(await s.answer(F.decision(), 'file')).toBeNull();
		expect(writes).toEqual([
			{ kind: 'answer', body: { decision_id: 'ask-1', option: 'file', note: null, surface: 'dojo' } }
		]);
		expect(t.readTask).toHaveBeenCalledTimes(1);
	});

	it('the teardown removes the channel', () => {
		const { sb, s } = build();
		const stop = s.connect();
		stop();
		expect(sb.client.removeChannel).toHaveBeenCalledTimes(1);
	});
});
