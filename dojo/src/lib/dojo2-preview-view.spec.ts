import { describe, expect, it } from 'vitest';
import {
	previewRungs,
	discardedTexts,
	effectiveRules,
	showsConflicts,
	lockCount,
	scopeIdsFor
} from './dojo2-preview-view';
import { ladder, conflicts } from './components/kit/fixtures';
import type { KitProject } from './components/kit/types';

// Pure derivations behind the dojo2 project-constitution preview (mockup
// ScrProjectPreview `previewRungs` / `showConflicts` / `effective`). Kept as a
// standalone module (NOT the shipped `preview-view.ts`, which the (console)
// group owns) so the drill-in resolves off the kit fixtures the mockup used.

const personal: KitProject = {
	id: 'p',
	name: 'personal-site',
	repo: 'rin/personal-site',
	classification: 'personal',
	phase: 'watch'
};
const company: KitProject = {
	id: 'c',
	name: 'lumen-auth',
	repo: 'acme/lumen-auth',
	classification: 'company',
	phase: 'notice'
};
const client: KitProject = {
	id: 'g',
	name: 'globex-portal',
	repo: 'globex/portal',
	classification: 'client',
	phase: 'adopt'
};

describe('scopeIdsFor — the rung ids a classification composes', () => {
	it('personal composes personal → project → stack', () => {
		expect(scopeIdsFor('personal')).toEqual(['personal', 'project', 'stack']);
	});

	it('client composes company → client → personal → project → stack', () => {
		expect(scopeIdsFor('client')).toEqual(['company', 'client', 'personal', 'project', 'stack']);
	});

	it('company (default) composes company → personal → project → stack', () => {
		expect(scopeIdsFor('company')).toEqual(['company', 'personal', 'project', 'stack']);
	});
});

describe('previewRungs — the ladder for a project', () => {
	it('renames the project rung to the project name', () => {
		const rungs = previewRungs(personal, ladder);
		const project = rungs.find((r) => r.id === 'project');
		expect(project?.name).toBe('personal-site');
	});

	it('only includes rungs the fixtures define (unknown ids are dropped)', () => {
		// the kit fixture ladder defines company / client / personal only — the
		// project / stack ids have no fixture rung, so they synthesize a stub.
		const rungs = previewRungs(client, ladder);
		expect(rungs.map((r) => r.id)).toEqual(['company', 'client', 'personal', 'project', 'stack']);
	});

	it('a personal project omits the company + client rungs', () => {
		const ids = previewRungs(personal, ladder).map((r) => r.id);
		expect(ids).not.toContain('company');
		expect(ids).not.toContain('client');
	});
});

describe('showsConflicts — the ladder discards only for non-personal work', () => {
	it('is false for a personal project (its ladder alone — nothing to settle)', () => {
		expect(showsConflicts(personal)).toBe(false);
	});

	it('is true for a company project', () => {
		expect(showsConflicts(company)).toBe(true);
	});

	it('is true for a client project', () => {
		expect(showsConflicts(client)).toBe(true);
	});
});

describe('discardedTexts — the losing rule copy the ladder eliminated', () => {
	it('is empty for a personal project', () => {
		expect(discardedTexts(personal, conflicts)).toEqual([]);
	});

	it('lists every conflict loser for a company project', () => {
		const discarded = discardedTexts(company, conflicts);
		expect(discarded).toContain(conflicts[0].loser.text);
		expect(discarded).toContain(conflicts[1].loser.text);
	});
});

describe('effectiveRules — the consolidated constitution', () => {
	it('flattens every rung rule with its scope level attached', () => {
		const rules = effectiveRules(company, ladder, conflicts);
		expect(rules.length).toBeGreaterThan(0);
		// each carries the rung scope as its level (for the "by layer" jump).
		expect(rules.every((r) => typeof r.level === 'string' && r.level.length > 0)).toBe(true);
	});

	it('drops the rules the ladder discarded', () => {
		const rules = effectiveRules(company, ladder, conflicts);
		const discarded = discardedTexts(company, conflicts);
		for (const text of discarded) {
			expect(rules.some((r) => r.text === text)).toBe(false);
		}
	});

	it('a personal project keeps every rule (nothing discarded)', () => {
		const rules = effectiveRules(personal, ladder, conflicts);
		// personal ladder = personal + project + stack; only personal has fixtures.
		expect(rules.length).toBeGreaterThan(0);
		expect(discardedTexts(personal, conflicts)).toEqual([]);
	});
});

describe('lockCount — the non-negotiable (★) rules across the ladder', () => {
	it('counts the hard rules in the composed rungs', () => {
		const rungs = previewRungs(company, ladder);
		// company rung has 3 hard rules in the fixture; personal/project/stack none.
		expect(lockCount(rungs)).toBe(3);
	});

	it('is zero when no rung has a hard rule (personal-only ladder)', () => {
		const rungs = previewRungs(personal, ladder);
		expect(lockCount(rungs)).toBe(0);
	});
});
