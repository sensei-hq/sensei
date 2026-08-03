import { describe, expect, it } from 'vitest';
import { previewRungs, discardedTexts, effectiveRules, showsConflicts, lockCount } from './view';
import { ladder, conflicts } from '../components/kit/fixtures';
import type { KitLadderRung, KitProject } from '../components/kit/types';

// Pure derivations behind the dojo project-constitution preview (mockup
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

describe('previewRungs — the ladder for a project', () => {
	it('renames the project rung to the project name', () => {
		const withProject: KitLadderRung[] = [
			{ id: 'general', kanji: '全', scope: 'General', name: 'General', caption: '', rules: [] },
			{ id: 'project', kanji: '件', scope: 'Project', name: 'placeholder', caption: '', rules: [] }
		];
		const rungs = previewRungs(personal, withProject);
		const project = rungs.find((r) => r.id === 'project');
		expect(project?.name).toBe('personal-site');
		// A composed project rung is reused (relabelled), never duplicated by a synth anchor.
		expect(rungs.filter((r) => r.id === 'project').length).toBe(1);
	});

	it('renders EVERY scope the daemon composed — never drops one the old scaffold omitted', () => {
		// The regression: the daemon federates rules at governance scopes the mockup
		// scaffold never listed (general · organization · team). previewRungs must
		// render them all, or a real 30-rule constitution shows as one rule.
		const composed: KitLadderRung[] = [
			{ id: 'general', kanji: '全', scope: 'General', name: 'General', caption: '', rules: [
				{ kanji: '守', text: 'g1', hard: true }, { kanji: '守', text: 'g2', hard: false }] },
			{ id: 'organization', kanji: '社', scope: 'Company', name: 'Acme', caption: '', rules: [
				{ kanji: '守', text: 'o1', hard: false }] },
			{ id: 'project', kanji: '件', scope: 'Project', name: 'x', caption: '', rules: [
				{ kanji: '守', text: 'p1', hard: false }] }
		];
		const ids = previewRungs(personal, composed).map((r) => r.id);
		expect(ids).toEqual(['general', 'organization', 'project']);
	});

	it('synthesizes an empty project anchor when the daemon composed no project rung', () => {
		// the kit fixture ladder is company / client / personal / stack — no project.
		const rungs = previewRungs(client, ladder);
		expect(rungs.map((r) => r.id)).toEqual(['company', 'client', 'personal', 'stack', 'project']);
		expect(rungs.at(-1)?.rules).toEqual([]); // the anchor is honest-empty
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
		// A personal project settles no conflicts, so every composed rung rule survives.
		expect(rules.length).toBeGreaterThan(0);
		expect(discardedTexts(personal, conflicts)).toEqual([]);
	});
});

describe('lockCount — the non-negotiable (★) rules across the ladder', () => {
	it('counts the hard rules across every composed rung', () => {
		const rungs = previewRungs(company, ladder);
		// The fixture ladder carries 4 hard rules: company (3) + client (1);
		// personal / stack / the synth project anchor add none.
		expect(lockCount(rungs)).toBe(4);
	});

	it('is zero when no composed rung has a hard rule', () => {
		const soft: KitLadderRung[] = [
			{ id: 'general', kanji: '全', scope: 'General', name: 'General', caption: '', rules: [
				{ kanji: '守', text: 's1', hard: false }] }
		];
		expect(lockCount(previewRungs(personal, soft))).toBe(0);
	});
});
