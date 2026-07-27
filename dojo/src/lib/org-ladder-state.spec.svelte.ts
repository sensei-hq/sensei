import { describe, expect, it } from 'vitest';
import { createOrgLadder } from './org-ladder-state.svelte';
import { orgConstitutionFor } from './components/kit/fixtures';

// The reactive authoring state for ScrOrgLadder (mockup) — the active section,
// the per-rule include toggles, the show-excluded toggle, and the RuleEditor
// open/close. Delegates grouping + include math to the pure
// `org-ladder-view` module; here we prove the UI state reacts. Runs under
// the .svelte.ts test transform (like dojo2-preview-state.spec.svelte.ts).

const sections = orgConstitutionFor('acme');

describe('createOrgLadder — reactive authoring state', () => {
	it('opens on the first section with the editor closed', () => {
		const l = createOrgLadder(sections);
		expect(l.active).toBe(sections[0].id);
		expect(l.section.id).toBe(sections[0].id);
		expect(l.editing).toBe(null);
	});

	it('buckets sections into Company · Teams · Stacks groups', () => {
		const l = createOrgLadder(sections);
		expect(l.groups.map((g) => g.group)).toEqual(['Company', 'Teams', 'Stacks']);
	});

	it('setActive focuses another section', () => {
		const l = createOrgLadder(sections);
		l.setActive('team-pay');
		expect(l.active).toBe('team-pay');
		expect(l.section.id).toBe('team-pay');
	});

	it('every rule is included on open (excluded = 0)', () => {
		const l = createOrgLadder(sections);
		expect(l.excluded).toBe(0);
		expect(l.isIncluded(0)).toBe(true);
	});

	it('toggleInclude excludes then re-includes a rule (per active section)', () => {
		const l = createOrgLadder(sections);
		l.toggleInclude(1);
		expect(l.isIncluded(1)).toBe(false);
		expect(l.excluded).toBe(1);
		l.toggleInclude(1);
		expect(l.isIncluded(1)).toBe(true);
		expect(l.excluded).toBe(0);
	});

	it('include state is scoped to a section — switching sections resets the count', () => {
		const l = createOrgLadder(sections);
		l.toggleInclude(0); // exclude a company rule
		expect(l.excluded).toBe(1);
		l.setActive('team-pay');
		expect(l.excluded).toBe(0);
	});

	it('showExcluded toggles the hidden-rules reveal', () => {
		const l = createOrgLadder(sections);
		expect(l.showExcluded).toBe(false);
		l.toggleShowExcluded();
		expect(l.showExcluded).toBe(true);
	});

	it('openNew opens the editor with no rule (add mode)', () => {
		const l = createOrgLadder(sections);
		l.openNew();
		expect(l.editing).not.toBe(null);
		expect(l.editing?.rule).toBeUndefined();
	});

	it('openEdit opens the editor seeded with the rule', () => {
		const l = createOrgLadder(sections);
		const rule = sections[0].rules![0];
		l.openEdit(rule);
		// `$state` wraps the target in a reactive proxy, so compare by value.
		expect(l.editing?.rule).toStrictEqual(rule);
	});

	it('closeEditor clears the editor', () => {
		const l = createOrgLadder(sections);
		l.openNew();
		l.closeEditor();
		expect(l.editing).toBe(null);
	});
});
