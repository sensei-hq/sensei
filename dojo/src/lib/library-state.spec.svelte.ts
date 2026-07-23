import { describe, expect, it } from 'vitest';
import { createLibraryStore } from '$lib/library-state.svelte';
import { LIB_PACKS } from '$lib/library-data';

// The rune-store spec (`.spec.svelte.ts` so $state compiles). Proves the store
// holds the working selection and delegates the math correctly: toggles change
// counts, level overrides stick, the ★ auto-locks on hard guards, and authored
// rules are added / removed / classified.

// Real catalog fixtures used across the tests.
const owasp = LIB_PACKS.sec.find((p) => p.id === 'sec-owasp')!; // has hard rules
const solid = LIB_PACKS.core.find((p) => p.id === 'core-solid')!; // all soft, defLevel org
const HARD_RULE = 'sec-owasp:0'; // "No secrets in source" — hard guard
const SOFT_RULE = 'sec-owasp:3'; // "Validate & encode…" — soft

describe('createLibraryStore — selection toggles', () => {
	it('starts empty', () => {
		const s = createLibraryStore();
		expect(s.selectedCount).toBe(0);
		expect(s.nonNegotiableCount).toBe(0);
		expect(s.isIncluded(SOFT_RULE)).toBe(false);
	});

	it('toggleRule includes then excludes a rule, moving the count', () => {
		const s = createLibraryStore();
		s.toggleRule(SOFT_RULE);
		expect(s.isIncluded(SOFT_RULE)).toBe(true);
		expect(s.selectedCount).toBe(1);
		s.toggleRule(SOFT_RULE);
		expect(s.isIncluded(SOFT_RULE)).toBe(false);
		expect(s.selectedCount).toBe(0);
	});

	it('toggleAll adds every rule of a pack, then clears them', () => {
		const s = createLibraryStore();
		s.toggleAll(solid);
		expect(s.isPackFullyChosen(solid)).toBe(true);
		expect(s.packChosenCount(solid)).toBe(solid.rules.length);
		expect(s.selectedCount).toBe(solid.rules.length);
		s.toggleAll(solid);
		expect(s.isPackFullyChosen(solid)).toBe(false);
		expect(s.packChosenCount(solid)).toBe(0);
	});
});

describe('createLibraryStore — non-negotiable ★', () => {
	it('auto-locks a hard guard as non-negotiable once included', () => {
		const s = createLibraryStore();
		expect(s.isNonNegotiable(HARD_RULE)).toBe(false); // not included yet
		s.toggleRule(HARD_RULE);
		expect(s.isNonNegotiable(HARD_RULE)).toBe(true);
		expect(s.nonNegotiableCount).toBe(1);
	});

	it('toggleStar is a no-op on an included hard guard (can not be relaxed)', () => {
		const s = createLibraryStore();
		s.toggleRule(HARD_RULE);
		s.toggleStar(HARD_RULE); // attempt to relax
		expect(s.isNonNegotiable(HARD_RULE)).toBe(true);
		expect(s.nonNegotiableCount).toBe(1);
	});

	it('stars an included soft rule, adding it to the non-negotiable count', () => {
		const s = createLibraryStore();
		s.toggleRule(SOFT_RULE);
		expect(s.isNonNegotiable(SOFT_RULE)).toBe(false);
		expect(s.nonNegotiableCount).toBe(0);
		s.toggleStar(SOFT_RULE);
		expect(s.isNonNegotiable(SOFT_RULE)).toBe(true);
		expect(s.nonNegotiableCount).toBe(1);
		s.toggleStar(SOFT_RULE); // relax
		expect(s.isNonNegotiable(SOFT_RULE)).toBe(false);
		expect(s.nonNegotiableCount).toBe(0);
	});

	it('a starred rule that is not included does not count', () => {
		const s = createLibraryStore();
		s.toggleStar(SOFT_RULE);
		expect(s.isNonNegotiable(SOFT_RULE)).toBe(false);
		expect(s.nonNegotiableCount).toBe(0);
	});
});

describe('createLibraryStore — per-pack level', () => {
	it('returns the pack default until overridden, then the override', () => {
		const s = createLibraryStore();
		expect(s.levelForPack(owasp.id)).toBe('org'); // default
		s.setPackLevel(owasp.id, 'proj');
		expect(s.levelForPack(owasp.id)).toBe('proj');
	});
});

describe('createLibraryStore — write your own', () => {
	it('adds an authored rule and counts it as selected', () => {
		const s = createLibraryStore();
		const id = s.addAuthored('flags removed within two releases', 'core', 'org', false);
		expect(id).not.toBeNull();
		expect(s.selectedCount).toBe(1);
		expect(s.authored).toHaveLength(1);
		expect(s.authored[0].text).toBe('flags removed within two releases');
	});

	it('trims and ignores blank authored text', () => {
		const s = createLibraryStore();
		expect(s.addAuthored('   ', 'core', 'org', false)).toBeNull();
		expect(s.selectedCount).toBe(0);
		const id = s.addAuthored('  keep me  ', 'core', 'org', false);
		expect(id).not.toBeNull();
		expect(s.authored[0].text).toBe('keep me');
	});

	it('counts an authored hard rule as non-negotiable', () => {
		const s = createLibraryStore();
		s.addAuthored('no PII in logs', 'sec', 'org', true);
		expect(s.nonNegotiableCount).toBe(1);
	});

	it('classifies authored rules per area and removes by id', () => {
		const s = createLibraryStore();
		s.addAuthored('core rule', 'core', 'org', false);
		const secId = s.addAuthored('sec rule', 'sec', 'org', false);
		expect(s.authoredInArea('core')).toHaveLength(1);
		expect(s.authoredInArea('sec')).toHaveLength(1);
		expect(s.authoredInArea('design')).toHaveLength(0);
		s.removeAuthored(secId!);
		expect(s.authoredInArea('sec')).toHaveLength(0);
		expect(s.selectedCount).toBe(1);
	});
});
