// Unit tests for toKitBilling — the live-usage overlay onto the billing fixture.
import { describe, it, expect } from 'vitest';
import { toKitBilling } from './dojo2-billing-map';
import type { KitBilling } from './components/kit/types';
import type { BillingResponse } from './admin-data';

const fixture: KitBilling = {
	plan: 'Team · private',
	perSeat: 12,
	seatsActive: 34,
	seatsReadonly: 14,
	renews: 'Aug 1',
	// Empty catalog arrays — the tests assert these are preserved by reference
	// (the live overlay never touches the pricing catalog / invoice history).
	tiers: [],
	relayRows: [],
	invoices: []
};

const usage = (seats_used: number): BillingResponse['usage'] => ({
	seats_used,
	total_active_seats: seats_used,
	billable_users: []
});

describe('toKitBilling', () => {
	it('overlays the live seat count and keeps the catalog when there is no account', () => {
		const out = toKitBilling(fixture, { account: null, usage: usage(3) });
		expect(out.seatsActive).toBe(3); // LIVE
		// catalog + plan untouched (no provider / account)
		expect(out.plan).toBe('Team · private');
		expect(out.perSeat).toBe(12);
		expect(out.tiers).toBe(fixture.tiers);
		expect(out.invoices).toBe(fixture.invoices);
		expect(out.seatsReadonly).toBe(14);
	});

	it('overlays plan + renewal from a billing account', () => {
		const out = toKitBilling(fixture, {
			account: {
				plan: 'team',
				status: 'active',
				seats_included: 25,
				seats_used: 7,
				seats_computed_at: null,
				period_start: null,
				period_end: '2026-08-01T00:00:00Z'
			},
			usage: usage(7)
		});
		expect(out.seatsActive).toBe(7);
		expect(out.plan).toBe('Team'); // capitalized raw plan key
		expect(out.renews).toBe('Aug 1'); // period_end → short date
		// pricing catalog still fixture
		expect(out.perSeat).toBe(12);
	});

	it('keeps the fixture renewal when the account has no period_end', () => {
		const out = toKitBilling(fixture, {
			account: {
				plan: 'free',
				status: 'active',
				seats_included: 0,
				seats_used: 0,
				seats_computed_at: null,
				period_start: null,
				period_end: null
			},
			usage: usage(0)
		});
		expect(out.plan).toBe('Free');
		expect(out.renews).toBe('Aug 1');
		expect(out.seatsActive).toBe(0);
	});
});
