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
	it('is the ladder order Dōjō · Workspace · Repository (D25)', () => {
		expect(LADDER_GROUPS).toEqual(['Dōjō', 'Workspace', 'Repository']);
	});
});

describe('sectionsByGroup', () => {
	it('buckets the sections under their authoring group, dropping empty groups', () => {
		const grouped = sectionsByGroup(sections);
		expect(grouped.map((g) => g.group)).toEqual(['Dōjō', 'Workspace', 'Repository']);
		expect(grouped.find((g) => g.group === 'Dōjō')?.sections.map((s) => s.id)).toEqual(['dojo']);
		expect(grouped.find((g) => g.group === 'Workspace')?.sections.map((s) => s.id)).toEqual([
			'ws-payments',
			'ws-platform'
		]);
		expect(grouped.find((g) => g.group === 'Repository')?.sections.map((s) => s.id)).toEqual([
			'repo-web',
			'repo-ledger'
		]);
	});

	it('omits a group with no sections', () => {
		const workspacesOnly = sections.filter((s) => s.group === 'Workspace');
		const grouped = sectionsByGroup(workspacesOnly);
		expect(grouped.map((g) => g.group)).toEqual(['Workspace']);
	});
});

describe('include state (includeKey · isIncluded · excludedCount)', () => {
	it('a rule is included by default (absent from the map)', () => {
		expect(isIncluded({}, 'dojo', 0)).toBe(true);
	});

	it('a rule is excluded only when its key is explicitly false', () => {
		const map = { [includeKey('dojo', 1)]: false };
		expect(isIncluded(map, 'dojo', 1)).toBe(false);
		// a different index in the same section stays included.
		expect(isIncluded(map, 'dojo', 0)).toBe(true);
	});

	it('excludedCount counts the section rules turned off', () => {
		const dojo = sections.find((s) => s.id === 'dojo')!;
		const map = {
			[includeKey('dojo', 1)]: false,
			[includeKey('dojo', 3)]: false
		};
		expect(excludedCount(map, dojo)).toBe(2);
	});

	it('excludedCount is zero for an all-included section', () => {
		const dojo = sections.find((s) => s.id === 'dojo')!;
		expect(excludedCount({}, dojo)).toBe(0);
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
