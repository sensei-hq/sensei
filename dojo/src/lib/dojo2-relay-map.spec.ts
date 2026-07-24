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
	runState
} from './dojo2-relay-map';

// The dojo2 personal-Relay wire→kit mappers. The screens are presentational off
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
