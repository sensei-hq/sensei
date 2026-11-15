// The org page's billing section, as a function the page calls and a test can drive
// (reviews/Design/L4-billing-followup-r1.md L2). The billing read and the caller's
// repositories read run in PARALLEL; the second is grouped to this org's
// billing-hidden repositories (`loadOrgHidden`). Each read fails on its own: a failed
// repositories read is that section's error, and the billing read stands.
import type { MyRepoWire } from './client-data';
import type { KitBilling } from './components/kit/types';
import type { ReadError } from './read-error';
import { loadOrgHidden, type BillingHiddenOrg } from './billing-hidden';

export interface BillingSection {
	billing: KitBilling;
	billingError: ReadError | null;
	billingHidden: BillingHiddenOrg | null;
	billingHiddenError: ReadError | null;
}

export async function loadBillingSection(
	slug: string,
	readBilling: () => Promise<{ value: KitBilling; error: ReadError | null }>,
	readRepos: () => Promise<MyRepoWire[]>
): Promise<BillingSection> {
	const [billing, hidden] = await Promise.all([readBilling(), loadOrgHidden(slug, readRepos)]);
	return { billing: billing.value, billingError: billing.error, billingHidden: hidden.hidden, billingHiddenError: hidden.error };
}
