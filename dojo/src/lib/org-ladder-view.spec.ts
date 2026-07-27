import { describe, expect, it } from 'vitest';
import {
	RULE_FAMILIES,
	LADDER_GROUPS,
	sectionsByGroup,
	includeKey,
	isIncluded,
	excludedCount,
	familyKanji
} from './org-ladder-view';
import { orgConstitutionFor } from './components/kit/fixtures';

// Pure derivations behind the dōjō-authoring ladder (mockup ScrOrgLadder). No
// Svelte, no DOM — the grouping, per-rule include state, and RuleEditor family
// vocab all unit-test as plain functions.

const sections = orgConstitutionFor('acme');

describe('LADDER_GROUPS', () => {
	it('is the authoring order Company · Teams · Stacks', () => {
		expect(LADDER_GROUPS).toEqual(['Company', 'Teams', 'Stacks']);
	});
});

describe('sectionsByGroup', () => {
	it('buckets the sections under their authoring group, dropping empty groups', () => {
		const grouped = sectionsByGroup(sections);
		expect(grouped.map((g) => g.group)).toEqual(['Company', 'Teams', 'Stacks']);
		expect(grouped.find((g) => g.group === 'Company')?.sections.map((s) => s.id)).toEqual([
			'company'
		]);
		expect(grouped.find((g) => g.group === 'Teams')?.sections.map((s) => s.id)).toEqual([
			'team-pay',
			'team-plat'
		]);
		expect(grouped.find((g) => g.group === 'Stacks')?.sections.map((s) => s.id)).toEqual([
			'stack-react',
			'stack-pg'
		]);
	});

	it('omits a group with no sections', () => {
		const teamsOnly = sections.filter((s) => s.group === 'Teams');
		const grouped = sectionsByGroup(teamsOnly);
		expect(grouped.map((g) => g.group)).toEqual(['Teams']);
	});
});

describe('include state (includeKey · isIncluded · excludedCount)', () => {
	it('a rule is included by default (absent from the map)', () => {
		expect(isIncluded({}, 'company', 0)).toBe(true);
	});

	it('a rule is excluded only when its key is explicitly false', () => {
		const map = { [includeKey('company', 1)]: false };
		expect(isIncluded(map, 'company', 1)).toBe(false);
		// a different index in the same section stays included.
		expect(isIncluded(map, 'company', 0)).toBe(true);
	});

	it('excludedCount counts the section rules turned off', () => {
		const company = sections.find((s) => s.id === 'company')!;
		const map = {
			[includeKey('company', 1)]: false,
			[includeKey('company', 3)]: false
		};
		expect(excludedCount(map, company)).toBe(2);
	});

	it('excludedCount is zero for an all-included section', () => {
		const company = sections.find((s) => s.id === 'company')!;
		expect(excludedCount({}, company)).toBe(0);
	});
});

describe('RULE_FAMILIES · familyKanji', () => {
	it('offers the six rule families the editor picks from', () => {
		expect(RULE_FAMILIES.map((f) => f.label)).toEqual([
			'guard',
			'pattern',
			'principle',
			'review',
			'stack',
			'shield'
		]);
	});

	it('defaults an unknown family to guard (守)', () => {
		expect(familyKanji('nope')).toBe('守');
		expect(familyKanji('技')).toBe('技');
	});
});
