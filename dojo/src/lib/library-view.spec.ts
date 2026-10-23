import { describe, expect, it } from 'vitest';
import {
	ALL_PACKS,
	LIB_CATS,
	LIB_LEVELS,
	LIB_PACKS,
	type LibPack
} from '$lib/library-data';
import {
	defaultLevel,
	emptySelection,
	findRule,
	includedCount,
	isHardLocked,
	isIncluded,
	isNonNegotiable,
	isPackFullyChosen,
	levelForPack,
	levelLabel,
	nonNegotiableCount,
	packChosenCount,
	packsInArea,
	selectedCount,
	type LibrarySelection
} from '$lib/library-view';

// Pure selection-math tests for the constitution library. Exercises the four
// things the screen depends on: which rules are included, the per-rule level,
// the per-rule non-negotiable ★ (hard guards auto-locked), and the live counts.

// A small pack fixture with a mix of hard/soft rules, independent of the real
// catalog so the math is tested in isolation.
function fixturePack(): LibPack {
	return {
		id: 'fix',
		area: 'core',
		name: 'Fixture',
		source: 'test',
		defLevel: 'workspace',
		rec: false,
		regulated: false,
		checkers: [],
		rules: [
			{ id: 'fix:0', text: 'soft one', hard: false },
			{ id: 'fix:1', text: 'hard one', hard: true },
			{ id: 'fix:2', text: 'soft two', hard: false }
		]
	};
}

// A real catalog rule id (used to test findRule + isNonNegotiable against the
// actual data). OWASP "No secrets in source" is a hard guard.
const HARD_RULE_ID = 'sec-owasp:0';
const SOFT_RULE_ID = 'sec-owasp:3'; // "Validate & encode every external input…"

describe('emptySelection', () => {
	it('starts with nothing included, no overrides, no authored rules', () => {
		const sel = emptySelection();
		expect(includedCount(sel)).toBe(0);
		expect(selectedCount(sel)).toBe(0);
		expect(nonNegotiableCount(sel)).toBe(0);
		expect(sel.authored).toEqual([]);
	});
});

describe('catalog helpers', () => {
	it('packsInArea returns only that area, matching LIB_PACKS', () => {
		for (const cat of LIB_CATS) {
			const got = packsInArea(cat.id);
			expect(got.map((p) => p.id)).toEqual(LIB_PACKS[cat.id].map((p) => p.id));
			expect(got.every((p) => p.area === cat.id)).toBe(true);
		}
	});

	it('packsInArea is empty for an unknown area', () => {
		expect(packsInArea('nope')).toEqual([]);
	});

	it('findRule locates a rule by id and returns undefined otherwise', () => {
		expect(findRule(HARD_RULE_ID)?.hard).toBe(true);
		expect(findRule(SOFT_RULE_ID)?.hard).toBe(false);
		expect(findRule('does-not-exist')).toBeUndefined();
	});

	it('every catalog rule id is unique (selection keys never collide)', () => {
		const ids = ALL_PACKS.flatMap((p) => p.rules.map((r) => r.id));
		expect(new Set(ids).size).toBe(ids.length);
	});

	it('defaultLevel reads the pack defLevel, falling back to the dōjō', () => {
		expect(defaultLevel('sec-owasp')).toBe('dojo');
		expect(defaultLevel('arch-gof')).toBe('workspace');
		// a stack pack applies at each repository where the stack is detected (D33).
		expect(defaultLevel('stk-ts')).toBe('repository');
		expect(defaultLevel('unknown-pack')).toBe('dojo');
	});
});

describe('selection toggles', () => {
	it('isIncluded reflects the included map', () => {
		const sel: LibrarySelection = { ...emptySelection(), included: { 'fix:0': true } };
		expect(isIncluded(sel, 'fix:0')).toBe(true);
		expect(isIncluded(sel, 'fix:1')).toBe(false);
	});

	it('includedCount counts only truthy entries', () => {
		const sel: LibrarySelection = {
			...emptySelection(),
			included: { 'fix:0': true, 'fix:1': false, 'fix:2': true }
		};
		expect(includedCount(sel)).toBe(2);
	});

	it('selectedCount adds authored rules to included catalog rules', () => {
		const sel: LibrarySelection = {
			...emptySelection(),
			included: { 'fix:0': true },
			authored: [{ id: 'a1', text: 'ship flags in two releases', area: 'core', level: 'dojo', hard: false }]
		};
		expect(includedCount(sel)).toBe(1);
		expect(selectedCount(sel)).toBe(2);
	});
});

