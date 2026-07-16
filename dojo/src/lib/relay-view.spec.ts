import { describe, it, expect } from 'vitest';
import { statusBadge, progressWidth } from './relay-view';
import type { RelayRunStatus } from './relay-data';

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
});
