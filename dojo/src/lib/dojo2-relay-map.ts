// Pure wire→kit mappers for the personal Relay screens (dojo2 `/you/runs` ·
// `/you/approve` · `/you/decide` · `/you/chat`). They take the `relay-data.ts`
// wire types (RelayRun · RelayGate · RelaySegment — the SAME shapes the shipped
// `(console)` relay screens fetch) and project them onto the presentational kit
// shapes the dojo2 screens already declare (KitRun · KitGate · KitDecision ·
// KitChatTurn). Kept as a small, self-contained, side-effect-free module so the
// mapping is DRY, unit-tested once, and the screens stay dumb (data in → rows
// out). No `fetch`, no env, no `Date.now()` in the pure core — `now` is injected
// for deterministic elapsed/age.

import type { RelayRun, RelayGate, RelaySegment, RelayRunStatus } from './relay-data';
import { relativeAge } from './triage-view';
import type { KitRun, KitGate, KitDecision, KitChatTurn } from './components/kit/types';

/** The relay inbox kinds that are a sign-off DECISION (rules to adopt/promote)
 *  rather than a command APPROVAL. `/you/decide` filters gates to these. */
export const DECISION_KINDS = new Set(['decision']);

/** A gate that asks the human to APPROVE/deny a command (the `/you/approve`
 *  surface). Everything that isn't a decision/chat/nudge is treated as an
 *  approval so a novel kind still surfaces somewhere rather than vanishing. */
const CHATTY_KINDS = new Set(['chat', 'nudge']);

/** Compact elapsed/age from an ISO instant, empty-safe (null/'' → ''). Shares
 *  the console's `relativeAge` so the two shells read time the same way. */
function age(iso: string | null | undefined, now: Date): string {
	return iso ? relativeAge(iso, now) : '';
}

/** Read a string field off a gate's opaque (stripped) payload, or ''. */
function payloadStr(payload: Record<string, unknown>, key: string): string {
	const v = payload[key];
	return typeof v === 'string' ? v : '';
}

/** A run's live-vs-waiting state for the kit RunCard. `paused`/`blocked`/`stalled`
 *  read as "waiting" (the human is the blocker); everything else in flight is
 *  "running"; terminal states (done/failed/crashed) fall through to their name. */
export function runState(status: RelayRunStatus): string {
	switch (status) {
		case 'running':
			return 'running';
		case 'paused':
		case 'blocked':
		case 'stalled':
			return 'waiting';
		default:
			return status;
	}
}

/**
 * RelayRun → KitRun. The kit row wants: id, project, assistant, state, task,
 * elapsed, edits, gate?. RelayRun carries no assistant/edit-count (those live on
 * the daemon side, not the phone read), so `assistant` is left blank and `edits`
 * 0 — the card degrades gracefully. `gate` is set from the caller's gate set (a
 * pending gate targeting this run) so the "needs you" marker is live, not faked.
 *
 * @param gateRunIds the run ids that currently have a pending gate (from listGates).
 */
export function toKitRun(run: RelayRun, gateRunIds: Set<string>, now: Date = new Date()): KitRun {
	return {
		id: run.run_id,
		project: run.current_feature ?? run.title,
		assistant: '',
		state: runState(run.status),
		task: run.goal ?? run.title,
		elapsed: age(run.started_at, now),
		edits: run.progress_done,
		gate: gateRunIds.has(run.run_id)
	};
}

/** RelayRun[] → KitRun[], marking runs that have a pending gate. Pure. */
export function toKitRuns(runs: RelayRun[], gates: RelayGate[], now: Date = new Date()): KitRun[] {
	const gateRunIds = new Set(gates.map((g) => g.run_id).filter((id): id is string => !!id));
	return runs.map((r) => toKitRun(r, gateRunIds, now));
}

