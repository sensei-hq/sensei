import { describe, it, expect } from 'vitest';
import {
	statusBadge,
	segmentStateBadge,
	sessionView,
	runProgress,
	progressWidth,
	gateSeverity,
	orderGatesByUrgency,
	blockedSummary,
	gateHref
} from './relay-view';
import type { RelayRunStatus, SegmentState, RelayGate } from './relay-data';
import { K2_NODE } from '$lib/components/kit/vocab';

const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

// A gate builder — a real RelayGate shape with
// overridable fields so each ordering/summary case is explicit about what it varies.
function gate(overrides: Partial<RelayGate> = {}): RelayGate {
	return {
		id: 'g1',
		seq: 1,
		run_id: 'run-1',
		run_title: 'Round-trip',
		segment_id: null,
		kind: 'approval',
		payload: {},
		created_at: '2026-07-18T12:00:00.000Z',
		...overrides
	};
}

describe('relay-view', () => {
	describe('statusBadge', () => {
		it('maps every run status to a plain-language label + tone class', () => {
			const cases: Record<RelayRunStatus, { label: string; toneClass: string; pulseClass?: string }> = {
				// running is success, as the list's dot and the mockup's jade: accent
				// is "needs you", and a running run does not (audit 2026-10-28, item 5).
				running: { label: 'Running', toneClass: 'text-success', pulseClass: 'bg-success' },
				paused: { label: 'Paused', toneClass: 'text-ink-soft' },
				stalled: { label: 'Stuck', toneClass: 'text-warning' },
				crashed: { label: 'Crashed', toneClass: 'text-danger' },
				blocked: { label: 'Needs you', toneClass: 'text-accent' },
				done: { label: 'Done', toneClass: 'text-ink-soft' },
				failed: { label: 'Failed', toneClass: 'text-danger' }
			};
			for (const [status, expected] of Object.entries(cases)) {
				expect(statusBadge(status as RelayRunStatus)).toEqual(expected);
			}
		});

		it('uses phone plain-language: stalled → "Stuck", blocked → "Needs you"', () => {
			expect(statusBadge('stalled').label).toBe('Stuck');
			expect(statusBadge('blocked').label).toBe('Needs you');
		});

		it('keeps crashed and failed distinct labels but both danger-toned', () => {
			expect(statusBadge('crashed').label).toBe('Crashed');
			expect(statusBadge('failed').label).toBe('Failed');
			expect(statusBadge('crashed').toneClass).toBe('text-danger');
			expect(statusBadge('failed').toneClass).toBe('text-danger');
		});

		it('falls back to a muted "Unknown" for an unrecognized status', () => {
			expect(statusBadge('bogus' as RelayRunStatus)).toEqual({
				label: 'Unknown',
				toneClass: 'text-ink-soft'
			});
		});
	});

	describe('segmentStateBadge', () => {
		// One step-colour table: the kit's K2_NODE, which the detail's outline and
		// the list's pips also render (audit 2026-10-28, item 5: active was accent in
		// the list pips and success in the detail; blocked danger vs accent).
		it('takes every step colour from the kit, with plain-language labels', () => {
			const cases: Record<SegmentState, { label: string; toneClass: string; pipClass: string }> = {
				pending: { label: 'Queued', toneClass: K2_NODE.pending.text, pipClass: 'border border-paper-edge bg-transparent' },
				active: { label: 'In progress', toneClass: K2_NODE.active.text, pipClass: K2_NODE.active.fill },
				done: { label: 'Done', toneClass: K2_NODE.done.text, pipClass: K2_NODE.done.fill },
				skipped: { label: 'Skipped', toneClass: K2_NODE.skipped.text, pipClass: 'border border-paper-edge bg-transparent' },
				failed: { label: 'Failed', toneClass: K2_NODE.failed.text, pipClass: K2_NODE.failed.fill },
				blocked: { label: 'Blocked', toneClass: K2_NODE.blocked.text, pipClass: K2_NODE.blocked.fill },
				needs_review: { label: 'Needs you', toneClass: K2_NODE.needs_review.text, pipClass: K2_NODE.needs_review.fill }
			};
			for (const [state, expected] of Object.entries(cases)) {
				expect(segmentStateBadge(state as SegmentState), state).toEqual(expected);
			}
		});

		it('rises only the step that gates on you as "Needs you", in accent', () => {
			expect(segmentStateBadge('needs_review')).toMatchObject({ label: 'Needs you', toneClass: 'text-accent' });
		});

		it('falls back to a muted "Unknown" for an unrecognized state', () => {
			expect(segmentStateBadge('bogus' as SegmentState)).toEqual({
				label: 'Unknown',
				toneClass: 'text-ink-soft',
				pipClass: 'border border-paper-edge bg-transparent'
			});
		});
	});

	describe('sessionView — the list card, from the same tables', () => {
		const base = { status: 'running', needs: 0, attention: null, done: 5, total: 12, lastEventAt: null, plan: { phases: [] } };

		it('a gate waiting on you is accent, whatever else is going on', () => {
			const v = sessionView({ ...base, needs: 2, attention: 'stalled' } as never);
			expect(v).toMatchObject({ dotClass: 'bg-accent', whyClass: 'text-accent font-semibold' });
		});

		it('a non-gate problem takes the kit status tone: stalled and blocked warn, failed is danger', () => {
			expect(sessionView({ ...base, attention: 'stalled' } as never).dotClass).toBe('bg-warning');
			expect(sessionView({ ...base, attention: 'blocked' } as never).dotClass).toBe('bg-warning');
			expect(sessionView({ ...base, attention: 'failed' } as never).dotClass).toBe('bg-danger');
		});

		it('a running run with nothing wrong is success — as the detail badge', () => {
			expect(sessionView(base as never).dotClass).toBe('bg-success');
			expect(statusBadge('running').toneClass).toBe('text-success');
		});

		it('its pips are the detail\'s step colours, and it counts done/total', () => {
			const phases = [{ id: 'a', state: 'done' }, { id: 'b', state: 'active' }, { id: 'c', state: 'blocked' }];
			const v = sessionView({ ...base, plan: { phases } } as never);
			expect(v.pips.map((p) => p.pipClass)).toEqual(phases.map((p) => segmentStateBadge(p.state as SegmentState).pipClass));
			expect(v.count).toBe('5/12');
		});
	});

	describe('runProgress', () => {
		it('is the run\'s own counts: the stage is the first phase not done', () => {
			const phases = [{ state: 'done' }, { state: 'active' }, { state: 'pending' }] as never;
			expect(runProgress({ done: 5, total: 12, plan: { phases } } as never)).toEqual({ stage: 2, stages: 3, pct: 42, width: '42%' });
			expect(runProgress({ done: 0, total: 0, plan: { phases: [] } } as never)).toEqual({ stage: 0, stages: 0, pct: 0, width: '0%' });
		});
	});

	describe('progressWidth', () => {
		it('returns the clamped percentage for normal progress', () => {
			expect(progressWidth(0, 5)).toBe('0%');
			expect(progressWidth(1, 4)).toBe('25%');
			expect(progressWidth(3, 4)).toBe('75%');
			expect(progressWidth(5, 5)).toBe('100%');
		});

		it('rounds to the nearest whole percent', () => {
			expect(progressWidth(1, 3)).toBe('33%');
			expect(progressWidth(2, 3)).toBe('67%');
		});

		it('is divide-by-zero safe — 0% when total is <= 0 or non-finite', () => {
			expect(progressWidth(3, 0)).toBe('0%');
			expect(progressWidth(1, -2)).toBe('0%');
			expect(progressWidth(1, NaN)).toBe('0%');
			expect(progressWidth(1, Infinity)).toBe('0%');
		});

		it('clamps out-of-range done into 0–100%', () => {
			expect(progressWidth(9, 4)).toBe('100%');
			expect(progressWidth(-3, 4)).toBe('0%');
		});
	});

	describe('gateSeverity', () => {
		it('reads an explicit payload.gate_severity when present', () => {
			expect(gateSeverity(gate({ kind: 'chat', payload: { gate_severity: 'blocking' } }))).toBe(
				'blocking'
			);
			expect(gateSeverity(gate({ kind: 'approval', payload: { gate_severity: 'advisory' } }))).toBe(
				'advisory'
			);
		});

		it('falls back to kind: approval/decision/stall block, chat/nudge advise', () => {
			expect(gateSeverity(gate({ kind: 'approval', payload: {} }))).toBe('blocking');
			expect(gateSeverity(gate({ kind: 'decision', payload: {} }))).toBe('blocking');
			expect(gateSeverity(gate({ kind: 'stall', payload: {} }))).toBe('blocking');
			expect(gateSeverity(gate({ kind: 'chat', payload: {} }))).toBe('advisory');
			expect(gateSeverity(gate({ kind: 'nudge', payload: {} }))).toBe('advisory');
		});

		it('treats a garbage payload.gate_severity as absent and falls back to kind', () => {
			expect(gateSeverity(gate({ kind: 'approval', payload: { gate_severity: 'bogus' } }))).toBe(
				'blocking'
			);
			expect(gateSeverity(gate({ kind: 'chat', payload: { gate_severity: 42 } }))).toBe('advisory');
		});

		it('treats an unknown kind with no payload severity as advisory (not urgent)', () => {
			expect(gateSeverity(gate({ kind: 'weird', payload: {} }))).toBe('advisory');
		});
	});

	describe('orderGatesByUrgency', () => {
		it('puts blocking gates before advisory ones', () => {
			const adv = gate({ id: 'adv', kind: 'chat' });
			const blk = gate({ id: 'blk', kind: 'approval' });
			expect(orderGatesByUrgency([adv, blk]).map((g) => g.id)).toEqual(['blk', 'adv']);
		});

		it('within a severity band, orders oldest-waiting first', () => {
			const newer = gate({ id: 'newer', created_at: '2026-07-18T12:00:00.000Z' });
			const older = gate({ id: 'older', created_at: '2026-07-18T09:00:00.000Z' });
			expect(orderGatesByUrgency([newer, older]).map((g) => g.id)).toEqual(['older', 'newer']);
		});

		it('is a total order: blocking-oldest, blocking-newer, advisory-oldest, advisory-newer', () => {
			const gates = [
				gate({ id: 'adv-new', kind: 'chat', created_at: '2026-07-18T12:00:00.000Z' }),
				gate({ id: 'blk-new', kind: 'approval', created_at: '2026-07-18T12:00:00.000Z' }),
				gate({ id: 'adv-old', kind: 'chat', created_at: '2026-07-18T08:00:00.000Z' }),
				gate({ id: 'blk-old', kind: 'approval', created_at: '2026-07-18T08:00:00.000Z' })
			];
			expect(orderGatesByUrgency(gates).map((g) => g.id)).toEqual([
				'blk-old',
				'blk-new',
				'adv-old',
				'adv-new'
			]);
		});

		it('is stable for equal keys (same severity + timestamp keep input order)', () => {
			const a = gate({ id: 'a', kind: 'approval', created_at: '2026-07-18T10:00:00.000Z' });
			const b = gate({ id: 'b', kind: 'approval', created_at: '2026-07-18T10:00:00.000Z' });
			expect(orderGatesByUrgency([a, b]).map((g) => g.id)).toEqual(['a', 'b']);
			expect(orderGatesByUrgency([b, a]).map((g) => g.id)).toEqual(['b', 'a']);
		});

		it('is pure — does not mutate the input array', () => {
			const gates = [gate({ id: 'a', kind: 'chat' }), gate({ id: 'b', kind: 'approval' })];
			const before = gates.map((g) => g.id);
			orderGatesByUrgency(gates);
			expect(gates.map((g) => g.id)).toEqual(before);
		});

		it('handles the empty list', () => {
			expect(orderGatesByUrgency([])).toEqual([]);
		});
	});

	describe('blockedSummary', () => {
		it('counts total, blocking and advisory', () => {
			const gates = [
				gate({ id: '1', kind: 'approval' }),
				gate({ id: '2', kind: 'decision' }),
				gate({ id: '3', kind: 'chat' })
			];
			expect(blockedSummary(gates)).toEqual({ total: 3, blocking: 2, advisory: 1 });
		});

		it('is all-zero for the empty list', () => {
			expect(blockedSummary([])).toEqual({ total: 0, blocking: 0, advisory: 0 });
		});
	});

	describe('gateHref', () => {
		it('deep-links to the gate’s run', () => {
			expect(gateHref(gate({ run_id: 'run-42' }))).toBe('/you/runs/run-42');
		});

		it('returns null for a missing run id or the all-zeros uuid (no orphan link)', () => {
			expect(gateHref(gate({ run_id: null }))).toBeNull();
			expect(gateHref(gate({ run_id: ZERO_UUID }))).toBeNull();
		});
	});
});
