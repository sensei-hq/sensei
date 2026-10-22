import { describe, it, expect } from 'vitest';
import {
	rulesToLadder,
	rulesToSections,
	relayRules,
	relayToKitConflicts,
	isHardRule,
	type ConstitutionRule,
	type RelayConstitution
} from './constitution-map';

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
			rule('workspace', 'p'),
			rule('organization', 'o'),
			rule('user', 'u'),
			rule('repository', 'r')
		]);
		expect(rungs.map((r) => r.scope)).toEqual(['Personal', 'Dōjō', 'Workspace', 'Repository']);
	});

	it('groups rules under their scope and surfaces the mandatory ★ lock', () => {
		const rungs = rulesToLadder([
			rule('organization', 'no secrets in logs', 'mandatory'),
			rule('organization', 'prefer Result', 'recommended')
		]);
		expect(rungs).toHaveLength(1);
		expect(rungs[0].scope).toBe('Dōjō');
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

	it('knows no retired rung — team · client · technology are not scopes (D25 · D29 · D32)', () => {
		// The ladder is dōjō → workspace → repository; a stack is a pack, not a level.
		const rungs = rulesToLadder([rule('team', 'x'), rule('client', 'y'), rule('technology', 'z')]);
		expect(rungs.map((r) => r.scope)).toEqual(['Other', 'Other', 'Other']);
	});
});

const CONSTITUTION: RelayConstitution = {
	rules: [
		{ scope_key: 'organization', namespace: 'Acme', title: 'never log secrets', enforcement: 'mandatory' },
		{ scope_key: 'workspace', namespace: null, title: 'prefer early returns', enforcement: 'recommended' }
	],
	conflicts: [
		{
			topic: 'never log secrets',
			loser_scope: 'repository',
			winner_scope: 'organization',
			why: 'a higher-authority scope already states this rule',
			locked: true
		}
	],
	locks: 1
};

describe('relayRules — federated constitution → ConstitutionRule[] (reuses rulesToLadder)', () => {
	it('maps scope_key/title/enforcement through and uses the namespace as the rung name', () => {
		const rungs = rulesToLadder(relayRules(CONSTITUTION));
		expect(rungs.map((r) => r.scope)).toEqual(['Dōjō', 'Workspace']);
		const dojo = rungs.find((r) => r.scope === 'Dōjō')!;
		expect(dojo.name).toBe('Acme'); // real namespace name, not a stub
		expect(dojo.rules?.[0]).toMatchObject({ text: 'never log secrets', hard: true });
	});

	it('falls back to the scope label when a rule has no namespace (honest, not blank)', () => {
		const rungs = rulesToLadder(relayRules(CONSTITUTION));
		expect(rungs.find((r) => r.scope === 'Workspace')!.name).toBe('Workspace');
	});
});

describe('relayToKitConflicts — discards → KitConflict[] (daemon-computed winner/loser)', () => {
	it('maps each discard to both sides with the shared topic + the ★ lock', () => {
		const conflicts = relayToKitConflicts(CONSTITUTION);
		expect(conflicts).toHaveLength(1);
		expect(conflicts[0]).toMatchObject({
			topic: 'never log secrets',
			loser: { level: 'Repository', text: 'never log secrets' },
			winner: { level: 'Dōjō', text: 'never log secrets' },
			locked: true
		});
	});

	it('is empty when nothing was discarded', () => {
		expect(relayToKitConflicts({ rules: [], conflicts: [], locks: 0 })).toEqual([]);
	});
});

describe('rulesToSections', () => {
	it('groups the dōjō scopes into Dōjō / Workspace / Repository, ordered by level', () => {
		const sections = rulesToSections([
			rule('repository', 'strict types'),
			rule('workspace', 'payments owns X'),
			rule('organization', 'dōjō-wide')
		]);
		expect(sections.map((s) => [s.scope, s.group])).toEqual([
			['organization-ns', 'Dōjō'],
			['workspace-ns', 'Workspace'],
			['repository-ns', 'Repository']
		]);
	});

	it('is empty for no rules', () => {
		expect(rulesToSections([])).toEqual([]);
	});
});
