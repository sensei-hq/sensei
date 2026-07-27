import { describe, expect, it } from 'vitest';
import { phases, tasks, stageState, planProgress, runFlag } from './plan';
import type { KitPlan, KitRun } from './types';

// A two-phase authored plan: a parallel phase (done + active) then a sequential
// phase (a gate waiting).
const PLAN: KitPlan = {
	goal: 'ship it',
	phases: [
		{
			id: 'design',
			title: 'Design',
			tasks: [
				{ id: 'a', title: 'sketch', state: 'done' },
				{ id: 'b', title: 'review', state: 'active' }
			]
		},
		{
			id: 'build',
			title: 'Build',
			tasks: [
				{ id: 'c', title: 'gate', state: 'needs_review', is_gate: true, deps: ['b'] }
			]
		}
	]
};

const run = (over: Partial<KitRun> = {}): KitRun => ({
	id: 'r1',
	project: 'p',
	assistant: '',
	state: 'running',
	task: 't',
	elapsed: '1m',
	edits: 0,
	...over
});

describe('kit plan normalizers', () => {
	it('phases normalizes an authored plan and defaults deps', () => {
		const ps = phases(PLAN);
		expect(ps.map((p) => p.id)).toEqual(['design', 'build']);
		expect(ps[0].tasks[0].deps).toEqual([]); // missing deps → []
		expect(tasks(PLAN)).toHaveLength(3);
	});

	it('phases accepts the legacy array shape and aliases legacy states', () => {
		const legacy = [{ id: 'x', title: 'X', tasks: [{ id: 't', title: 'go', state: 'running' }] }] as unknown as KitPlan;
		expect(phases(legacy)[0].tasks[0].state).toBe('active'); // running → active
	});

	it('phases tolerates null/undefined', () => {
		expect(phases(null)).toEqual([]);
		expect(tasks(undefined)).toEqual([]);
	});

	it('stageState returns the most-urgent live state, else done', () => {
		expect(stageState(phases(PLAN)[0])).toBe('active'); // done + active → active
		expect(stageState(phases(PLAN)[1])).toBe('needs_review');
		const allDone = { id: 'p', title: 'P', tasks: [{ id: 'a', title: 'a', state: 'done' as const }] };
		expect(stageState(allDone)).toBe('done');
		const withFail = {
			id: 'p',
			title: 'P',
			tasks: [
				{ id: 'a', title: 'a', state: 'done' as const },
				{ id: 'b', title: 'b', state: 'failed' as const }
			]
		};
		expect(stageState(withFail)).toBe('failed'); // failed outranks done
	});

	it('planProgress reports done/total, current stage, and half-counts active', () => {
		const pr = planProgress(PLAN);
		expect(pr.total).toBe(3);
		expect(pr.done).toBe(1); // one done task
		expect(pr.running).toBe(1);
		expect(pr.stage).toBe(1); // first not-done phase is Design (active)
		expect(pr.stages).toBe(2);
		expect(pr.stageName).toBe('Design');
		expect(pr.pct).toBe(Math.round(((1 + 0.5) / 3) * 100)); // 50
	});

	it('planProgress on an empty plan is safe (total 1, 0%)', () => {
		const pr = planProgress([]);
		expect(pr).toMatchObject({ done: 0, total: 1, pct: 0, stages: 0 });
	});

	it('runFlag prioritizes a pending gate → Needs approval', () => {
		const f = runFlag(run({ plan: PLAN }));
		expect(f.key).toBe('gate');
		expect(f.label).toBe('Needs approval');
		expect(f.cta).toBe('Approve');
		expect(f.act).toBe(true);
		expect(f.fill).toBe('bg-accent');
	});

	it('runFlag falls through failed → blocked → running → waiting', () => {
		const failed = run({ plan: { phases: [{ id: 'p', title: 'P', tasks: [{ id: 'a', title: 'a', state: 'failed' }] }] } });
		expect(runFlag(failed).key).toBe('failed');
		const blocked = run({ plan: { phases: [{ id: 'p', title: 'P', tasks: [{ id: 'a', title: 'a', state: 'blocked' }] }] } });
		expect(runFlag(blocked).key).toBe('blocked');
		expect(runFlag(run({ state: 'running' })).key).toBe('running'); // no plan, running
		expect(runFlag(run({ state: 'waiting' })).key).toBe('waiting');
	});

	it('runFlag honours run.gate even without a plan gate', () => {
		expect(runFlag(run({ gate: true, state: 'running' })).key).toBe('gate');
	});
});