/**
 * RelayGate → KitGate (the `/you/approve` command card). The kit card wants:
 * project, cmd, kind, risk, why, session, age. The stripped payload carries the
 * command line under `command`/`cmd` and the human-language reason under
 * `prompt`/`reason`; `session` is the owning run.
 *
 * Risk is derived from the gate's severity. The daemon's hook-gate payload
 * (sessions.rs) distinguishes a HARD-block from a soft gate by presence, not a
 * `gate_severity` field: a hard-block carries the matched danger `category`
 * (+ `reason`), a soft gate carries neither. So a gate is blocking iff its
 * payload names a `category` → "high"; otherwise "guarded". (An earlier version
 * read `payload.gate_severity`, which the inbox payload never carries, so every
 * gate read as "guarded".)
 */
export function toKitGate(gate: RelayGate, now: Date = new Date()): KitGate {
	const cmd = payloadStr(gate.payload, 'command') || payloadStr(gate.payload, 'cmd');
	const why = payloadStr(gate.payload, 'reason') || payloadStr(gate.payload, 'prompt');
	const blocking = gate.payload?.category != null;
	return {
		id: gate.id,
		project: gate.run_title ?? '',
		cmd: cmd || payloadStr(gate.payload, 'prompt') || 'command awaiting approval',
		kind: gate.kind,
		risk: blocking ? 'high' : 'guarded',
		why,
		session: gate.run_id ?? '',
		age: age(gate.created_at, now)
	};
}

/** The gates that are command approvals (NOT decisions or chat/nudge) → KitGate[]. */
export function toKitGates(gates: RelayGate[], now: Date = new Date()): KitGate[] {
	return gates
		.filter((g) => !DECISION_KINDS.has(g.kind) && !CHATTY_KINDS.has(g.kind))
		.map((g) => toKitGate(g, now));
}

/**
 * RelayGate (decision kind) → KitDecision (the `/you/decide` sign-off card). The
 * kit card wants: project, title, options[], context, age. The payload carries
 * the rokkit form schema — `prompt` (the ask), `options` (the choices), and an
 * evidence line under `context`/`evidence`. A decision with no options still
 * renders (the screen shows the title + a default-empty option list).
 */
export function toKitDecision(gate: RelayGate, now: Date = new Date()): KitDecision {
	const options = Array.isArray(gate.payload?.options)
		? (gate.payload.options as unknown[]).filter((o): o is string => typeof o === 'string')
		: [];
	return {
		id: gate.id,
		project: gate.run_title ?? '',
		title: payloadStr(gate.payload, 'prompt') || 'Rule to sign off',
		options,
		context: payloadStr(gate.payload, 'context') || payloadStr(gate.payload, 'evidence'),
		age: age(gate.created_at, now)
	};
}

/** The gates that are DECISIONS → KitDecision[] (the `/you/decide` list). */
export function toKitDecisions(gates: RelayGate[], now: Date = new Date()): KitDecision[] {
	return gates.filter((g) => DECISION_KINDS.has(g.kind)).map((g) => toKitDecision(g, now));
}

/**
 * RelaySegment → KitChatTurn (the `/you/chat` thread). The relay outline is the
 * closest available narration of a run to the human — each segment (a Phase or
 * Step, title + summary) reads as one sensei turn describing what the run is
 * doing. The mentor voice is always `sensei` (the human's own replies flow the
 * other way, via `sendNudge`, and aren't part of the read). A segment with no
 * summary renders its title alone; `when` is the age of its submission (or '').
 */
export function toKitChatTurn(seg: RelaySegment, now: Date = new Date()): KitChatTurn {
	const text = seg.summary ? `${seg.title} — ${seg.summary}` : seg.title;
	return {
		who: 'sensei',
		kanji: '先',
		text,
		when: age(seg.submitted_at, now)
	};
}

/** RelaySegment[] → KitChatTurn[] (outline order = seq, as the API returns it). */
export function toKitChatThread(segments: RelaySegment[], now: Date = new Date()): KitChatTurn[] {
	return segments.map((s) => toKitChatTurn(s, now));
}
