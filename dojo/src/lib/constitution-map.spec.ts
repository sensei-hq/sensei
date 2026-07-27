import { describe, it, expect } from 'vitest';
import { rulesToLadder, rulesToSections, isHardRule, type ConstitutionRule } from './constitution-map';

const rule = (scope_key: string, title: string, enforcement = 'required'): ConstitutionRule => ({
	scope_key,
	namespace_name: `${scope_key}-ns`,
	title,
	enforcement
});

describe('isHardRule', () => {
	it('is true only for mandatory enforcement', () => {
		expect(isHardRule('mandatory')).toBe(true);
		for (const e of ['advisory', 'recommended', 'required', 'unknown']) {
			expect(isHardRule(e)).toBe(false);
		}
	});
});

describe('rulesToLadder', () => {
	it('is empty for no rules (honest empty, never fabricated)', () => {
		expect(rulesToLadder([])).toEqual([]);
	});

	it('orders rungs broad→specific by scope level', () => {
		// deliberately out of order on input
		const rungs = rulesToLadder([
			rule('project', 'p'),
			rule('organization', 'o'),
			rule('user', 'u'),
			rule('technology', 't')
		]);
		expect(rungs.map((r) => r.scope)).toEqual(['Personal', 'Company', 'Stack', 'Project']);
	});

	it('groups rules under their scope and surfaces the mandatory ★ lock', () => {
		const rungs = rulesToLadder([
			rule('organization', 'no secrets in logs', 'mandatory'),
			rule('organization', 'prefer Result', 'recommended')
		]);
		expect(rungs).toHaveLength(1);
		expect(rungs[0].scope).toBe('Company');
		expect(rungs[0].name).toBe('organization-ns');
		expect(rungs[0].caption).toBe('2 rules');
		expect(rungs[0].rules?.map((r) => [r.text, r.hard])).toEqual([
			['no secrets in logs', true],
			['prefer Result', false]
		]);
	});

	it('routes an unknown scope to the fallback rung, sorted last', () => {
		const rungs = rulesToLadder([rule('wildcard', 'x'), rule('user', 'u')]);
		expect(rungs.map((r) => r.scope)).toEqual(['Personal', 'Other']);
	});
});

describe('rulesToSections', () => {
	it('groups org scopes into Company / Teams / Stacks, ordered by level', () => {
		const sections = rulesToSections([
			rule('technology', 'strict types'),
			rule('team', 'payments owns X'),
			rule('organization', 'company-wide')
		]);
		expect(sections.map((s) => [s.scope, s.group])).toEqual([
			['organization-ns', 'Company'],
			['technology-ns', 'Stacks'],
			['team-ns', 'Teams']
		]);
	});

	it('is empty for no rules', () => {
		expect(rulesToSections([])).toEqual([]);
	});
});
