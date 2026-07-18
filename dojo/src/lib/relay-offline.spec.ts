import { describe, it, expect, vi } from 'vitest';
import {
	memoryStore,
	loadDrafts,
	saveDraft,
	clearDrafts,
	enqueue,
	peekAll,
	remove,
	size,
	flushQueue,
	newActionId,
	type KeyValueStore,
	type QueuedAction,
	type RunDraft,
	type SendResult
} from './relay-offline';

// Relay P4.5 offline core — draft store + durable action queue + flush. All PURE
// over an injectable KeyValueStore, so these tests use the in-memory fake
// (memoryStore) and never touch real localStorage. The load-bearing guarantees:
// drafts survive close/reopen + tolerate corrupt storage; the queue is a stable
// FIFO deduped by id; flush is partial-failure safe and can't double-send.

describe('relay-offline draft store', () => {
	it('round-trips a run draft (segments + nudge) through save → load', () => {
		const store = memoryStore();
		const draft: RunDraft = {
			segments: { 1: { verdict: 'approve' }, 2: { verdict: 'request_changes', note: 'tighten the query' } },
			nudge: 'focus on the API first'
		};
		saveDraft(store, 'run-1', draft);
		expect(loadDrafts(store, 'run-1')).toEqual(draft);
	});

	it('returns an empty draft for a run that was never saved', () => {
		expect(loadDrafts(memoryStore(), 'unknown')).toEqual({ segments: {} });
	});

	it('clearDrafts removes a run draft', () => {
		const store = memoryStore();
		saveDraft(store, 'run-1', { segments: { 1: { verdict: 'comment' } } });
		clearDrafts(store, 'run-1');
		expect(loadDrafts(store, 'run-1')).toEqual({ segments: {} });
	});

	it('tolerates corrupt JSON in a draft key (returns empty, never throws)', () => {
		const store = memoryStore({ 'relay-offline:v1:draft:run-1': '{not json' });
		expect(() => loadDrafts(store, 'run-1')).not.toThrow();
		expect(loadDrafts(store, 'run-1')).toEqual({ segments: {} });
	});

	it('tolerates a non-object draft body (e.g. a bare number)', () => {
		const store = memoryStore({ 'relay-offline:v1:draft:run-1': '42' });
		expect(loadDrafts(store, 'run-1')).toEqual({ segments: {} });
	});

	it('tolerates a draft missing the segments field (defaults segments to {})', () => {
		const store = memoryStore({ 'relay-offline:v1:draft:run-1': JSON.stringify({ nudge: 'hi' }) });
		expect(loadDrafts(store, 'run-1')).toEqual({ segments: {}, nudge: 'hi' });
	});

	it('survives a throwing storage on read and on write (best-effort persistence)', () => {
		const throwing: KeyValueStore = {
			getItem: () => {
				throw new Error('SecurityError: storage disabled');
			},
			setItem: () => {
				throw new Error('QuotaExceededError');
			},
			removeItem: () => {
				throw new Error('nope');
			}
		};
		expect(loadDrafts(throwing, 'run-1')).toEqual({ segments: {} });
		expect(() => saveDraft(throwing, 'run-1', { segments: {} })).not.toThrow();
		expect(() => clearDrafts(throwing, 'run-1')).not.toThrow();
	});

	it('namespaces drafts per run so two runs do not collide', () => {
		const store = memoryStore();
		saveDraft(store, 'run-a', { segments: { 1: { verdict: 'approve' } } });
		saveDraft(store, 'run-b', { segments: { 1: { verdict: 'request_changes' } } });
		expect(loadDrafts(store, 'run-a').segments[1].verdict).toBe('approve');
		expect(loadDrafts(store, 'run-b').segments[1].verdict).toBe('request_changes');
		clearDrafts(store, 'run-a');
		// clearing run-a leaves run-b intact
		expect(loadDrafts(store, 'run-a')).toEqual({ segments: {} });
		expect(loadDrafts(store, 'run-b').segments[1].verdict).toBe('request_changes');
	});
});

