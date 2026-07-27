import { describe, expect, it } from 'vitest';
import type { RelayRun, RelayGate, RelaySegment } from './relay-data';
import {
	toKitRun,
	toKitRuns,
	toKitGate,
	toKitGates,
	toKitDecision,
	toKitDecisions,
	toKitChatTurn,
	toKitChatThread,
	runState,
	inboxStatus,
	inboxRow,
	toKitInbox,
	filterInbox,
	segmentsToPlan,
	segmentsToActivity
} from './relay-map';
import type { KitPlanObject } from './components/kit/types';

// The dojo personal-Relay wire→kit mappers. The screens are presentational off
// these shapes, so the mapping is where the /v1 truth meets the kit contract —
// tested here once, deterministically (a fixed `now`).

const NOW = new Date('2026-07-23T12:00:00Z');

function run(over: Partial<RelayRun> = {}): RelayRun {
	return {
		id: 'sess-1',
		run_id: 'run-1',
		title: 'lumen-auth',
		goal: 'refactor refresh-token rotation',
		status: 'running',
		progress_done: 12,
		progress_total: 30,
		current_phase: 'Phase 2',
		current_feature: 'token-rotation',
		last_event_at: '2026-07-23T11:30:00Z',
		paused_until: null,
		pause_reason: null,
		heartbeat_at: '2026-07-23T11:59:30Z',
		started_at: '2026-07-23T11:22:00Z',
		completed_at: null,
		...over
	};
}

function gate(over: Partial<RelayGate> = {}): RelayGate {
	return {
		id: 'g1',
		seq: 1,
		run_id: 'run-1',
		run_title: 'lumen-auth',
		segment_id: null,
		kind: 'approval',
		payload: {},
		created_at: '2026-07-23T11:57:00Z',
		...over
	};
}

function seg(over: Partial<RelaySegment> = {}): RelaySegment {
	return {
		id: 's1',
		parent_id: null,
		seq: 1,
		title: 'Rotate refresh tokens',
		summary: 'route the debug line through the redaction sink',
		detail: null,
		agent: null,
		model: null,
		spec_ref: null,
		state: 'active',
		is_gate: false,
		gate_severity: null,
		response_verdict: null,
		response_note: null,
		submitted_at: '2026-07-23T11:22:00Z',
		...over
	};
}

describe('runState', () => {
	it('maps running → running and pause-family → waiting', () => {
		expect(runState('running')).toBe('running');
		expect(runState('paused')).toBe('waiting');
		expect(runState('blocked')).toBe('waiting');
		expect(runState('stalled')).toBe('waiting');
	});
	it('passes terminal states through by name', () => {
		expect(runState('done')).toBe('done');
		expect(runState('failed')).toBe('failed');
		expect(runState('crashed')).toBe('crashed');
	});
});

describe('toKitRun / toKitRuns', () => {
	it('maps the run onto the kit row shape', () => {
		const k = toKitRun(run(), new Set(), NOW);
		expect(k.id).toBe('run-1');
		expect(k.project).toBe('token-rotation'); // current_feature preferred
		expect(k.task).toBe('refactor refresh-token rotation'); // goal
		expect(k.state).toBe('running');
		expect(k.elapsed).toBe('38m'); // 11:22 → 12:00
		expect(k.edits).toBe(12); // progress_done
		expect(k.assistant).toBe('');
		expect(k.gate).toBe(false);
	});

	it('falls back to the title when there is no feature/goal', () => {
		const k = toKitRun(run({ current_feature: null, goal: null, title: 'ledger-core' }), new Set(), NOW);
		expect(k.project).toBe('ledger-core');
		expect(k.task).toBe('ledger-core');
	});

	it('marks a run that has a pending gate', () => {
		const k = toKitRun(run(), new Set(['run-1']), NOW);
		expect(k.gate).toBe(true);
	});

	it('toKitRuns derives the gate flag from the gate set', () => {
		const rows = toKitRuns([run({ run_id: 'run-1' }), run({ run_id: 'run-2', id: 'sess-2' })], [gate({ run_id: 'run-1' })], NOW);
		expect(rows.map((r) => r.gate)).toEqual([true, false]);
	});
});

