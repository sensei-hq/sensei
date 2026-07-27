// Map the live `/v1/…/billing` response onto the Plan & billing screen's
// KitBilling shape. Only the figures the schema-only billing authoritatively
// knows are overlaid — the billable seat count (dojo.tenant_seat_usage) and, when
// a billing account exists, the plan + renewal date. The pricing catalog
// (perSeat / tiers / relayRows) and invoice history stay the illustrative fixture
// until a payment provider is wired (D-BILLING = schema + route only). Pure +
// side-effect-free so it unit-tests without a DOM.
import type { KitBilling } from './components/kit/types';
import type { BillingResponse } from './admin-data';

/** ISO date → the fixture's short "Aug 1" renewal label; passthrough if
 *  unparseable. Formatted in UTC so the label matches the stored calendar day
 *  regardless of the viewer's timezone. */
function shortDate(iso: string): string {
	const d = new Date(iso);
	if (Number.isNaN(d.getTime())) return iso;
	return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

/** Capitalize a raw plan key for display ("team" → "Team"). */
function planLabel(plan: string): string {
	return plan ? plan.charAt(0).toUpperCase() + plan.slice(1) : plan;
}

/** Overlay the LIVE billing figures onto the illustrative fixture. `seatsActive`
 *  (the one number the billing plane computes: unique active users on private
 *  projects) is always taken from live usage; the plan + renewal come from the
 *  account when one exists. Everything else stays fixture. Pure. */
export function toKitBilling(fixture: KitBilling, res: BillingResponse): KitBilling {
	const seatsActive = res.usage?.seats_used ?? fixture.seatsActive;
	if (!res.account) {
		return { ...fixture, seatsActive };
	}
	return {
		...fixture,
		seatsActive,
		plan: planLabel(res.account.plan),
		renews: res.account.period_end ? shortDate(res.account.period_end) : fixture.renews
	};
}
