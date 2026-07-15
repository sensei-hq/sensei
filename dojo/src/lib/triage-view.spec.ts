import { describe, expect, it } from 'vitest';
import {
	confidencePct,
	confidenceToneClass,
	confidenceWidth,
	groupByScope,
	kindKanji,
	kindLabel,
	relativeAge,
	scopeLabel,
	similarityBand,
	similarityToneClass
} from '$lib/triage-view';
import type { TriageRow } from '$lib/triage-data';

// A TriageRow factory with sensible defaults so each test names only what matters.
function row(over: Partial<TriageRow> = {}): TriageRow {
	return {
		signature: 'sig',
		artifact_id: 'a1',
		kind: 'pattern',
		title: 'A learning',
		owner_scope: { label: 'Company' },
		confidence: 0.8,
		contributor_count: 3,
		similarity: null,
		nearest_artifact_id: null,
		state: 'queued',
		created_at: '2026-07-07T00:00:00Z',
		...over
	};
}

describe('kindLabel / kindKanji', () => {
	it('maps known kinds to their label', () => {
		expect(kindLabel('guard')).toBe('Guard');
		expect(kindLabel('pattern')).toBe('Pattern');
		expect(kindLabel('persona')).toBe('Persona');
	});
	it('capitalises an unknown kind rather than failing', () => {
		expect(kindLabel('recipe')).toBe('Recipe');
		expect(kindLabel('')).toBe('Artifact');
	});
	it('gives a kanji per known kind and a default otherwise', () => {
		expect(kindKanji('guard')).toBe('守');
		expect(kindKanji('mystery')).toBe('芽');
	});
});

describe('scopeLabel', () => {
	it('reads a label/name field from an object scope', () => {
		expect(scopeLabel({ label: 'Team · Payments' })).toBe('Team · Payments');
		expect(scopeLabel({ name: 'Company' })).toBe('Company');
	});
	it('composes kind + value when there is no label', () => {
		expect(scopeLabel({ kind: 'stack', value: 'React' })).toBe('stack · React');
	});
	it('passes a string scope through', () => {
		expect(scopeLabel('Company')).toBe('Company');
	});
	it('handles null/odd shapes without throwing', () => {
		expect(scopeLabel(null)).toBe('Unscoped');
		expect(scopeLabel({ foo: 1 })).toBe('{"foo":1}');
	});
});

describe('confidence helpers', () => {
	it('tones by the mockup thresholds', () => {
		expect(confidenceToneClass(0.9)).toBe('text-success');
		expect(confidenceToneClass(0.75)).toBe('text-accent');
		expect(confidenceToneClass(0.5)).toBe('text-warning');
		expect(confidenceToneClass(null)).toBe('text-ink-mute');
	});
	it('renders percent + width, with a dash for unscored', () => {
		expect(confidencePct(0.84)).toBe('84');
		expect(confidencePct(null)).toBe('—');
		expect(confidenceWidth(0.5)).toBe('50%');
		expect(confidenceWidth(null)).toBe('0%');
	});
});

describe('similarity bands', () => {
	it('bands by the auto-dedupe thresholds', () => {
		expect(similarityBand(0.95)).toBe('merge');
		expect(similarityBand(0.8)).toBe('flagged');
		expect(similarityBand(0.4)).toBe('weak');
		expect(similarityBand(null)).toBe('none');
	});
	it('tones the chip by band', () => {
		expect(similarityToneClass(0.95)).toBe('text-accent');
		expect(similarityToneClass(0.8)).toBe('text-warning');
		expect(similarityToneClass(0.2)).toBe('text-ink-mute');
	});
});

describe('relativeAge', () => {
	const now = new Date('2026-07-07T12:00:00Z');
	it('renders compact units', () => {
		expect(relativeAge('2026-07-07T11:59:30Z', now)).toBe('just now');
		expect(relativeAge('2026-07-07T11:48:00Z', now)).toBe('12m');
		expect(relativeAge('2026-07-07T07:00:00Z', now)).toBe('5h');
		expect(relativeAge('2026-07-04T12:00:00Z', now)).toBe('3d');
	});
	it('returns empty for an unparseable timestamp', () => {
		expect(relativeAge('not-a-date', now)).toBe('');
	});
});

describe('groupByScope', () => {
	it('groups rows by scope label', () => {
		const groups = groupByScope([
			row({ signature: 'a', owner_scope: { label: 'Company' } }),
			row({ signature: 'b', owner_scope: { label: 'Team · Payments' } }),
			row({ signature: 'c', owner_scope: { label: 'Company' } })
		]);
		const company = groups.find((g) => g.scope === 'Company');
		expect(company?.rows.map((r) => r.signature).sort()).toEqual(['a', 'c']);
		expect(groups.find((g) => g.scope === 'Team · Payments')?.rows.length).toBe(1);
	});

	it('ranks rows strongest-first within a group (confidence desc)', () => {
		const groups = groupByScope([
			row({ signature: 'low', confidence: 0.6 }),
			row({ signature: 'high', confidence: 0.95 }),
			row({ signature: 'mid', confidence: 0.8 })
		]);
		expect(groups[0].rows.map((r) => r.signature)).toEqual(['high', 'mid', 'low']);
	});

	it('ranks groups by their strongest row', () => {
		const groups = groupByScope([
			row({ signature: 'w', owner_scope: { label: 'Weak' }, confidence: 0.5 }),
			row({ signature: 's', owner_scope: { label: 'Strong' }, confidence: 0.95 })
		]);
		expect(groups[0].scope).toBe('Strong');
	});

	it('sorts a null-confidence row last', () => {
		const groups = groupByScope([
			row({ signature: 'unscored', confidence: null }),
			row({ signature: 'scored', confidence: 0.7 })
		]);
		expect(groups[0].rows.map((r) => r.signature)).toEqual(['scored', 'unscored']);
	});

	it('returns an empty list for no rows', () => {
		expect(groupByScope([])).toEqual([]);
	});
});
