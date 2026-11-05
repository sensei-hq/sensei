// One tracker board's facts, as a contributor's daemon pushes them, filed as the
// board's one copy, and the contributor's own effort on it (spec
// docs/spec/daemon/tracker-delivery-metrics.md D3, D4, A2; #352, #353).
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

/** At most this many sprints in one board's effort (as dojo.file_delivery bounds it). */
export const DELIVERY_MAX_SPRINTS = 1000;

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

/** One contributor's figures for one item (A2): numerators and counts, never a
 *  ratio, with the reason for every null. */
export interface DeliveryEffortRow {
	repo_keys: string[];
	values: Record<string, unknown>;
	reasons: Record<string, string>;
}

/** The caller's own effort on the board from one install (`source`); it replaces
 *  all that install reported for the board before (B13). */
export interface DeliveryEffort {
	source: string;
	project: DeliveryEffortRow | null;
	sprints: (DeliveryEffortRow & { external_id: string })[];
	tickets: (DeliveryEffortRow & { key: string })[];
}

export interface DeliveryBody {
	v: 1;
	tenant_id: string;
	repo_keys: string[];
	board_key: string;
	kind: string;
	board_name: string;
	facts: DeliveryFacts;
	effort: DeliveryEffort | null;
}

export type DeliveryCheck =
	| { ok: true; body: DeliveryBody }
	| { ok: false; status: 400 | 413; message: string };

const BOARD_KEY_RE = /^[0-9a-f]{64}$/;
const SPRINT_KEYS = ['external_id', 'seq', 'name', 'state', 'start_at', 'end_at', 'complete_at', 'removed', 'sdlc'];
const TICKET_KEYS = ['key', 'title', 'status_category', 'points', 'sprints', 'issue_created_at'];
const SPRINT_STATES = new Set(['future', 'active', 'closed']);

/** The values each effort row may carry (D1, A2): nothing else travels — no rate,
 *  no calibration, no linkage diagnostics (B10, B14). */
const TICKET_VALUES = new Set([
	'sessions', 'api_cost_n', 'api_cost_partial', 'worked_ms', 'prompts', 'ai_token_cost',
	'compile_defects', 'test_defects', 'review_fixes', 'escaped_task_defects', 'build_ok', 'build_fail',
	'runs_unknown', 'unclassified_failures', 'build_unrecovered', 'recovery_ms_sum', 'recovery_n',
	'on_hold_ms', 'on_hold_count', 'on_hold_by_reason', 'execution_ms', 'rework_ms', 'rework_cycles',
	'effort_cost', 'rework_cost', 'work_started_at', 'submitted_at', 'submitted_via'
]);
const SPRINT_VALUES = new Set([
	'sessions', 'shared', 'by_source', 'active_secs', 'active_n', 'tokens_fresh', 'tokens_fresh_n',
	'cache_read', 'cache_read_n', 'cache_write', 'cache_write_n', 'tokens_out', 'tokens_out_n',
	'tokens_reasoning', 'tokens_reasoning_n', 'metered_cost', 'metered_cost_n', 'claude_secs', 'claude_n',
	'user_secs', 'user_n', 'wall_secs', 'wall_n', 'first_at', 'last_at', 'api_cost', 'api_cost_n',
	'api_cost_partial'
]);
const PROJECT_VALUES = new Set(['sessions', 'tokens', 'sessions_without_tokens', 'prompts']);
const ROW_KEYS = ['repo_keys', 'values', 'reasons'];

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

const TIMES = new Set(['work_started_at', 'submitted_at', 'first_at', 'last_at']);
const MAPS = new Set(['by_source', 'on_hold_by_reason']);
const finite = (n: unknown) => typeof n === 'number' && Number.isFinite(n);

/** Each value of its own type, or null (as dojo.delivery_effort_row_ok checks it):
 *  a time a timestamp, submitted_via pr or push, a map of numbers, else a number. */
function valueOk(key: string, x: unknown): boolean {
	if (x === null) return true;
	if (TIMES.has(key)) return typeof x === 'string' && !Number.isNaN(Date.parse(x));
	if (key === 'submitted_via') return x === 'pr' || x === 'push';
	if (MAPS.has(key)) return isObject(x) && Object.values(x).every(finite);
	return finite(x);
}

