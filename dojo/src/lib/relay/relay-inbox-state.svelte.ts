// Inbox State (layer 2 of ui-state-pattern) — the single source of truth for the
// relay inbox. Owns the session list + filter + selection; every change is a named
// method (the component never mutates directly). Pure derivations live in
// relay-inbox-view.ts. Realtime lives here (`patch`/`remove`), not in components,
// so the UI is a pure function of this state.
//
// Load client-side (onMount) — a module-level rune singleton must not be populated
// during SSR (it would leak across requests). The mock/real Load feeds `load()`.
import type { RelaySession, RelayFilter } from './types';
import type { InboxLoad } from './relay-inbox';
import * as m from '$lib/paraglide/messages';
import { filterSessions, needsCount } from './relay-inbox-view';
import { askReply } from './relay-inbox-map';

/** Deliver a reply to a gate in a dōjō (the page binds `replyToGate` to its session). */
export type SendReply = (tenantKey: string, inboxId: string, reply: Record<string, unknown>) => Promise<void>;

let sessions = $state<RelaySession[]>([]);
let filter = $state<RelayFilter>('needs');
let selectedId = $state<string | null>(null);
// Per-ask answers (askId → the answer text), recorded only once the reply has
// reached the dōjō (#305) — never as a local echo of an unsent reply.
let answered = $state<Record<string, string>>({});
// Why an ask's reply did not land, and which asks are sending now.
let askErrors = $state<Record<string, string>>({});
let sending = $state<Record<string, boolean>>({});
// The load's outcome (#104). `ready` with `unreachable`/`plansMissing` is a partial
// inbox; `error` is an inbox that could not be read at all — never shown as empty.
let status = $state<'loading' | 'ready' | 'error'>('ready');
let error = $state<string | null>(null);
let unreachable = $state<string[]>([]);
let plansMissing = $state(0);

const shown = $derived(filterSessions(sessions, filter));
const needs = $derived(needsCount(sessions));
const selected = $derived(sessions.find((s) => s.id === selectedId) ?? null);

export const relayInboxState = {
	get sessions() {
		return sessions;
	},
	/** Filtered + ranked list for the rail. */
	get shown() {
		return shown;
	},
	/** Total open asks — the "{n} need you" count. */
	get needsCount() {
		return needs;
	},
	get filter() {
		return filter;
	},
	get selectedId() {
		return selectedId;
	},
	/** The open session, or null. */
	get selected() {
		return selected;
	},
	/** Answered asks (askId → verdict text) — drives the AskCard verdict echo. */
	get answered() {
		return answered;
	},

	get status() {
		return status;
	},
	/** Why the inbox could not be read, when `status` is `error`. */
	get error() {
		return error;
	},
	/** Dōjōs whose relay did not answer — their runs are missing from the list. */
	get unreachable() {
		return unreachable;
	},
	/** Runs shown without their plan. */
	get plansMissing() {
		return plansMissing;
	},

	load(next: RelaySession[]) {
		sessions = next;
		status = 'ready';
		error = null;
		// A fresh read is the dōjō's truth: an answered gate leaves its pending list,
		// and one still listed is still open.
		answered = {};
		askErrors = {};
	},
	/** The load started; the rail shows its skeleton. */
	begin() {
		status = 'loading';
		error = null;
	},
	/** The fan-out's result. Every dōjō unreachable is an error, not an empty inbox. */
	loaded(r: InboxLoad) {
		sessions = r.sessions;
		unreachable = r.unreachable;
		plansMissing = r.plansMissing;
		const allDown = r.tenants > 0 && r.unreachable.length === r.tenants;
		status = allDown ? 'error' : 'ready';
		error = !allDown
			? null
			: r.tenants === 1
				? m.inbox_error_one_down()
				: m.inbox_error_all_down({ count: r.tenants });
	},
	/** The load itself threw. */
	fail(message: string) {
		status = 'error';
		error = message;
	},
	/** Record a reply to an ask — the mock echoes it locally; the real POST swaps in here. */
	/** Why the reply to an ask did not land, or null. */
	askError(askId: string): string | null {
		return askErrors[askId] ?? null;
	},
	isSending(askId: string): boolean {
		return sending[askId] === true;
	},
	/** Send an answer to the dōjō `run` lives in, as the daemon reads it
	 *  (`askReply`) — for an inbox run or one opened by a /you/runs link alike. The
	 *  answer is recorded only once the reply lands; otherwise the ask says why and
	 *  stays open. Resolves whether it landed. */
	async answerAsk(send: SendReply, run: RelaySession, askId: string, answer: string, note?: string): Promise<boolean> {
		const ask = run.asks.find((a) => a.id === askId);
		if (!ask) {
			askErrors = { ...askErrors, [askId]: 'this ask is no longer on the run' };
			return false;
		}
		sending = { ...sending, [askId]: true };
		const { [askId]: _cleared, ...rest } = askErrors;
		askErrors = rest;
		try {
			await send(run.tenantKey, askId, askReply(ask, answer, note));
			answered = { ...answered, [askId]: answer };
			return true;
		} catch (e) {
			askErrors = { ...askErrors, [askId]: e instanceof Error && e.message ? e.message : 'could not send your answer' };
			return false;
		} finally {
			const { [askId]: _done, ...still } = sending;
			sending = still;
		}
	},
	select(id: string | null) {
		selectedId = id;
	},
	setFilter(f: RelayFilter) {
		filter = f;
	},
	/** Realtime upsert — replace an existing run in place, else append. */
	patch(s: RelaySession) {
		const i = sessions.findIndex((x) => x.id === s.id);
		if (i >= 0) sessions[i] = s;
		else sessions = [...sessions, s];
	},
	/** Realtime removal — a run left the inbox (finished + aged out). */
	remove(id: string) {
		sessions = sessions.filter((x) => x.id !== id);
		if (selectedId === id) selectedId = null;
	}
};
