import { describe, expect, it } from 'vitest';
import { monthlyTotal, relayTone, tierCtaLabel } from './billing-view';

// The admin billing helpers — the live monthly-total math + the free-vs-paid
// relay tone + the tier CTA label. Pure functions, so they assert without a DOM.

describe('monthlyTotal — active contributors × per-seat', () => {
	it('multiplies billable seats by the per-seat price', () => {
		expect(monthlyTotal({ seatsActive: 34, perSeat: 12 })).toBe(408);
	});

	it('is zero when there are no billable seats', () => {
		expect(monthlyTotal({ seatsActive: 0, perSeat: 12 })).toBe(0);
	});
});

describe('relayTone — free vs paid', () => {
	it('reads a free row as success with the individuals label', () => {
		const t = relayTone(true);
		expect(t.text).toBe('text-success');
		expect(t.soft).toBe('bg-success-soft');
		expect(t.label).toBe('free · individuals');
	});

	it('reads a paid row as accent with the team label', () => {
		const t = relayTone(false);
		expect(t.text).toBe('text-accent');
		expect(t.label).toBe('paid · team');
	});
});

describe('tierCtaLabel', () => {
	it('offers a downgrade to the free tier', () => {
		expect(tierCtaLabel('free')).toBe('Downgrade');
	});

	it('routes paid tiers to sales', () => {
		expect(tierCtaLabel('ent')).toBe('Contact sales');
		expect(tierCtaLabel('team')).toBe('Contact sales');
	});
});
