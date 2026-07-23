import { describe, expect, it } from 'vitest';
import { flattenCandidates, impactTone, needsSecondApproval } from './dojo2-triage-view';
import type { KitTriageGroup } from './components/kit/types';

// Pure presentation helpers for the maintainer triage console (ScrTriage). No
// DOM, no runes — the selection store composes these so the grouping, the
// impact tone, and the second-approval rule all unit-test on their own.

const groups: KitTriageGroup[] = [
	{
		scope: 'Payments',
		items: [
			{ id: 't1', kanji: '紋', title: 'A', origin: 'x', conf: 0.9, conflicts: 1, dups: 0, impact: 'high' },
			{ id: 't2', kanji: '検', title: 'B', origin: 'y', conf: 0.7, conflicts: 0, dups: 2, impact: 'normal' }
		]
	},
	{
		scope: 'Auth',
		items: [
			{ id: 't3', kanji: '守', title: 'C', origin: 'z', conf: 0.95, conflicts: 0, dups: 0, impact: 'safety' }
		]
	}
];

describe('flattenCandidates — every candidate across the scope groups, in order', () => {
	it('flattens groups to a single ordered list', () => {
		const all = flattenCandidates(groups);
		expect(all.map((c) => c.id)).toEqual(['t1', 't2', 't3']);
	});

	it('returns an empty list for no groups', () => {
		expect(flattenCandidates([])).toEqual([]);
	});
});

describe('impactTone — token classes for the impact chip', () => {
	it('tints high impact accent', () => {
		expect(impactTone('high').text).toBe('text-accent');
		expect(impactTone('high').soft).toBe('bg-accent-soft');
	});

	it('tints safety impact danger', () => {
		expect(impactTone('safety').text).toBe('text-danger');
		expect(impactTone('safety').soft).toBe('bg-danger-soft');
	});

	it('falls back to a neutral tone for normal/low/unknown', () => {
		expect(impactTone('normal').text).toBe('text-ink-mute');
		expect(impactTone('low').text).toBe('text-ink-mute');
		expect(impactTone('whatever').text).toBe('text-ink-mute');
	});
});

describe('needsSecondApproval — high + safety candidates route to a second signature', () => {
	it('is true for high and safety', () => {
		expect(needsSecondApproval('high')).toBe(true);
		expect(needsSecondApproval('safety')).toBe(true);
	});

	it('is false for normal and low', () => {
		expect(needsSecondApproval('normal')).toBe(false);
		expect(needsSecondApproval('low')).toBe(false);
	});
});
