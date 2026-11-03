// Map the live `/v1/…/billing` response onto the Plan & billing screen's
// KitBilling shape. Only what the billing plane knows is shown: the billable seat
// count (dojo.tenant_seat_usage) and, when a billing account exists, its plan and
// renewal date. With no account the org is on Free (D46). The Org price is the
// catalog's; no read-only count or invoice exists until a payment provider is
// wired (D-BILLING), so those are absent — never a mockup's (#106). Pure, so it
// unit-tests without a DOM.
import type { KitBilling } from './components/kit/types';
import type { BillingResponse } from './admin-data';
import { BILLING_TIERS, ORG_PRICE_PER_CONTRIBUTOR } from './billing-catalog';

/** ISO date → a short "Aug 1" renewal label; passthrough if unparseable.
 *  Formatted in UTC so the label matches the stored calendar day regardless of
 *  the viewer's timezone. */
function shortDate(iso: string): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

/** The catalog plan an account's raw plan key is on. "team" was the Org plan's
 *  old name; anything not Org reads as Free. */
function planId(plan: string | null): 'free' | 'org' {
	return plan === 'org' || plan === 'team' ? 'org' : 'free';
}

/** The screen built from the live response: the seat count always, the plan,
 *  its renewal and the current plan from the account — Free without one. Pure. */
export function toKitBilling(res: BillingResponse): KitBilling {
	const id = planId(res.account?.plan ?? null);
	return {
		plan: BILLING_TIERS.find((t) => t.id === id)?.name ?? null,
		perSeat: ORG_PRICE_PER_CONTRIBUTOR,
		seatsActive: res.usage.seats_used,
		seatsReadonly: null,
		renews: res.account?.period_end ? shortDate(res.account.period_end) : null,
		tiers: BILLING_TIERS.map((t) => ({ ...t, current: t.id === id })),
		invoices: []
	};
}

/** The screen with nothing loaded (another section, or a failed read whose
 *  error the page shows): no plan, no seats, no invoices. */
export function noBilling(): KitBilling {
	return {
		plan: null,
		perSeat: null,
		seatsActive: null,
		seatsReadonly: null,
		renews: null,
		tiers: BILLING_TIERS,
		invoices: []
	};
}
