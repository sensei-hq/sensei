// WP8: the rows the plan hides, grouped for the one line that replaces them.
// The decision is the SERVER's (`isBillingHiddenRepo`, sent as
// `MyRepoWire.billing_hidden`); this reads that flag and takes no predicate, so
// nothing on a screen re-derives the rule. Sharing's hidden line uses it, and
// Billing's member line will count with it. Pure.
import type { MyRepoWire } from './client-data';

/** One organization whose private repositories the plan hides, and where to fix it. */
export interface BillingHiddenOrg {
	org: string;
	count: number;
	href: string;
}

/** The rows to list (in their order) and, per organization in first-seen order,
 *  how many the plan hides — linked to that organization's Plan & billing. */
export function splitBillingHidden(rows: MyRepoWire[]): { shown: MyRepoWire[]; hidden: BillingHiddenOrg[] } {
	const shown: MyRepoWire[] = [];
	const counts = new Map<string, number>();
	for (const r of rows) {
		if (!r.billing_hidden || r.owning_org === null) {
			shown.push(r);
			continue;
		}
		counts.set(r.owning_org, (counts.get(r.owning_org) ?? 0) + 1);
	}
	return { shown, hidden: [...counts].map(([org, count]) => ({ org, count, href: `/org/${org}/billing` })) };
}
