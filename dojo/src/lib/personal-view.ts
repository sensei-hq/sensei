// Pure derivations for the personal-zone Govern + Dōjōs screens (chunk 3 —
// mockup `ScrConstitution` · `ScrRulePacks` · `ScrMyDojos` · `ScrContributions`).
// Kept as a plain module (no Svelte, no `$app/*`) so the grouping, splitting,
// and tallying unit-test without rendering — the same discipline as
// `nav.ts` and `preview/view.ts`.

import type {
	KitDojo,
	KitLadderRung,
	KitRulePack,
	KitContribution,
	KitContribStat
} from './components/kit/types';

/* ── constitution ───────────────────────────────────────────────────────── */

/** The rungs that make up your standing personal constitution — personal +
 *  stack, the two scopes that apply to every project you touch (mockup
 *  `ScrConstitution` `personalRungs`). Order-preserving against the ladder. */
export function personalRungs(ladder: KitLadderRung[]): KitLadderRung[] {
	return ladder.filter((r) => r.id === 'personal' || r.id === 'stack');
}

/* ── rule packs ───────────────────────────────────────────────────────────── */

/** Split rule packs into the adopted set and the still-available set (mockup
 *  `ScrRulePacks` `adopted` / `avail`). Pure — reads `adopted` off each pack. */
export function splitPacks(packs: KitRulePack[]): {
	adopted: KitRulePack[];
	available: KitRulePack[];
} {
	return {
		adopted: packs.filter((p) => p.adopted),
		available: packs.filter((p) => !p.adopted)
	};
}

/* ── my dōjōs ─────────────────────────────────────────────────────────────── */

/** The membership groups shown on My dōjōs, in mockup order — employer, then
 *  clients, then communities, then the user's own Personal dōjō (my-dojos
 *  resolved design Q5). A group with no members is dropped so the screen never
 *  renders an empty section (mockup `ScrMyDojos` `groups.map`+`if`). */
export const DOJO_GROUPS: readonly { kind: string; label: string; icon: string }[] = [
	{ kind: 'employer', label: 'Employer', icon: 'buildings-2' },
	{ kind: 'client', label: 'Clients', icon: 'case-round' },
	{ kind: 'community', label: 'Communities', icon: 'users-group-two-rounded' },
	{ kind: 'personal', label: 'Personal', icon: 'user' }
];

/** A rendered membership group — its label/icon and the dōjōs in it. */
export interface DojoGroup {
	kind: string;
	label: string;
	icon: string;
	items: KitDojo[];
}

/** Group memberships by kind in the fixed employer → clients → communities
 *  order, dropping any empty group. */
export function groupDojos(dojos: KitDojo[]): DojoGroup[] {
	return DOJO_GROUPS.map((g) => ({ ...g, items: dojos.filter((d) => d.kind === g.kind) })).filter(
		(g) => g.items.length > 0
	);
}

/* ── contributions ────────────────────────────────────────────────────────── */

/** Tally the contributions you shared upstream by status (approved / pending),
 *  keeping the lifetime `helped` count from the fixture stat. Derived from the
 *  rows so the stat row stays honest against the list it summarizes. */
export function tallyContributions(
	mine: KitContribution[],
	stat: KitContribStat
): KitContribStat {
	return {
		approved: mine.filter((c) => c.status === 'approved').length,
		pending: mine.filter((c) => c.status === 'pending').length,
		helped: stat.helped
	};
}
