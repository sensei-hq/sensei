import { describe, it, expect } from 'vitest';
import {
	statusBadge,
	segmentStateBadge,
	progressWidth,
	gateSeverity,
	orderGatesByUrgency,
	blockedSummary,
	gateHref
} from './relay-view';
import type { RelayRunStatus, SegmentState, RelayGate } from './relay-data';

const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

// A gate builder mirroring the RelayGateCard spec — a real RelayGate shape with
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
			const cases: Record<RelayRunStatus, { label: string; toneClass: string }> = {
				running: { label: 'Running', toneClass: 'text-accent' },
				paused: { label: 'Paused', toneClass: 'text-ink-mute' },
				stalled: { label: 'Stuck', toneClass: 'text-warning' },
				crashed: { label: 'Crashed', toneClass: 'text-danger' },
				blocked: { label: 'Needs you', toneClass: 'text-accent' },
				done: { label: 'Done', toneClass: 'text-success' },
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
				toneClass: 'text-ink-mute'
			});
		});
	});

	describe('segmentStateBadge', () => {
		it('maps every segment state to a plain-language label + tone class', () => {
			const cases: Record<SegmentState, { label: string; toneClass: string }> = {
				pending: { label: 'Queued', toneClass: 'text-ink-mute' },
				active: { label: 'In progress', toneClass: 'text-success' },
				done: { label: 'Done', toneClass: 'text-ink-mute' },
				skipped: { label: 'Skipped', toneClass: 'text-ink-faint' },
				failed: { label: 'Failed', toneClass: 'text-danger' },
				blocked: { label: 'Needs you', toneClass: 'text-accent' },
				needs_review: { label: 'Needs you', toneClass: 'text-accent' }
			};
			for (const [state, expected] of Object.entries(cases)) {
				expect(segmentStateBadge(state as SegmentState)).toEqual(expected);
			}
		});

		it('rises needs_review and blocked as accent-toned "Needs you" (they gate on you)', () => {
			expect(segmentStateBadge('needs_review')).toEqual({ label: 'Needs you', toneClass: 'text-accent' });
			expect(segmentStateBadge('blocked')).toEqual({ label: 'Needs you', toneClass: 'text-accent' });
		});

		it('reads active as the live "In progress" success tone', () => {
			expect(segmentStateBadge('active').label).toBe('In progress');
			expect(segmentStateBadge('active').toneClass).toBe('text-success');
		});

		it('tones failed danger and skipped faint', () => {
			expect(segmentStateBadge('failed').toneClass).toBe('text-danger');
			expect(segmentStateBadge('skipped').toneClass).toBe('text-ink-faint');
		});

		it('falls back to a muted "Unknown" for an unrecognized state', () => {
			expect(segmentStateBadge('bogus' as SegmentState)).toEqual({
				label: 'Unknown',
				toneClass: 'text-ink-mute'
			});
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
			expect(gateHref(gate({ run_id: 'run-42' }))).toBe('/console/relay/run-42');
		});

		it('returns null for a missing run id or the all-zeros uuid (no orphan link)', () => {
			expect(gateHref(gate({ run_id: null }))).toBeNull();
			expect(gateHref(gate({ run_id: ZERO_UUID }))).toBeNull();
		});
	});
});
