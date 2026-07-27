// Plan-model normalizers (ported from dojo2-kit.jsx k2Phases / k2Tasks /
// k2StageState / k2PlanProgress / k2RunFlag). Pure functions over a KitPlan —
// the single source of truth for how a run's plan rolls up to phases, progress,
// and the "what does this run want from me" flag. Tones live in kit/vocab
// (K2_NODE); this module owns the shape logic.

import type { KitPhase, KitPlan, KitRun, KitTask, KitTaskState } from './types';
import { taskState } from './vocab';

/** Roll-up order — the most urgent live state a phase can be in wins. */
const URGENCY: KitTaskState[] = ['failed', 'needs_review', 'blocked', 'active', 'pending'];

/** Normalize any plan (authored `{goal, phases}` or a legacy phase array) into
 * phases of tasks, aliasing legacy states and defaulting missing fields. */
export function phases(plan: KitPlan | null | undefined): KitPhase[] {
	const raw = Array.isArray(plan) ? plan : (plan?.phases ?? []);
	return raw.map((p, i) => ({
		id: p.id || `p${i}`,
		title: p.title || `Phase ${i + 1}`,
		mode: p.mode,
		tasks: (p.tasks ?? []).map((t) => ({
			...t,
			title: t.title,
			state: taskState(t.state),
			deps: t.deps ?? []
		}))
	}));
}

/** Every task across all phases, flattened. */
export function tasks(plan: KitPlan | null | undefined): KitTask[] {
	return phases(plan).flatMap((p) => p.tasks);
}

/** A phase's roll-up state — the most urgent live task state, or `done`. */
export function stageState(phase: KitPhase): KitTaskState {
	const states = phase.tasks.map((t) => taskState(t.state));
	return URGENCY.find((k) => states.includes(k)) ?? 'done';
}

export interface PlanProgress {
	done: number;
	total: number;
	running: number;
	/** 1-based index of the current (first not-done) phase. */
	stage: number;
	stages: number;
	/** 0–100, counting active tasks as half-done. */
	pct: number;
	stageName: string;
}

/** Progress across the whole plan: done/total, current stage, and a percentage
 * that counts an active task as half-complete. */
export function planProgress(plan: KitPlan | null | undefined): PlanProgress {
	const ps = phases(plan);
	const ts = ps.flatMap((p) => p.tasks);
	const total = ts.length || 1;
	const done = ts.filter((t) => t.state === 'done' || t.state === 'skipped').length;
	const running = ts.filter((t) => t.state === 'active').length;
	const liveIdx = ps.findIndex((p) => stageState(p) !== 'done');
	return {
		done,
		total,
		running,
		stage: liveIdx === -1 ? ps.length : liveIdx + 1,
		stages: ps.length,
		pct: Math.round(((done + running * 0.5) / total) * 100),
		stageName: (ps[liveIdx] ?? ps[ps.length - 1])?.title ?? ''
	};
}

export interface RunFlag {
	key: 'gate' | 'failed' | 'blocked' | 'running' | 'waiting';
	label: string;
	/** Foreground text token class for the label/stripe. */
	text: string;
	/** Solid-fill token class for the stripe/dot. */
	fill: string;
	/** Primary action verb, when the run wants something from you. */
	cta?: string;
	/** True when the run is waiting on the viewer. */
	act?: boolean;
}

/** What the run wants from you right now — drives the stripe, label, and CTA.
 * Precedence: needs-approval → failed → blocked → running → waiting. */
export function runFlag(run: KitRun): RunFlag {
	const states = tasks(run.plan).map((t) => t.state);
	const gated = tasks(run.plan).some(
		(t) => t.is_gate && t.state !== 'done' && t.state !== 'skipped'
	);
	if (gated || states.includes('needs_review') || run.gate)
		return { key: 'gate', label: 'Needs approval', text: 'text-accent', fill: 'bg-accent', cta: 'Approve', act: true };
	if (states.includes('failed'))
		return { key: 'failed', label: 'Task failed', text: 'text-danger', fill: 'bg-danger', cta: 'Review', act: true };
	if (states.includes('blocked'))
		return { key: 'blocked', label: 'Blocked', text: 'text-warning', fill: 'bg-warning', cta: 'Unblock', act: true };
	if (run.state === 'running')
		return { key: 'running', label: 'Running', text: 'text-success', fill: 'bg-success' };
	return { key: 'waiting', label: 'Waiting', text: 'text-warning', fill: 'bg-warning' };
}
