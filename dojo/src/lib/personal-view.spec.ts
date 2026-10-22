import { describe, expect, it } from 'vitest';
import {
	personalRungs,
	splitPacks,
	groupDojos,
	tallyContributions,
	DOJO_GROUPS
} from './personal-view';
import {
	ladder,
	personalLadder,
	rulePacks,
	dojos,
	contributionsMine,
	contributionsStat
} from './components/kit/fixtures';
import type { KitDojo } from './components/kit/types';

// Pure derivations for the chunk-3 personal Govern + Dōjōs screens. No render —
// these back the screens' section splits, groupings, and stat tallies.

describe('personalRungs — your standing constitution', () => {
	it('keeps only your personal (user) rung — your rules stay yours, in sensei (D25)', () => {
		const rungs = personalRungs([...personalLadder, ...ladder]);
		expect(rungs.map((r) => r.id)).toEqual(['user']);
	});

	it('drops the dōjō ladder rungs (dōjō · workspace · repository)', () => {
		expect(personalRungs(ladder)).toEqual([]);
	});
});

describe('splitPacks — adopted vs available', () => {
	it('splits the fixture packs on their adopted flag', () => {
		const { adopted, available } = splitPacks(rulePacks);
		// derived from the fixture split so the credible-pack content can grow.
		const adoptedCount = rulePacks.filter((p) => p.adopted).length;
		expect(adopted.length).toBe(adoptedCount);
		expect(available.length).toBe(rulePacks.length - adoptedCount);
		expect(adopted.length + available.length).toBe(rulePacks.length);
		expect(adopted.every((p) => p.adopted)).toBe(true);
		expect(available.every((p) => !p.adopted)).toBe(true);
	});

	it('is empty-safe', () => {
		expect(splitPacks([])).toEqual({ adopted: [], available: [] });
	});
});

describe('rulePacks fixtures — every pack carries its rules', () => {
	it('gives each pack a non-empty rules list (the count is rules.length)', () => {
		for (const pack of rulePacks) {
			expect(Array.isArray(pack.rules)).toBe(true);
			expect(pack.rules.length).toBeGreaterThan(0);
		}
	});

	it('has no blank or duplicate rule statements within a pack', () => {
		for (const pack of rulePacks) {
			expect(pack.rules.every((r) => r.trim().length > 0)).toBe(true);
			expect(new Set(pack.rules).size).toBe(pack.rules.length);
		}
	});
});

describe('groupDojos — membership groups', () => {
	it('groups fixtures into organizations (the fixture has no personal dōjō)', () => {
		const groups = groupDojos(dojos);
		expect(groups.map((g) => g.kind)).toEqual(['organization']);
	});

	it('carries the right label + members into each group', () => {
		const groups = groupDojos(dojos);
		const orgs = groups.find((g) => g.kind === 'organization');
		expect(orgs?.label).toBe('Organizations');
		expect(orgs?.items.map((d) => d.slug)).toContain('acme');
	});

	it('drops a group with no members', () => {
		const onlyOrganization: KitDojo[] = dojos.filter((d) => d.kind === 'organization');
		const groups = groupDojos(onlyOrganization);
		expect(groups.map((g) => g.kind)).toEqual(['organization']);
	});

	it('returns no groups for an empty membership list', () => {
		expect(groupDojos([])).toEqual([]);
	});

	it('groups a team dōjō under Teams, apart from organisations', () => {
		const team: KitDojo = { ...dojos[0], slug: 'guild', kind: 'team' };
		const groups = groupDojos([...dojos, team]);
		expect(groups.map((g) => g.kind)).toEqual(['organization', 'team']);
		expect(groups.find((g) => g.kind === 'team')?.items.map((d) => d.slug)).toEqual(['guild']);
	});

	it('exposes the groups the dōjō can tell apart', () => {
		// Employer / Clients / Communities are gone: nothing could derive them, and
		// the one behaviour attached to Client (anonymised sends) described a
		// difference that does not exist — every insight is anonymised.
		// A Team is a hand-made dōjō — told apart from a forge-proven Organization
		// (decision 2026-10-12).
		expect(DOJO_GROUPS.map((g) => g.kind)).toEqual(['organization', 'team', 'personal']);
	});

	it('puts a personal-kind dōjō in its own Personal group (my-dojos resolved design Q5)', () => {
		const personal: KitDojo[] = [
			{ slug: 'me', kanji: '己', name: 'Personal', kind: 'personal', role: 'owner', route: 'gh/me' }
		];
		const groups = groupDojos(personal);
		expect(groups.map((g) => g.kind)).toEqual(['personal']);
		expect(groups[0].label).toBe('Personal');
		expect(groups[0].items.map((d) => d.slug)).toEqual(['me']);
	});
});

describe('tallyContributions — the stat row', () => {
	it('counts approved + pending off the shared rows', () => {
		const stat = tallyContributions(contributionsMine, contributionsStat);
		// fixtures: 2 approved, 1 pending, 1 declined.
		expect(stat.approved).toBe(2);
		expect(stat.pending).toBe(1);
	});

	it('keeps the lifetime helped count from the fixture stat', () => {
		const stat = tallyContributions(contributionsMine, contributionsStat);
		expect(stat.helped).toBe(contributionsStat.helped);
	});

	it('tallies to zero for an empty list', () => {
		const stat = tallyContributions([], contributionsStat);
		expect(stat.approved).toBe(0);
		expect(stat.pending).toBe(0);
		expect(stat.helped).toBe(contributionsStat.helped);
	});
});
