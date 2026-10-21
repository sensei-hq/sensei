import { describe, it, expect } from 'vitest';
import { askReply, toRelaySession, toAsk, toSessionPlan, mapRunStatus } from './relay-inbox-map';
import type { RelayRun, RelayGate, RelaySegment } from '$lib/relay-data';

function run(over: Partial<RelayRun> = {}): RelayRun {
	return {
		id: 'db1',
		run_id: 'run-1',
		title: 'refactor refresh-token rotation',
		goal: 'rotate without logout',
		status: 'running',
		progress_done: 5,
		progress_total: 12,
		current_phase: 'Implement',
		current_feature: 'lumen-auth',
		last_event_at: '2026-07-28T12:00:00Z',
		paused_until: null,
		pause_reason: null,
		heartbeat_at: null,
		started_at: '2026-07-28T11:00:00Z',
		completed_at: null,
		...over
	};
}
const seg = (over: Partial<RelaySegment>): RelaySegment => ({
	id: 's1', parent_id: null, seq: 0, title: 'Phase 1', summary: null, detail: null,
	agent: null, model: null, spec_ref: null, state: 'done', is_gate: false,
	gate_severity: null, response_verdict: null, response_note: null, submitted_at: null, ...over
});
const gate = (over: Partial<RelayGate>): RelayGate => ({
	id: 'g1', seq: 0, run_id: 'run-1', run_title: 'refactor', segment_id: null,
	kind: 'approval', payload: {}, created_at: '2026-07-28T12:00:00Z', ...over
});

describe('relay-inbox-map', () => {
	it('mapRunStatus folds crashed into failed', () => {
		expect(mapRunStatus('crashed')).toBe('failed');
		expect(mapRunStatus('running')).toBe('running');
	});

	it('toSessionPlan nests steps under phases by parent_id, ordered by seq', () => {
		const plan = toSessionPlan([
			seg({ id: 'p1', seq: 0, title: 'Plan', state: 'done' }),
			seg({ id: 'p2', seq: 1, title: 'Implement', state: 'active' }),
			seg({ id: 't1', parent_id: 'p2', seq: 2, title: 'write code', state: 'active', model: 'claude-sonnet' })
		]);
		expect(plan.phases.map((p) => p.title)).toEqual(['Plan', 'Implement']);
		expect(plan.phases[1].tasks[0].title).toBe('write code');
		expect(plan.phases[1].tasks[0].model).toBe('claude-sonnet');
	});

	it('toAsk maps kind→action, pulls prompt/options, resolves held task title', () => {
		const a = toAsk(
			gate({ kind: 'decision', segment_id: 't1', payload: { prompt: 'Which sink?', options: ['A', 'B'] } }),
			new Map([['t1', 'Scrub logging']])
		);
		expect(a.action).toBe('choose');
		expect(a.prompt).toBe('Which sink?');
		expect(a.options).toEqual(['A', 'B']);
		expect(a.taskTitle).toBe('Scrub logging');
		expect(a.blocking).toBe(true);
	});

	it('toRelaySession rolls a run + gates + segments into the UI shape', () => {
		const s = toRelaySession(
			run(),
			[gate({ id: 'g1' }), gate({ id: 'g2' })],
			[seg({ id: 'p1', state: 'done', model: 'claude-sonnet' })],
			'organization/acme'
		);
		expect(s.id).toBe('run-1');
		expect(s.project).toBe('lumen-auth');
		expect(s.done).toBe(5);
		expect(s.total).toBe(12);
		expect(s.needs).toBe(2);
		expect(s.model).toBe('claude-sonnet');
		expect(s.asks).toHaveLength(2);
		expect(s.plan.phases).toHaveLength(1);
	});

	// #305: the daemon reads a reply fail-open — only {verdict: 'deny'} blocks — so
	// an approval must be an explicit choice, never a free line read as "allow".
	it('gives an approval gate with no options an explicit Approve / Deny', () => {
		const a = toAsk(gate({ kind: 'approval', payload: { prompt: 'Run it?' } }), new Map());
		expect(a.options).toEqual(['Approve', 'Deny']);
	});

	it('toRelaySession carries the tenant the run belongs to', () => {
		const s = toRelaySession(run(), [], [], 'organization/acme');
		expect(s.tenantKey).toBe('organization/acme');
	});
});

describe('askReply — the wire body the daemon reads', () => {
	const ask = (action: 'approve' | 'choose' | 'resume' | 'chat') =>
		({ id: 'g1', action, options: ['Approve', 'Deny'] }) as never;

	it('an approval is a verdict', () => {
		expect(askReply(ask('approve'), 'Approve')).toEqual({ verdict: 'approve' });
		expect(askReply(ask('approve'), 'Deny')).toEqual({ verdict: 'deny' });
		expect(askReply(ask('approve'), 'deny')).toEqual({ verdict: 'deny' });
	});

	it('a note travels with the answer', () => {
		expect(askReply(ask('approve'), 'Deny', ' not in prod ')).toEqual({ verdict: 'deny', note: 'not in prod' });
	});

	it('a decision is a choice; a typed answer is text', () => {
		expect(askReply({ id: 'g', action: 'choose', options: ['A', 'B'] } as never, 'B')).toEqual({ choice: 'B' });
		expect(askReply({ id: 'g', action: 'choose', options: ['A', 'B'] } as never, 'neither')).toEqual({ text: 'neither' });
	});

	it('a resume or a chat is text', () => {
		expect(askReply(ask('resume'), 'carry on')).toEqual({ text: 'carry on' });
		expect(askReply(ask('chat'), 'try the other adapter')).toEqual({ text: 'try the other adapter' });
	});
});