describe('relay-offline action queue', () => {
	it('newActionId yields distinct ids', () => {
		const a = newActionId();
		const b = newActionId();
		expect(a).not.toBe(b);
	});

	it('enqueue appends in FIFO order and stamps id + queuedAt', () => {
		const store = memoryStore();
		let t = 0;
		const now = () => new Date(1_700_000_000_000 + t++ * 1000);
		const first = enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'first' } }, now);
		const second = enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'second' } }, now);
		expect(first.id).toBeTruthy();
		expect(first.queuedAt < second.queuedAt).toBe(true);
		const all = peekAll(store);
		expect(all.map((e) => (e.payload as { text: string }).text)).toEqual(['first', 'second']);
	});

	it('peekAll scopes to a run when a runId is given', () => {
		const store = memoryStore();
		enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'a' } });
		enqueue(store, { kind: 'nudge', runId: 'run-2', payload: { text: 'b' } });
		enqueue(store, { kind: 'review', runId: 'run-1', payload: { reviews: [] } });
		expect(size(store)).toBe(3);
		expect(size(store, 'run-1')).toBe(2);
		expect(size(store, 'run-2')).toBe(1);
		expect(peekAll(store, 'run-2').map((e) => e.runId)).toEqual(['run-2']);
	});

	it('remove drops exactly one entry by id (idempotent — removing twice is a no-op)', () => {
		const store = memoryStore();
		const e = enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'x' } });
		enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'y' } });
		remove(store, e.id);
		expect(size(store)).toBe(1);
		remove(store, e.id); // already gone
		expect(size(store)).toBe(1);
	});

	it('tolerates a corrupt queue key (reads as empty)', () => {
		const store = memoryStore({ 'relay-offline:v1:queue': 'not-an-array' });
		expect(peekAll(store)).toEqual([]);
		expect(size(store)).toBe(0);
		// and can still enqueue onto it (overwrites the garbage)
		enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'ok' } });
		expect(size(store)).toBe(1);
	});
});

describe('relay-offline flushQueue', () => {
	function seed(store: KeyValueStore): QueuedAction[] {
		enqueue(store, { kind: 'review', runId: 'run-1', payload: { reviews: [{ seq: 1, verdict: 'approve' }] } });
		enqueue(store, { kind: 'nudge', runId: 'run-1', payload: { text: 'steer' } });
		enqueue(store, { kind: 'reply', runId: 'run-1', payload: { inboxId: 'gate-1', reply: { verdict: 'approve' } } });
		return peekAll(store);
	}

	it('sends every entry, removes each on success, and reports them all sent', async () => {
		const store = memoryStore();
		const entries = seed(store);
		const sent: string[] = [];
		const sender = vi.fn(async (e: QueuedAction): Promise<SendResult> => {
			sent.push(e.id);
			return { ok: true };
		});
		const out = await flushQueue(store, entries, sender);
		expect(out.sent).toEqual(entries.map((e) => e.id));
		expect(out.failed).toEqual([]);
		expect(size(store)).toBe(0);
		expect(sent.length).toBe(3);
	});

	it('partial failure: successes are removed, failures stay queued', async () => {
		const store = memoryStore();
		const entries = seed(store);
		// The middle entry (nudge) fails; the review + reply succeed.
		const sender = vi.fn(async (e: QueuedAction): Promise<SendResult> =>
			e.kind === 'nudge' ? { ok: false, error: 'network error' } : { ok: true }
		);
		const out = await flushQueue(store, entries, sender);
		expect(out.sent.length).toBe(2);
		expect(out.failed.length).toBe(1);
		expect(out.failed[0].kind).toBe('nudge');
		// only the failed nudge remains queued
		const remaining = peekAll(store);
		expect(remaining.length).toBe(1);
		expect(remaining[0].kind).toBe('nudge');
	});

	it('a throwing/rejecting sender is caught → the entry is KEPT, not lost', async () => {
		const store = memoryStore();
		const entries = seed(store);
		const sender = vi.fn(async (e: QueuedAction): Promise<SendResult> => {
			if (e.kind === 'reply') throw new Error('boom');
			return { ok: true };
		});
		const out = await flushQueue(store, entries, sender);
		expect(out.failed.map((e) => e.kind)).toEqual(['reply']);
		expect(peekAll(store).map((e) => e.kind)).toEqual(['reply']);
	});

	it('re-flush is idempotent — a second pass over the same snapshot does not double-send removed entries', async () => {
		const store = memoryStore();
		const entries = seed(store);
		let calls = 0;
		const sender = vi.fn(async (): Promise<SendResult> => {
			calls++;
			return { ok: true };
		});
		// First flush delivers all three and empties the queue.
		await flushQueue(store, entries, sender);
		expect(calls).toBe(3);
		expect(size(store)).toBe(0);
		// A second flush over a FRESH snapshot of the (now empty) queue sends nothing.
		await flushQueue(store, peekAll(store), sender);
		expect(calls).toBe(3); // unchanged — no double-send
	});

	it('retry after a partial failure only re-sends the still-queued entry', async () => {
		const store = memoryStore();
		const entries = seed(store);
		let attempt = 0;
		const sender = vi.fn(async (e: QueuedAction): Promise<SendResult> => {
			attempt++;
			// nudge fails on the first pass, succeeds when retried
			if (e.kind === 'nudge' && attempt <= 3) return { ok: false, error: 'offline' };
			return { ok: true };
		});
		await flushQueue(store, entries, sender);
		expect(size(store)).toBe(1); // the nudge is still queued
		// Retry with a fresh snapshot — only the nudge is attempted, and it now sends.
		const retry = await flushQueue(store, peekAll(store), sender);
		expect(retry.sent.length).toBe(1);
		expect(size(store)).toBe(0);
	});
});
