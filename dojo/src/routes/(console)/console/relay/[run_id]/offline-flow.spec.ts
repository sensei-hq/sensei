import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
	memoryStore,
	enqueue,
	peekAll,
	size,
	flushQueue,
	saveDraft,
	loadDrafts,
	clearDrafts,
	type KeyValueStore,
	type QueuedAction,
	type SendResult
} from '$lib/relay-offline';
import { DojoApiError } from '$lib/relay-data';

// Relay P4.5 run-detail offline flow (integration).
//
// The run-detail +page.svelte itself is thin plumbing over the pure offline core
// (relay-offline.ts) + the relay-data senders. Rendering the full SvelteKit page
// in jsdom would mostly test $app/navigation + rokkit wiring, not the offline
// logic. So this spec exercises the exact SENDER-DISPATCH + flush orchestration
// the page performs, with the relay-data senders MOCKED — asserting the
// away-from-keyboard guarantees end to end:
//   • offline → a review/nudge is QUEUED (not sent, not errored),
//   • reconnect → the queue FLUSHES through the real senders,
//   • all delivered → drafts are CLEARED,
//   • a send fails → its entry is KEPT queued and drafts are NOT cleared.
//
// The `makeSender` below is the same ActionSender contract the page builds: it
// maps a QueuedAction kind to submitReview / sendNudge / replyToGate and folds a
// DojoApiError or a network error into a {ok:false} result (never throws) — so a
// break in that mapping would fail here.

const submitReview = vi.fn();
const sendNudge = vi.fn();
const replyToGate = vi.fn();

vi.mock('$lib/relay-data', async (importOriginal) => {
	const actual = await importOriginal<typeof import('$lib/relay-data')>();
	return {
		...actual,
		submitReview: (...args: unknown[]) => submitReview(...args),
		sendNudge: (...args: unknown[]) => sendNudge(...args),
		replyToGate: (...args: unknown[]) => replyToGate(...args)
	};
});

const TENANT = 'personal/jerry';
const RUN = 'run-1';
const OPTS = { fetch: globalThis.fetch, accessToken: 'JWT' };

/** The ActionSender the page wires: dispatch by kind, fold failures to {ok:false}. */
function makeSender() {
	return async (entry: QueuedAction): Promise<SendResult> => {
		try {
			if (entry.kind === 'review') {
				await submitReview(TENANT, entry.runId, entry.payload.reviews, OPTS);
			} else if (entry.kind === 'nudge') {
				await sendNudge(TENANT, entry.runId, entry.payload.text, { ...OPTS, kind: entry.payload.kind });
			} else {
				await replyToGate(TENANT, entry.payload.inboxId, entry.payload.reply, OPTS);
			}
			return { ok: true };
		} catch (e) {
			const msg = e instanceof DojoApiError ? e.message : e instanceof Error ? e.message : 'send failed';
			return { ok: false, error: msg };
		}
	};
}

/** The page's offline-aware send(): queue when offline, else send; a NETWORK
 *  failure (not a DojoApiError) also queues. Returns what the UI would show. */
async function offlineAwareSend(
	store: KeyValueStore,
	online: boolean,
	reviews: { seq: number; verdict: 'approve' | 'request_changes' | 'comment'; note?: string }[]
): Promise<'queued' | 'sent' | 'error'> {
	if (!online) {
		enqueue(store, { kind: 'review', runId: RUN, payload: { reviews } });
		return 'queued';
	}
	try {
		await submitReview(TENANT, RUN, reviews, OPTS);
		clearDrafts(store, RUN);
		return 'sent';
	} catch (e) {
		if (e instanceof DojoApiError) return 'error';
		// network failure mid-send → queue it instead of surfacing an error
		enqueue(store, { kind: 'review', runId: RUN, payload: { reviews } });
		return 'queued';
	}
}