/** One effort row: exactly its keys, values among `allowed`, repositories among the push's. */
function rowOk(r: unknown, allowed: Set<string>, id: string | null, pushed: Set<string>): boolean {
	if (!isObject(r) || !hasExactly(r, id ? [...ROW_KEYS, id] : ROW_KEYS)) return false;
	if (id && !text(r[id])) return false;
	const { repo_keys, values, reasons } = r;
	return (
		Array.isArray(repo_keys) &&
		repo_keys.length > 0 &&
		repo_keys.every((k) => text(k) && pushed.has(k as string)) &&
		isObject(values) &&
		// At least one session of the contributor's own on the item, or no row (D1).
		typeof values.sessions === 'number' &&
		values.sessions >= 1 &&
		Object.entries(values).every(([k, v]) => allowed.has(k) && valueOk(k, v)) &&
		isObject(reasons) &&
		Object.values(reasons).every(text)
	);
}

/** Check the effort part (A2); a reason for refusing it, or null. */
function effortProblem(e: unknown, pushed: Set<string>): { status: 400 | 413; message: string } | null {
	const bad = (message: string) => ({ status: 400 as const, message });
	if (!isObject(e) || !hasExactly(e, ['source', 'project', 'sprints', 'tickets'])) {
		return bad('effort carries exactly: source, project, sprints, tickets');
	}
	if (!isUuid(e.source)) return bad('effort.source names the install as a uuid');
	if (!Array.isArray(e.sprints) || !Array.isArray(e.tickets)) return bad('effort.sprints and effort.tickets are lists');
	if (e.sprints.length > DELIVERY_MAX_SPRINTS) {
		return { status: 413, message: `at most ${DELIVERY_MAX_SPRINTS} effort sprints per board, got ${e.sprints.length}` };
	}
	if (e.tickets.length > DELIVERY_MAX_TICKETS) {
		return { status: 413, message: `at most ${DELIVERY_MAX_TICKETS} effort tickets per board, got ${e.tickets.length}` };
	}
	if (e.project !== null && !rowOk(e.project, PROJECT_VALUES, null, pushed)) {
		return bad('effort.project is a row of the project fields over the pushed repositories');
	}
	if (!e.sprints.every((r) => rowOk(r, SPRINT_VALUES, 'external_id', pushed))) {
		return bad('each effort sprint is a row of usage fields over the pushed repositories');
	}
	if (!e.tickets.every((r) => rowOk(r, TICKET_VALUES, 'key', pushed))) {
		return bad('each effort ticket is a row of the D1 wire fields over the pushed repositories');
	}
	const unique = (ids: unknown[]) => new Set(ids).size === ids.length;
	if (!unique(e.sprints.map((r) => (r as Record<string, unknown>).external_id)) || !unique(e.tickets.map((r) => (r as Record<string, unknown>).key))) {
		return bad('an effort item appears once');
	}
	return null;
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
	const effort = raw.effort === undefined ? null : raw.effort;
	if (effort !== null) {
		const problem = effortProblem(effort, new Set(raw.repo_keys as string[]));
		if (problem) return { ok: false, ...problem };
	}
	return {
		ok: true,
		body: {
			v: 1,
			tenant_id: raw.tenant_id,
			repo_keys: raw.repo_keys as string[],
			board_key: raw.board_key,
			kind: raw.kind as string,
			board_name: raw.board_name,
			facts: f as unknown as DeliveryFacts,
			effort: effort as DeliveryEffort | null
		}
	};
}

/** What the dōjō answers for a push: the facts' fate, and the effort's when sent. */
export interface DeliveryOutcome {
	facts: 'accepted' | 'stale';
	effort?: 'accepted';
}

/** File a checked board as the caller, both parts in one transaction
 *  (dojo.file_delivery). A refusal keeps its status (403, 404, 409); an answer
 *  that is not the outcome of what was sent is a failure. */
export async function fileDelivery(db: DojoClient, body: DeliveryBody): Promise<DeliveryOutcome> {
	const out = await callFn<unknown>(db, 'file_delivery', {
		p_tenant: body.tenant_id,
		p_repo_keys: body.repo_keys,
		p_board: { board_key: body.board_key, kind: body.kind, board_name: body.board_name, facts: body.facts },
		p_effort: body.effort
	});
	const answer = isObject(out) ? out : null;
	const facts = answer?.facts;
	const effortOk = body.effort === null ? answer?.effort === undefined : answer?.effort === 'accepted';
	if ((facts !== 'accepted' && facts !== 'stale') || !effortOk) {
		throw new AdminError(500, `file_delivery answered ${JSON.stringify(out)}`);
	}
	return body.effort === null ? { facts } : { facts, effort: 'accepted' };
}
