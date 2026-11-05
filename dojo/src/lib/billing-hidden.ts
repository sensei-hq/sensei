// WP8: the rows the plan hides. The decision is the SERVER's (`isBillingHiddenRepo`,
// sent as `MyRepoWire.billing_hidden`); this reads that flag and takes no predicate,
// so nothing on a screen re-derives the rule. A hidden repository is hidden for every
// user, and members are told nothing about it (user ruling 2026-11-05): Sharing only
// leaves the rows out. Admins see their org's group — count and names — in Billing.
// Pure, apart from `loadOrgHidden`'s injected read.
import type { MyRepoWire } from './client-data';
import { readError, type ReadError } from './read-error';

/** One organization's repositories the plan hides: how many, and which. */
export interface BillingHiddenOrg {
	org: string;
	count: number;
	repos: string[];
}

const label = (r: MyRepoWire) => r.name ?? r.repo_key;

/** The rows to list (in their order) and, per organization in first-seen order,
 *  the repositories the plan hides. A flagged row with no owning org can't be
 *  grouped, so it stays listed (the server's predicate makes that impossible). */
export function splitBillingHidden(rows: MyRepoWire[]): { shown: MyRepoWire[]; hidden: BillingHiddenOrg[] } {
	const shown: MyRepoWire[] = [];
	const byOrg = new Map<string, string[]>();
	for (const r of rows) {
		if (!r.billing_hidden || r.owning_org === null) {
			shown.push(r);
			continue;
		}
		byOrg.set(r.owning_org, [...(byOrg.get(r.owning_org) ?? []), label(r)]);
	}
	return { shown, hidden: [...byOrg].map(([org, repos]) => ({ org, count: repos.length, repos })) };
}

/** One organization's hidden group, or `null` when none of its repositories is hidden. */
export function orgHidden(rows: MyRepoWire[], org: string): BillingHiddenOrg | null {
	return splitBillingHidden(rows).hidden.find((h) => h.org === org) ?? null;
}

/** Billing's read: the caller's repositories (a tenant's repository appears for each
 *  of its members, so an admin's read covers the org), grouped for `org`. A failed
 *  read is the error — never "nothing hidden" (D83). */
export async function loadOrgHidden(
	org: string,
	read: () => Promise<MyRepoWire[]>
): Promise<{ hidden: BillingHiddenOrg | null; error: ReadError | null }> {
	try {
		return { hidden: orgHidden(await read(), org), error: null };
	} catch (e) {
		return { hidden: null, error: readError(e) };
	}
}