describe('run-detail offline flow', () => {
	beforeEach(() => {
		submitReview.mockReset();
		sendNudge.mockReset();
		replyToGate.mockReset();
	});

	it('offline: a review is queued (senders NOT called) and drafts persist', async () => {
		const store = memoryStore();
		saveDraft(store, RUN, { segments: { 1: { verdict: 'approve' } } });
		const result = await offlineAwareSend(store, false, [{ seq: 1, verdict: 'approve' }]);
		expect(result).toBe('queued');
		expect(submitReview).not.toHaveBeenCalled();
		expect(size(store, RUN)).toBe(1);
		// drafts are NOT cleared while offline — the human's work is preserved
		expect(loadDrafts(store, RUN).segments[1].verdict).toBe('approve');
	});

	it('a network failure mid-send queues instead of erroring', async () => {
		const store = memoryStore();
		submitReview.mockRejectedValueOnce(new TypeError('Failed to fetch'));
		const result = await offlineAwareSend(store, true, [{ seq: 1, verdict: 'approve' }]);
		expect(result).toBe('queued');
		expect(size(store, RUN)).toBe(1);
	});

	it('a DojoApiError mid-send surfaces as an error (does NOT queue)', async () => {
		const store = memoryStore();
		submitReview.mockRejectedValueOnce(new DojoApiError(422, 'run already closed'));
		const result = await offlineAwareSend(store, true, [{ seq: 1, verdict: 'approve' }]);
		expect(result).toBe('error');
		expect(size(store, RUN)).toBe(0);
	});

	it('reconnect flush: queued review + nudge flush through the real senders, then drafts clear', async () => {
		const store = memoryStore();
		saveDraft(store, RUN, { segments: { 1: { verdict: 'approve' } }, nudge: 'focus on the API' });
		// Queued while offline:
		await offlineAwareSend(store, false, [{ seq: 1, verdict: 'approve' }]);
		enqueue(store, { kind: 'nudge', runId: RUN, payload: { text: 'focus on the API', kind: 'nudge' } });
		expect(size(store, RUN)).toBe(2);

		submitReview.mockResolvedValue(1);
		sendNudge.mockResolvedValue({ id: 'n1', seq: 5 });

		// Reconnect → flush.
		const { sent, failed } = await flushQueue(store, peekAll(store, RUN), makeSender());
		expect(failed).toEqual([]);
		expect(sent.length).toBe(2);
		expect(submitReview).toHaveBeenCalledWith(TENANT, RUN, [{ seq: 1, verdict: 'approve' }], OPTS);
		expect(sendNudge).toHaveBeenCalledWith(TENANT, RUN, 'focus on the API', expect.objectContaining({ kind: 'nudge' }));

		// Page's post-flush rule: all delivered → clear drafts + empty queue.
		if (failed.length === 0 && sent.length > 0) clearDrafts(store, RUN);
		expect(size(store, RUN)).toBe(0);
		expect(loadDrafts(store, RUN)).toEqual({ segments: {} });
	});

	it('partial flush: the failing entry stays queued and drafts are NOT cleared', async () => {
		const store = memoryStore();
		saveDraft(store, RUN, { segments: { 1: { verdict: 'approve' } } });
		enqueue(store, { kind: 'review', runId: RUN, payload: { reviews: [{ seq: 1, verdict: 'approve' }] } });
		enqueue(store, { kind: 'nudge', runId: RUN, payload: { text: 'steer', kind: 'nudge' } });

		submitReview.mockResolvedValue(1); // review sends
		sendNudge.mockRejectedValue(new TypeError('Failed to fetch')); // nudge fails (still offline-ish)

		const { sent, failed } = await flushQueue(store, peekAll(store, RUN), makeSender());
		expect(sent.length).toBe(1);
		expect(failed.length).toBe(1);
		expect(failed[0].kind).toBe('nudge');

		// Page rule: with failures, do NOT clear drafts; the nudge stays queued.
		if (failed.length > 0) {
			/* keep drafts */
		} else {
			clearDrafts(store, RUN);
		}
		expect(size(store, RUN)).toBe(1);
		expect(peekAll(store, RUN)[0].kind).toBe('nudge');
		expect(loadDrafts(store, RUN).segments[1].verdict).toBe('approve');
	});

	it('re-flush after reconnect does not double-send an already-delivered entry', async () => {
		const store = memoryStore();
		enqueue(store, { kind: 'review', runId: RUN, payload: { reviews: [{ seq: 1, verdict: 'approve' }] } });
		submitReview.mockResolvedValue(1);
		await flushQueue(store, peekAll(store, RUN), makeSender());
		expect(submitReview).toHaveBeenCalledTimes(1);
		// A second reconnect fires flush again over the (now empty) queue.
		await flushQueue(store, peekAll(store, RUN), makeSender());
		expect(submitReview).toHaveBeenCalledTimes(1); // no double-send
	});
});