describe('per-pack level', () => {
	it('levelForPack returns the default when no override is set', () => {
		expect(levelForPack(emptySelection(), 'sec-owasp')).toBe('dojo');
		expect(levelForPack(emptySelection(), 'arch-gof')).toBe('workspace');
	});

	it('levelForPack returns the override when set', () => {
		const sel: LibrarySelection = { ...emptySelection(), packLevel: { 'sec-owasp': 'workspace' } };
		expect(levelForPack(sel, 'sec-owasp')).toBe('workspace');
	});

	it('the levels are where a pack is applied: dōjō · workspace · repository (D25 · D29)', () => {
		expect(LIB_LEVELS.map((l) => l.id)).toEqual(['dojo', 'workspace', 'repository']);
		expect(LIB_LEVELS.map((l) => l.label)).toEqual(['Dōjō', 'Workspace', 'Repository']);
	});

	it('every language & stack pack applies at the repository where its stack is detected (D33)', () => {
		expect(LIB_PACKS.stack.length).toBeGreaterThan(0);
		expect(LIB_PACKS.stack.every((p) => p.defLevel === 'repository')).toBe(true);
	});

	it('levelLabel maps ids to labels and passes an unknown through', () => {
		expect(levelLabel('dojo')).toBe('Dōjō');
		expect(levelLabel('workspace')).toBe('Workspace');
		expect(levelLabel('repository')).toBe('Repository');
	});
});

describe('hard-lock + non-negotiable', () => {
	const pack = fixturePack();

	it('a hard guard is not locked until it is included', () => {
		const hard = pack.rules[1];
		expect(isHardLocked(hard, false)).toBe(false);
		expect(isHardLocked(hard, true)).toBe(true);
	});

	it('a soft rule is never hard-locked', () => {
		const soft = pack.rules[0];
		expect(isHardLocked(soft, true)).toBe(false);
	});

	it('an included hard-guard catalog rule is non-negotiable even without a star', () => {
		const sel: LibrarySelection = { ...emptySelection(), included: { [HARD_RULE_ID]: true } };
		expect(isNonNegotiable(sel, HARD_RULE_ID)).toBe(true);
	});

	it('an excluded hard guard is not non-negotiable (★ only counts once included)', () => {
		expect(isNonNegotiable(emptySelection(), HARD_RULE_ID)).toBe(false);
	});

	it('an included soft rule is non-negotiable only when starred', () => {
		const off: LibrarySelection = { ...emptySelection(), included: { [SOFT_RULE_ID]: true } };
		expect(isNonNegotiable(off, SOFT_RULE_ID)).toBe(false);
		const on: LibrarySelection = {
			...emptySelection(),
			included: { [SOFT_RULE_ID]: true },
			starred: { [SOFT_RULE_ID]: true }
		};
		expect(isNonNegotiable(on, SOFT_RULE_ID)).toBe(true);
	});

	it('a starred-but-excluded soft rule is not non-negotiable', () => {
		const sel: LibrarySelection = { ...emptySelection(), starred: { [SOFT_RULE_ID]: true } };
		expect(isNonNegotiable(sel, SOFT_RULE_ID)).toBe(false);
	});
});

describe('counts (the live footer)', () => {
	it('counts a hard guard as non-negotiable automatically', () => {
		const sel: LibrarySelection = {
			...emptySelection(),
			included: { [HARD_RULE_ID]: true, [SOFT_RULE_ID]: true }
		};
		expect(selectedCount(sel)).toBe(2);
		expect(nonNegotiableCount(sel)).toBe(1); // only the hard guard
	});

	it('adds a starred soft rule to the non-negotiable count', () => {
		const sel: LibrarySelection = {
			...emptySelection(),
			included: { [HARD_RULE_ID]: true, [SOFT_RULE_ID]: true },
			starred: { [SOFT_RULE_ID]: true }
		};
		expect(nonNegotiableCount(sel)).toBe(2);
	});

	it('counts an authored hard rule as non-negotiable', () => {
		const sel: LibrarySelection = {
			...emptySelection(),
			authored: [
				{ id: 'a1', text: 'no PII in logs', area: 'sec', level: 'dojo', hard: true },
				{ id: 'a2', text: 'log at info by default', area: 'core', level: 'dojo', hard: false }
			]
		};
		expect(selectedCount(sel)).toBe(2);
		expect(nonNegotiableCount(sel)).toBe(1);
	});

	it('never counts an unincluded rule even if starred', () => {
		const sel: LibrarySelection = { ...emptySelection(), starred: { [SOFT_RULE_ID]: true } };
		expect(nonNegotiableCount(sel)).toBe(0);
	});
});

describe('whole-pack helpers', () => {
	const pack = fixturePack();

	it('packChosenCount counts included rules of the pack', () => {
		const sel: LibrarySelection = { ...emptySelection(), included: { 'fix:0': true, 'fix:2': true } };
		expect(packChosenCount(sel, pack)).toBe(2);
	});

	it('isPackFullyChosen is true only when all rules are included', () => {
		const partial: LibrarySelection = { ...emptySelection(), included: { 'fix:0': true } };
		expect(isPackFullyChosen(partial, pack)).toBe(false);
		const full: LibrarySelection = {
			...emptySelection(),
			included: { 'fix:0': true, 'fix:1': true, 'fix:2': true }
		};
		expect(isPackFullyChosen(full, pack)).toBe(true);
	});
});
