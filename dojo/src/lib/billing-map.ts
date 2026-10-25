// Map the live `/v1/…/billing` response onto the Plan & billing screen's
// KitBilling shape. Only what the billing plane knows is shown: the billable seat
// count (dojo.tenant_seat_usage) and, when a billing account exists, its plan and
// renewal date. No price, read-only count or invoice exists until a payment
// provider is wired (D-BILLING), so those are absent — never a mockup's (#106).
// The tier and relay copy is the product catalog. Pure, so it unit-tests without
// a DOM.
import type { KitBilling } from './components/kit/types';
import type { BillingResponse } from './admin-data';
import { BILLING_TIERS, RELAY_ROWS } from './billing-catalog';

/** ISO date → a short "Aug 1" renewal label; passthrough if unparseable.
 *  Formatted in UTC so the label matches the stored calendar day regardless of
 *  the viewer's timezone. */
function shortDate(iso: string): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

/** Capitalize a raw plan key for display ("team" → "Team"). */
function planLabel(plan: string): string {
	return plan ? plan.charAt(0).toUpperCase() + plan.slice(1) : plan;
}

/** The screen built from the live response: the seat count always, the plan,
 *  its renewal and the current tier from the account when there is one. Pure. */
export function toKitBilling(res: BillingResponse): KitBilling {
	const plan = res.account?.plan ?? null;
	return {
		plan: plan === null ? null : planLabel(plan),
		perSeat: null,
		seatsActive: res.usage.seats_used,
		seatsReadonly: null,
		renews: res.account?.period_end ? shortDate(res.account.period_end) : null,
		tiers: BILLING_TIERS.map((t) => ({ ...t, current: t.id === plan })),
		relayRows: RELAY_ROWS,
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
		relayRows: RELAY_ROWS,
		invoices: []
	};
}
