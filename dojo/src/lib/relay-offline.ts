// Relay P4.5 — offline / reconnect draft + action-queue core (PURE, unit-tested).
//
// While the phone/console is offline the human should never be blocked: they can
// still read the last-loaded outline, draft per-segment review verdicts + notes,
// and compose a nudge/reply. All of that is held locally here; on reconnect a
// Send-batch flushes the queued mutations through the real relay-data senders.
//
// Everything in this module is PURE over an INJECTABLE storage — a tiny
// {getItem,setItem,removeItem} interface — so the unit tests use an in-memory
// fake and never touch real localStorage. The .svelte page adapts the browser's
// localStorage into this interface (guarded for SSR) and injects it. Nothing here
// reaches for `window`/`localStorage` directly, so it imports cleanly under SSR
// and tests deterministically.
//
// Durability model: the server rows (dojo.relay_inbox / relay_segments) are the
// source of truth. This store is a local convenience buffer — drafts survive a
// close/reopen, and the queue survives so a transient connectivity drop can't
// lose in-progress guidance. Idempotency is by a stable per-entry id so a
// double-flush can never double-send.

import type { SegmentReview } from './relay-data';

// ── Injectable storage ───────────────────────────────────────────────────────

/** The minimal storage surface this module needs — a subset of the Web Storage
 *  API so a browser `localStorage` satisfies it directly, and an in-memory fake
 *  satisfies it in tests. Kept synchronous to match Web Storage. */
export interface KeyValueStore {
	getItem(key: string): string | null;
	setItem(key: string, value: string): void;
	removeItem(key: string): void;
}

/** An in-memory {@link KeyValueStore} — the test fake, and the SSR/no-storage
 *  fallback so callers never have to null-check the store. */
export function memoryStore(seed: Record<string, string> = {}): KeyValueStore {
	const map = new Map<string, string>(Object.entries(seed));
	return {
		getItem: (k) => (map.has(k) ? (map.get(k) as string) : null),
		setItem: (k, v) => void map.set(k, v),
		removeItem: (k) => void map.delete(k)
	};
}

// ── Key namespacing ──────────────────────────────────────────────────────────

// Version prefix so a future shape change can migrate/ignore old keys rather than
// mis-parsing them. Bump when the persisted JSON shape changes incompatibly.
const VERSION = 'v1';
const NS = `relay-offline:${VERSION}`;

/** Draft key for one run (holds every segment draft + the nudge text for it). */
function draftKey(runId: string): string {
	return `${NS}:draft:${runId}`;
}

/** The single queue key — one durable FIFO across all runs (peek filters by run). */
const QUEUE_KEY = `${NS}:queue`;

// ── Draft store ──────────────────────────────────────────────────────────────

/** One segment's drafted verdict + note, keyed by seq (mirrors the page's Draft). */
export interface SegmentDraft {
	verdict?: SegmentReview['verdict'];
	note?: string;
}

/** The per-run draft bundle: every in-progress segment review + the nudge text.
 *  Persisted as one JSON blob per run so a close/reopen restores it whole. */
export interface RunDraft {
	/** Per-segment review drafts, keyed by segment seq. */
	segments: Record<number, SegmentDraft>;
	/** The in-progress nudge composer text. */
	nudge?: string;
}

const EMPTY_DRAFT: RunDraft = { segments: {} };

/**
 * Load a run's drafts. Tolerant of absent/corrupt storage: any parse failure or
 * unexpected shape yields a fresh empty draft rather than throwing — a garbled
 * key must never break the page.
 */
export function loadDrafts(store: KeyValueStore, runId: string): RunDraft {
	let raw: string | null = null;
	try {
		raw = store.getItem(draftKey(runId));
	} catch {
		return { segments: {} };
	}
	if (!raw) return { segments: {} };
	try {
		const parsed = JSON.parse(raw) as unknown;
		if (!parsed || typeof parsed !== 'object') return { segments: {} };
		const obj = parsed as Partial<RunDraft>;
		const segments =
			obj.segments && typeof obj.segments === 'object' ? (obj.segments as Record<number, SegmentDraft>) : {};
		const nudge = typeof obj.nudge === 'string' ? obj.nudge : undefined;
		return { segments, ...(nudge !== undefined ? { nudge } : {}) };
	} catch {
		return { segments: {} };
	}
}

/**
 * Persist a run's drafts (whole-bundle write). A write failure (quota, private
 * mode) is swallowed — losing a draft persist is a soft failure, never fatal to
 * the page, and the in-memory state is still the live source for the session.
 */
export function saveDraft(store: KeyValueStore, runId: string, draft: RunDraft): void {
	try {
		store.setItem(draftKey(runId), JSON.stringify(draft));
	} catch {
		// swallow — persistence is best-effort; the live UI state is authoritative.
	}
}

/** Clear a run's drafts (after a successful send/flush for that run). */
export function clearDrafts(store: KeyValueStore, runId: string): void {
	try {
		store.removeItem(draftKey(runId));
	} catch {
		// swallow — a failed clear is harmless; a stale draft is re-cleared next time.
	}
}

// ── Action queue ─────────────────────────────────────────────────────────────

/** The kind of mutation a queue entry replays on flush. */
export type QueuedActionKind = 'review' | 'nudge' | 'reply';

/** The review-batch payload — the SegmentReview[] submitReview takes. */
export interface ReviewPayload {
	reviews: SegmentReview[];
}

