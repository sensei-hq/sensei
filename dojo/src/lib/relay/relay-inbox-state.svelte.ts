// Inbox State (layer 2 of ui-state-pattern) — the single source of truth for the
// relay inbox. Owns the session list + filter + selection; every change is a named
// method (the component never mutates directly). Pure derivations live in
// relay-inbox-view.ts. Realtime lives here (`patch`/`remove`), not in components,
// so the UI is a pure function of this state.
//
// Load client-side (onMount) — a module-level rune singleton must not be populated
// during SSR (it would leak across requests). The mock/real Load feeds `load()`.
import type { RelaySession, RelayFilter } from './types';
import { filterSessions, needsCount } from './relay-inbox-view';

let sessions = $state<RelaySession[]>([]);
let filter = $state<RelayFilter>('needs');
let selectedId = $state<string | null>(null);
// Per-ask verdicts (askId → the answer text). Mock-phase local echo of the reply;
// the real write path (POST /relay/reply) swaps in behind answerAsk later.
let answered = $state<Record<string, string>>({});

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

	load(next: RelaySession[]) {
		sessions = next;
	},
	/** Record a reply to an ask — the mock echoes it locally; the real POST swaps in here. */
	answerAsk(askId: string, verdict: string) {
		answered = { ...answered, [askId]: verdict };
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
