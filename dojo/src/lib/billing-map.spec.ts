// Unit tests for toKitBilling — the Plan & billing screen built from the live
// `…/billing` response alone (#106). It used to overlay that response on the
// kit fixture, so an org with no billing account showed the fixture's plan,
// renewal, $12 seat price and three paid invoices as its own.
import { describe, it, expect } from 'vitest';
import { toKitBilling, noBilling } from './billing-map';
import { BILLING_TIERS, ORG_PRICE_PER_CONTRIBUTOR } from './billing-catalog';
import type { BillingResponse } from './admin-data';

const usage = (seats_used: number): BillingResponse['usage'] => ({
	seats_used,
	total_active_seats: seats_used,
	billable_users: []
});

const account = (plan: string, period_end: string | null): NonNullable<BillingResponse['account']> => ({
	plan,
	status: 'active',
	seats_included: 25,
	seats_used: 7,
	seats_computed_at: null,
	period_start: null,
	period_end
});

describe('toKitBilling', () => {
	it('with no billing account, reads Free (D46) — the live seat count, no renewal, no invoice', () => {
		const out = toKitBilling({ account: null, usage: usage(3) });
		expect(out.seatsActive).toBe(3);
		expect(out.plan).toBe('Free');
		expect(out.renews).toBeNull();
		expect(out.seatsReadonly).toBeNull();
		expect(out.invoices).toEqual([]);
		expect(out.tiers.filter((t) => t.current).map((t) => t.id)).toEqual(['free']);
	});

	it("the Org plan's price is the catalog's, the same for every org", () => {
		expect(toKitBilling({ account: null, usage: usage(3) }).perSeat).toBe(ORG_PRICE_PER_CONTRIBUTOR);
		expect(ORG_PRICE_PER_CONTRIBUTOR).toBe(12);
	});

	it('takes the plan, its renewal and the current tier from the account', () => {
		const out = toKitBilling({ account: account('org', '2026-08-01T00:00:00Z'), usage: usage(7) });
		expect(out.seatsActive).toBe(7);
		expect(out.plan).toBe('Org');
		expect(out.renews).toBe('Aug 1');
		expect(out.tiers.filter((t) => t.current).map((t) => t.id)).toEqual(['org']);
		expect(out.invoices).toEqual([]);
	});

	it('an account on the old Team plan reads as Org — the same plan, renamed', () => {
		const out = toKitBilling({ account: account('team', null), usage: usage(2) });
		expect(out.plan).toBe('Org');
		expect(out.tiers.filter((t) => t.current).map((t) => t.id)).toEqual(['org']);
	});

	it('an account with no period end has no renewal date', () => {
		expect(toKitBilling({ account: account('free', null), usage: usage(0) }).renews).toBeNull();
	});

	it('keeps the product catalog whatever the account: Free and Org', () => {
		const out = toKitBilling({ account: null, usage: usage(0) });
		expect(out.tiers.map((t) => t.id)).toEqual(BILLING_TIERS.map((t) => t.id));
		expect(BILLING_TIERS.map((t) => t.id)).toEqual(['free', 'org']);
	});
});

describe('noBilling', () => {
	it('is the screen with nothing loaded: no plan, no seats, no invoices', () => {
		const out = noBilling();
		expect(out.plan).toBeNull();
		expect(out.seatsActive).toBeNull();
		expect(out.invoices).toEqual([]);
	});
});