describe('toKitGate / toKitGates', () => {
	it('maps a HARD-block gate onto the kit card, deriving high risk from the category', () => {
		// The daemon's hard-block payload (sessions.rs): prompt + tool + the matched
		// danger `category` + `reason`. A `category` marks it blocking → "high".
		const k = toKitGate(
			gate({
				payload: {
					prompt: 'Hard-block: touches auth schema. Approve Bash?',
					tool: 'Bash',
					category: 'destructive_db',
					reason: 'touches auth schema'
				}
			}),
			NOW
		);
		expect(k.why).toBe('touches auth schema'); // reason preferred over prompt
		expect(k.risk).toBe('high'); // hard-block carries a category
		expect(k.kind).toBe('approval');
		expect(k.project).toBe('lumen-auth');
		expect(k.session).toBe('run-1');
		expect(k.age).toBe('3m'); // 11:57 → 12:00
	});

	it('falls back to guarded risk for a SOFT gate (no category) + shows the prompt', () => {
		// The daemon's soft-gate payload carries only prompt + tool — no `category`,
		// so it reads as "guarded" (not blocking).
		const k = toKitGate(gate({ payload: { prompt: 'Approve Bash?', tool: 'Bash' } }), NOW);
		expect(k.risk).toBe('guarded');
		expect(k.cmd).toBe('Approve Bash?');
	});

	it('toKitGates keeps only approvals (drops decisions + chat/nudge)', () => {
		const rows = toKitGates(
			[gate({ id: 'a', kind: 'approval' }), gate({ id: 'd', kind: 'decision' }), gate({ id: 'c', kind: 'chat' })],
			NOW
		);
		expect(rows.map((r) => r.id)).toEqual(['a']);
	});
});

describe('toKitDecision / toKitDecisions', () => {
	it('maps a decision gate onto the kit decision card', () => {
		const k = toKitDecision(
			gate({
				kind: 'decision',
				payload: {
					prompt: 'adopt “verify webhook signature” as a client guard',
					options: ['adopt', 'keep as note', 'decline'],
					context: '4 sessions · confidence 0.91'
				}
			}),
			NOW
		);
		expect(k.title).toBe('adopt “verify webhook signature” as a client guard');
		expect(k.options).toEqual(['adopt', 'keep as note', 'decline']);
		expect(k.context).toBe('4 sessions · confidence 0.91');
		expect(k.age).toBe('3m');
	});

	it('renders with an empty option list when the payload has none', () => {
		const k = toKitDecision(gate({ kind: 'decision', payload: { prompt: 'sign off' } }), NOW);
		expect(k.options).toEqual([]);
		expect(k.title).toBe('sign off');
	});

	it('toKitDecisions keeps only decision-kind gates', () => {
		const rows = toKitDecisions(
			[gate({ id: 'd', kind: 'decision' }), gate({ id: 'a', kind: 'approval' })],
			NOW
		);
		expect(rows.map((r) => r.id)).toEqual(['d']);
	});
});

describe('toKitChatTurn / toKitChatThread', () => {
	it('maps a segment to a sensei turn (title — summary)', () => {
		const k = toKitChatTurn(seg(), NOW);
		expect(k.who).toBe('sensei');
		expect(k.kanji).toBe('先');
		expect(k.text).toBe('Rotate refresh tokens — route the debug line through the redaction sink');
		expect(k.when).toBe('38m');
	});

	it('renders the title alone when a segment has no summary', () => {
		const k = toKitChatTurn(seg({ summary: null }), NOW);
		expect(k.text).toBe('Rotate refresh tokens');
	});

	it('empty submitted_at yields an empty when', () => {
		expect(toKitChatTurn(seg({ submitted_at: null }), NOW).when).toBe('');
	});

	it('toKitChatThread preserves segment order', () => {
		const thread = toKitChatThread(
			[seg({ id: 's1', title: 'A', summary: null }), seg({ id: 's2', title: 'B', summary: null })],
			NOW
		);
		expect(thread.map((t) => t.text)).toEqual(['A', 'B']);
	});
});

