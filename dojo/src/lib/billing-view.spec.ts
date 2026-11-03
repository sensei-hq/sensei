import { describe, expect, it } from 'vitest';
import { monthlyTotal, attachNotice, tierAction } from './billing-view';

// The admin billing helpers — the estimate math, the tier actions and what the
// screen says about attaching a plan. Pure functions, so they assert without a DOM.

describe('monthlyTotal — active contributors × per-seat', () => {
	it('multiplies billable seats by the per-seat price', () => {
		expect(monthlyTotal({ seatsActive: 18, perSeat: 12 })).toBe(216);
	});

	it('is zero when there are no billable seats', () => {
		expect(monthlyTotal({ seatsActive: 0, perSeat: 12 })).toBe(0);
		// No price set, or no seat count: there is no total to show (#106).
		expect(monthlyTotal({ seatsActive: 34, perSeat: null })).toBeNull();
		expect(monthlyTotal({ seatsActive: null, perSeat: 12 })).toBeNull();
	});
});

describe('tierAction — each plan card\'s button', () => {
	it('the current plan has none', () => {
		expect(tierAction('free', 'free')).toBeNull();
		expect(tierAction('org', 'org')).toBeNull();
	});

	it('on Free, Org offers to attach; on Org, Free offers to move back', () => {
		expect(tierAction('org', 'free')).toEqual({ label: 'Attach Org plan', primary: true });
		expect(tierAction('free', 'org')).toEqual({ label: 'Move to Free', primary: false });
	});
});

describe('attachNotice — what the screen says about changing plan', () => {
	it('says plan changes open once payments are connected — nothing is charged or attached yet', () => {
		expect(attachNotice()).toEqual({
			tone: 'normal',
			title: 'Plan changes open once payments are connected',
			detail: 'Until then every organization is on Free: its public repositories sync, and its private ones stay hidden.',
			code: 'billing_not_connected',
		});
	});
});