/** The nudge payload — the text + kind sendNudge takes. */
export interface NudgePayload {
	text: string;
	kind?: 'nudge' | 'chat';
}

/** The gate-reply payload — the inbox id + the reply body replyToGate takes. */
export interface ReplyPayload {
	inboxId: string;
	reply: unknown;
}

/** A discriminated union so the flush sender can dispatch on `kind` type-safely. */
export type QueuedAction =
	| { id: string; kind: 'review'; runId: string; payload: ReviewPayload; queuedAt: string }
	| { id: string; kind: 'nudge'; runId: string; payload: NudgePayload; queuedAt: string }
	| { id: string; kind: 'reply'; runId: string; payload: ReplyPayload; queuedAt: string };

/** Read the whole queue, tolerating absent/corrupt storage (→ empty). */
function readQueue(store: KeyValueStore): QueuedAction[] {
	let raw: string | null = null;
	try {
		raw = store.getItem(QUEUE_KEY);
	} catch {
		return [];
	}
	if (!raw) return [];
	try {
		const parsed = JSON.parse(raw) as unknown;
		return Array.isArray(parsed) ? (parsed as QueuedAction[]) : [];
	} catch {
		return [];
	}
}

/** Overwrite the whole queue (best-effort persist). */
function writeQueue(store: KeyValueStore, entries: QueuedAction[]): void {
	try {
		store.setItem(QUEUE_KEY, JSON.stringify(entries));
	} catch {
		// swallow — a dropped persist can't be recovered here; the caller's in-memory
		// state is still correct for this session.
	}
}

/** A stable id generator for a new entry. `crypto.randomUUID` when present
 *  (browser + Workers), else a time+random fallback so ids stay unique. */
export function newActionId(): string {
	const c = (globalThis as { crypto?: { randomUUID?: () => string } }).crypto;
	if (c?.randomUUID) return c.randomUUID();
	return `act-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

/** The fields a caller supplies; `id`/`queuedAt` are stamped by {@link enqueue}. */
export type NewAction =
	| { kind: 'review'; runId: string; payload: ReviewPayload }
	| { kind: 'nudge'; runId: string; payload: NudgePayload }
	| { kind: 'reply'; runId: string; payload: ReplyPayload };

/**
 * Append a mutation to the durable FIFO. Stamps a stable id + queuedAt so a
 * re-flush is idempotent (dedup by id) and the queue reads oldest-first. Returns
 * the stored entry so the caller can show/track it.
 */
export function enqueue(store: KeyValueStore, action: NewAction, now: () => Date = () => new Date()): QueuedAction {
	const entry = {
		id: newActionId(),
		queuedAt: now().toISOString(),
		...action
	} as QueuedAction;
	const entries = readQueue(store);
	entries.push(entry);
	writeQueue(store, entries);
	return entry;
}

/** All queued entries, oldest first. Pass a runId to scope to one run. */
export function peekAll(store: KeyValueStore, runId?: string): QueuedAction[] {
	const entries = readQueue(store);
	return runId ? entries.filter((e) => e.runId === runId) : entries;
}

/** Remove one entry by id (after it flushed successfully). No-op if absent. */
export function remove(store: KeyValueStore, id: string): void {
	const entries = readQueue(store);
	const next = entries.filter((e) => e.id !== id);
	if (next.length !== entries.length) writeQueue(store, next);
}

/** How many entries are queued (optionally scoped to one run). */
export function size(store: KeyValueStore, runId?: string): number {
	return peekAll(store, runId).length;
}

// ── Flush ────────────────────────────────────────────────────────────────────

/** The per-entry send result. `ok` removes the entry; anything else keeps it. */
export type SendResult = { ok: true } | { ok: false; error: string };

/** Sends one queued action to the backend. Injected so flush is pure/testable. */
export type ActionSender = (entry: QueuedAction) => Promise<SendResult>;

/** The outcome of a flush pass: what got delivered vs what stayed queued. */
export interface FlushOutcome {
	/** Ids delivered + removed from the queue this pass. */
	sent: string[];
	/** Entries that failed and remain queued (for a retry / inline note). */
	failed: QueuedAction[];
}

/**
 * Flush a set of queued entries through `sender`, one at a time, in order.
 *
 * Partial-failure safe: each success is removed via {@link remove} (durable),
 * each failure is KEPT so a transient drop never loses guidance. A double-flush
 * can't double-send — an already-removed id simply isn't in the queue, and each
 * pass only removes on a genuine `ok`. Returns which ids were sent and which
 * entries remain queued.
 *
 * @param store    the durable store (successes are removed from it).
 * @param entries  the snapshot to attempt (typically `peekAll(store)`).
 * @param sender   delivers one entry; resolves ok|error, never rejects for a
 *                 normal failure (a thrown/rejected sender is caught → kept).
 */
export async function flushQueue(
	store: KeyValueStore,
	entries: QueuedAction[],
	sender: ActionSender
): Promise<FlushOutcome> {
	const sent: string[] = [];
	const failed: QueuedAction[] = [];
	for (const entry of entries) {
		let result: SendResult;
		try {
			result = await sender(entry);
		} catch (e) {
			result = { ok: false, error: e instanceof Error ? e.message : 'send failed' };
		}
		if (result.ok) {
			remove(store, entry.id);
			sent.push(entry.id);
		} else {
			failed.push(entry);
		}
	}
	return { sent, failed };
}