describe('inbox mappers', () => {
	it('inboxStatus collapses paused→waiting and crashed→failed', () => {
		expect(inboxStatus('running')).toBe('running');
		expect(inboxStatus('paused')).toBe('waiting');
		expect(inboxStatus('crashed')).toBe('failed');
		expect(inboxStatus('failed')).toBe('failed');
		expect(inboxStatus('stalled')).toBe('stalled');
		expect(inboxStatus('done')).toBe('done');
	});

	it('inboxRow ranks a needs-you run first with attention=gate', () => {
		const r = inboxRow(run({ status: 'running' }), 2, NOW);
		expect(r.needs).toBe(2);
		expect(r.attention).toBe('gate');
		expect(r.rank).toBe(0);
		expect(r.run.gate).toBe(true); // KitRun marked as needing you
		expect(r.run.last).toBe('30m'); // last_event_at 11:30 → now 12:00
	});

	it('inboxRow attention + rank by status when nothing pends', () => {
		expect(inboxRow(run({ status: 'stalled' }), 0, NOW)).toMatchObject({ attention: 'stalled', rank: 1, status: 'stalled' });
		expect(inboxRow(run({ status: 'blocked' }), 0, NOW)).toMatchObject({ attention: 'blocked', rank: 1 });
		expect(inboxRow(run({ status: 'crashed' }), 0, NOW)).toMatchObject({ attention: 'failed', rank: 1, status: 'failed' });
		expect(inboxRow(run({ status: 'running' }), 0, NOW)).toMatchObject({ attention: null, rank: 2 });
		expect(inboxRow(run({ status: 'paused' }), 0, NOW)).toMatchObject({ attention: null, rank: 3, status: 'waiting' });
		expect(inboxRow(run({ status: 'done' }), 0, NOW)).toMatchObject({ attention: null, rank: 4 });
	});

	it('toKitInbox counts answerable gates as needs and sorts needs-you first', () => {
		const runs = [
			run({ run_id: 'a', status: 'running' }),
			run({ run_id: 'b', status: 'stalled' }),
			run({ run_id: 'c', status: 'running' })
		];
		const gates = [
			gate({ run_id: 'c', kind: 'approval' }), // c needs you
			gate({ run_id: 'c', kind: 'chat' }) // chat does NOT count
		];
		const rows = toKitInbox(runs, gates, NOW);
		expect(rows.map((r) => r.run.id)).toEqual(['c', 'b', 'a']); // needs(0) → stalled(1) → running(2)
		expect(rows.find((r) => r.run.id === 'c')?.needs).toBe(1); // only the approval, not the chat
	});

	it('filterInbox splits needs / running / finished / all', () => {
		const rows = toKitInbox(
			[
				run({ run_id: 'a', status: 'running' }),
				run({ run_id: 'b', status: 'done' }),
				run({ run_id: 'c', status: 'stalled' })
			],
			[gate({ run_id: 'a', kind: 'approval' })],
			NOW
		);
		expect(filterInbox(rows, 'needs').map((r) => r.run.id).sort()).toEqual(['a', 'c']); // a=gate, c=stalled
		expect(filterInbox(rows, 'running').map((r) => r.run.id)).toEqual(['a']);
		expect(filterInbox(rows, 'finished').map((r) => r.run.id)).toEqual(['b']);
		expect(filterInbox(rows, 'all')).toHaveLength(3);
	});
});

describe('segmentsToPlan / segmentsToActivity', () => {
	it('groups flat segments into phases → tasks by parent_id', () => {
		const segs = [
			seg({ id: 'p1', parent_id: null, seq: 1, title: 'Design', state: 'done' }),
			seg({ id: 't1', parent_id: 'p1', seq: 2, title: 'sketch', state: 'done', agent: 'coder', model: 'opus' }),
			seg({ id: 't2', parent_id: 'p1', seq: 3, title: 'review', state: 'active' }),
			seg({ id: 'p2', parent_id: null, seq: 4, title: 'Build', state: 'pending' })
		];
		const plan = segmentsToPlan(segs) as KitPlanObject;
		expect(plan.phases.map((p) => p.id)).toEqual(['p1', 'p2']);
		expect(plan.phases[0].tasks.map((t) => t.title)).toEqual(['sketch', 'review']);
		expect(plan.phases[0].tasks[0]).toMatchObject({ state: 'done', agent: 'coder', model: 'opus', deps: [] });
		expect(plan.phases[1].tasks).toEqual([]); // empty phase
	});

	it('surfaces an orphan (parent not a phase) as a standalone phase', () => {
		const plan = segmentsToPlan([seg({ id: 'x', parent_id: 'missing', title: 'orphan' })]) as KitPlanObject;
		expect(plan.phases.map((p) => p.id)).toEqual(['x']);
	});

	it('carries the gate flag + severity onto the task', () => {
		const plan = segmentsToPlan([
			seg({ id: 'p', parent_id: null, title: 'P' }),
			seg({ id: 'g', parent_id: 'p', title: 'gate', is_gate: true, gate_severity: 'blocking' })
		]) as KitPlanObject;
		expect(plan.phases[0].tasks[0]).toMatchObject({ is_gate: true, gate_severity: 'blocking' });
	});

	it('builds a newest-first activity feed, omitting unsubmitted segments', () => {
		const feed = segmentsToActivity(
			[
				seg({ id: 'a', title: 'first', state: 'done', submitted_at: '2026-07-23T11:00:00Z' }),
				seg({ id: 'b', title: 'later', state: 'active', submitted_at: '2026-07-23T11:50:00Z' }),
				seg({ id: 'c', title: 'not yet', submitted_at: null })
			],
			NOW
		);
		expect(feed.map((f) => f.text)).toEqual(['later', 'first']); // newest first, 'not yet' omitted
		expect(feed[0].at).toBe('10m'); // 11:50 → 12:00
		expect(feed[0].icon).toBeTruthy();
	});
});
