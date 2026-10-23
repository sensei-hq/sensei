import { describe, expect, it } from 'vitest';
import { previewRungs, discardedTexts, effectiveRules, showsConflicts, lockCount } from './view';
import { ladder, conflicts } from '../components/kit/fixtures';
import type { KitLadderRung, KitWorkspace } from '../components/kit/types';

// Pure derivations behind the dojo workspace-constitution preview (mockup
// ScrWorkspacePreview `previewRungs` / `showConflicts` / `effective`). Kept as a
// standalone module (NOT the shipped `preview-view.ts`, which the (console)
// group owns) so the drill-in resolves off the kit fixtures the mockup used.

const personal: KitWorkspace = {
	id: 'p',
	name: 'personal-site',
	repo: 'rin/personal-site',
	classification: 'personal',
	phase: 'watch'
};
const company: KitWorkspace = {
	id: 'c',
	name: 'lumen-auth',
	repo: 'acme/lumen-auth',
	classification: 'company',
	phase: 'notice'
};
const client: KitWorkspace = {
	id: 'g',
	name: 'globex-portal',
	repo: 'globex/portal',
	classification: 'client',
	phase: 'adopt'
};

describe('previewRungs — the ladder for a workspace', () => {
	it('renames the workspace rung to the workspace name', () => {
		const withWorkspace: KitLadderRung[] = [
			{ id: 'general', kanji: '全', scope: 'General', name: 'General', caption: '', rules: [] },
			{ id: 'workspace', kanji: '件', scope: 'Workspace', name: 'placeholder', caption: '', rules: [] }
		];
		const rungs = previewRungs(personal, withWorkspace);
		const workspace = rungs.find((r) => r.id === 'workspace');
		expect(workspace?.name).toBe('personal-site');
		// A composed workspace rung is reused (relabelled), never duplicated by a synth anchor.
		expect(rungs.filter((r) => r.id === 'workspace').length).toBe(1);
	});

	it('renders EVERY scope the daemon composed — never drops one the old scaffold omitted', () => {
		// The regression: the daemon federates rules at governance scopes the mockup
		// scaffold never listed (general · organization). previewRungs must
		// render them all, or a real 30-rule constitution shows as one rule.
		const composed: KitLadderRung[] = [
			{ id: 'general', kanji: '全', scope: 'General', name: 'General', caption: '', rules: [
				{ kanji: '守', text: 'g1', hard: true }, { kanji: '守', text: 'g2', hard: false }] },
			{ id: 'organization', kanji: '社', scope: 'Dōjō', name: 'Acme', caption: '', rules: [
				{ kanji: '守', text: 'o1', hard: false }] },
			{ id: 'workspace', kanji: '件', scope: 'Workspace', name: 'x', caption: '', rules: [
				{ kanji: '守', text: 'p1', hard: false }] }
		];
		const ids = previewRungs(personal, composed).map((r) => r.id);
		// …ending at the synthesized repository anchor (none was composed).
		expect(ids).toEqual(['general', 'organization', 'workspace', 'repository']);
	});

	it('is the dōjō → workspace → repository ladder, with no retired rung (D25 · D29 · D32)', () => {
		const ids = previewRungs(company, ladder).map((r) => r.id);
		expect(ids).toEqual(['organization', 'workspace', 'repository']);
		for (const retired of ['team', 'client', 'technology', 'stack', 'company']) {
			expect(ids).not.toContain(retired);
		}
	});

	it('carries the packs applied at each rung — the stack packs sit on the repository (D33)', () => {
		const rungs = previewRungs(company, ladder);
		expect(rungs.find((r) => r.id === 'repository')?.packs).toEqual(['TypeScript', 'Svelte']);
		expect(rungs.find((r) => r.id === 'organization')?.packs).toEqual(['OWASP baseline']);
		expect(rungs.find((r) => r.id === 'workspace')?.packs).toBeUndefined();
	});

	it('synthesizes an empty repository anchor when the daemon composed no repository rung', () => {
		const rungs = previewRungs(client, ladder.filter((r) => r.id !== 'repository'));
		expect(rungs.map((r) => r.id)).toEqual(['organization', 'workspace', 'repository']);
		const anchor = rungs.at(-1)!;
		expect(anchor.scope).toBe('Repository');
		expect(anchor.name).toBe('globex/portal'); // this repository
		expect(anchor.rules).toEqual([]); // the anchor is honest-empty
		expect(anchor.packs).toBeUndefined(); // and adopts no fabricated pack
	});

	it('reuses a composed repository rung — never duplicated by the anchor', () => {
		const rungs = previewRungs(client, ladder);
		expect(rungs.filter((r) => r.id === 'repository').length).toBe(1);
	});
});

describe('showsConflicts — the ladder discards only for non-personal work', () => {
	it('is false for a personal workspace (its ladder alone — nothing to settle)', () => {
		expect(showsConflicts(personal)).toBe(false);
	});

	it('is true for a company workspace', () => {
		expect(showsConflicts(company)).toBe(true);
	});

	it('is true for a client workspace', () => {
		expect(showsConflicts(client)).toBe(true);
	});
});

describe('discardedTexts — the losing rule copy the ladder eliminated', () => {
	it('is empty for a personal workspace', () => {
		expect(discardedTexts(personal, conflicts)).toEqual([]);
	});

	it('lists every conflict loser for a company workspace', () => {
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

	it('a personal workspace keeps every rule (nothing discarded)', () => {
		const rules = effectiveRules(personal, ladder, conflicts);
		// A personal workspace settles no conflicts, so every composed rung rule survives.
		expect(rules.length).toBeGreaterThan(0);
		expect(discardedTexts(personal, conflicts)).toEqual([]);
	});
});

describe('lockCount — the non-negotiable (★) rules across the ladder', () => {
	it('counts the hard rules across every composed rung', () => {
		const rungs = previewRungs(company, ladder);
		// The fixture ladder carries 4 hard rules: dōjō (3) + workspace (1);
		// the repository rung adds none.
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
