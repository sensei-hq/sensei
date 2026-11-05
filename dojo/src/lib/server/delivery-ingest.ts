// One tracker board's facts, as a contributor's daemon pushes them, filed as the
// board's one copy (spec docs/spec/daemon/tracker-delivery-metrics.md D3, D4; #352).
//
// The shape is checked here, exactly: a ticket or sprint carrying any other key
// is refused, so nothing but the title ever travels as ticket free text (B9) — no
// description, assignee or URL can ride along by accident. Consent, the
// workspace and which copy wins are the database's (dojo.file_delivery_facts),
// decided as the caller; its refusals keep their status (db-refusal).
import { AdminError, type DojoClient } from './admin-data';
import type { DeliveryTicketWire } from '$lib/delivery-wire';
import { isUuid } from './ids';
import { callFn } from './tenant-rows';

/** At most this many tickets in one board (D3). */
export const DELIVERY_MAX_TICKETS = 5000;

/** At most this many bytes in one request, checked before the parse (D3). */
export const DELIVERY_MAX_BYTES = 4 * 1024 * 1024;

export interface DeliverySprint {
	external_id: string;
	seq: number;
	name: string;
	state: 'future' | 'active' | 'closed';
	start_at: string | null;
	end_at: string | null;
	complete_at: string | null;
	removed: boolean;
	/** The sprint's SprintSdlc as the daemon computed it; null when it has no replay. */
	sdlc: Record<string, unknown> | null;
}

/** One non-removed ticket as pushed: the wire's own shape. */
export type DeliveryTicket = DeliveryTicketWire;

export interface DeliveryFacts {
	history_at: string | null;
	pending_tickets: number;
	predictability: number | null;
	predictability_reason: string | null;
	sprints: DeliverySprint[];
	tickets: DeliveryTicket[];
}

export interface DeliveryBody {
	v: 1;
	tenant_id: string;
	repo_keys: string[];
	board_key: string;
	kind: string;
	board_name: string;
	facts: DeliveryFacts;
}

export type DeliveryCheck =
	| { ok: true; body: DeliveryBody }
	| { ok: false; status: 400 | 413; message: string };

const BOARD_KEY_RE = /^[0-9a-f]{64}$/;
const SPRINT_KEYS = ['external_id', 'seq', 'name', 'state', 'start_at', 'end_at', 'complete_at', 'removed', 'sdlc'];
const TICKET_KEYS = ['key', 'title', 'status_category', 'points', 'sprints', 'issue_created_at'];
const SPRINT_STATES = new Set(['future', 'active', 'closed']);

const isObject = (x: unknown): x is Record<string, unknown> => typeof x === 'object' && x !== null && !Array.isArray(x);
const text = (x: unknown) => typeof x === 'string' && x.length > 0;
const textOrNull = (x: unknown) => x === null || typeof x === 'string';
const numberOrNull = (x: unknown) => x === null || (typeof x === 'number' && Number.isFinite(x));

/** Exactly these keys, no more and no fewer. */
function hasExactly(o: Record<string, unknown>, keys: string[]): boolean {
	const own = Object.keys(o);
	return own.length === keys.length && keys.every((k) => k in o);
}

function sprintOk(s: unknown): boolean {
	return (
		isObject(s) &&
		hasExactly(s, SPRINT_KEYS) &&
		text(s.external_id) &&
		Number.isInteger(s.seq) &&
		typeof s.name === 'string' &&
		SPRINT_STATES.has(s.state as string) &&
		textOrNull(s.start_at) &&
		textOrNull(s.end_at) &&
		textOrNull(s.complete_at) &&
		typeof s.removed === 'boolean' &&
		(s.sdlc === null || isObject(s.sdlc))
	);
}

function ticketOk(t: unknown): boolean {
	return (
		isObject(t) &&
		hasExactly(t, TICKET_KEYS) &&
		text(t.key) &&
		textOrNull(t.title) &&
		textOrNull(t.status_category) &&
		numberOrNull(t.points) &&
		Array.isArray(t.sprints) &&
		t.sprints.every(text) &&
		textOrNull(t.issue_created_at)
	);
}

/** Check a pushed body against the D3 shape. Pure. */
export function checkDeliveryBody(raw: unknown): DeliveryCheck {
	const bad = (message: string): DeliveryCheck => ({ ok: false, status: 400, message });
	if (!isObject(raw)) return bad('body must be a board object');
	if (raw.v !== 1) return bad('unknown version: v must be 1');
	if (!isUuid(raw.tenant_id)) return bad('tenant_id must be a uuid');
	if (!Array.isArray(raw.repo_keys) || raw.repo_keys.length === 0 || !raw.repo_keys.every(text)) {
		return bad('repo_keys must name at least one repository');
	}
	if (typeof raw.board_key !== 'string' || !BOARD_KEY_RE.test(raw.board_key)) {
		return bad('board_key must be 64 lowercase hex characters');
	}
	if (!text(raw.kind)) return bad('kind is required');
	if (typeof raw.board_name !== 'string') return bad('board_name is required');
	// The effort part arrives with F3; until then a body carrying one is refused
	// rather than half-filed.
	if (raw.effort !== undefined && raw.effort !== null) return bad('an effort part is not accepted yet');
	const f = raw.facts;
	if (!isObject(f)) return bad('facts is required');
	if (!textOrNull(f.history_at)) return bad('facts.history_at must be a timestamp or null');
	if (!Number.isInteger(f.pending_tickets) || (f.pending_tickets as number) < 0) {
		return bad('facts.pending_tickets must be a whole number, 0 or more');
	}
	if (!numberOrNull(f.predictability) || !textOrNull(f.predictability_reason)) {
		return bad('facts.predictability and its reason must be a number and a code, or null');
	}
	if (!Array.isArray(f.sprints) || !Array.isArray(f.tickets)) return bad('facts.sprints and facts.tickets are lists');
	if (f.tickets.length > DELIVERY_MAX_TICKETS) {
		return { ok: false, status: 413, message: `at most ${DELIVERY_MAX_TICKETS} tickets per board, got ${f.tickets.length}` };
	}
	if (!f.sprints.every(sprintOk)) return bad(`each sprint carries exactly: ${SPRINT_KEYS.join(', ')}`);
	if (!f.tickets.every(ticketOk)) return bad(`each ticket carries exactly: ${TICKET_KEYS.join(', ')}`);
	return {
		ok: true,
		body: {
			v: 1,
			tenant_id: raw.tenant_id,
			repo_keys: raw.repo_keys as string[],
			board_key: raw.board_key,
			kind: raw.kind as string,
			board_name: raw.board_name,
			facts: f as unknown as DeliveryFacts
		}
	};
}

/** File a checked board as the caller. A refusal keeps its status (403, 404,
 *  409); an answer that is neither `accepted` nor `stale` is a failure. */
export async function fileDelivery(db: DojoClient, body: DeliveryBody): Promise<{ facts: 'accepted' | 'stale' }> {
	const out = await callFn<unknown>(db, 'file_delivery_facts', {
		p_tenant: body.tenant_id,
		p_repo_keys: body.repo_keys,
		p_board: { board_key: body.board_key, kind: body.kind, board_name: body.board_name, facts: body.facts }
	});
	if (out !== 'accepted' && out !== 'stale') {
		throw new AdminError(500, `file_delivery_facts answered ${JSON.stringify(out)}`);
	}
	return { facts: out };
}
