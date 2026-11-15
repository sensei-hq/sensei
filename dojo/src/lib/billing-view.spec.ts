import { describe, expect, it } from 'vitest';
import { attachNotice, tierAction, currentTier } from './billing-view';
import { toKitBilling } from './billing-map';

// The admin billing helpers — the plan actions, which plan is current, and what
// the screen says about changing plan. Pure, so they assert without a DOM.

describe('tierAction — each plan card\'s button', () => {
	it('the current plan has none', () => {
		expect(tierAction('free', 'free')).toBeNull();
		expect(tierAction('pro', 'pro')).toBeNull();
	});

	it('on Free, Pro offers to attach; on Pro, Free offers to move back', () => {
		expect(tierAction('pro', 'free')).toEqual({ label: 'Attach Pro', primary: true });
		expect(tierAction('free', 'pro')).toEqual({ label: 'Move to Free', primary: false });
	});

	it('with no known current plan, nothing is offered', () => {
		expect(tierAction('pro', null)).toBeNull();
	});
});

describe('currentTier', () => {
	it('the current tier, or null when the plan is unknown', () => {
		const usage = { seats_used: 0, total_active_seats: 0, billable_users: [] };
		expect(currentTier(toKitBilling({ account: null, usage }))).toBe('free');
		const unknown = toKitBilling({ account: { plan: 'x', status: 'active', seats_included: 0, seats_used: 0, seats_computed_at: null, period_start: null, period_end: null }, usage });
		expect(currentTier(unknown)).toBeNull();
	});
});

describe('attachNotice — why the plan buttons cannot run yet (M6)', () => {
	it('on Free: plan changes open once payments are connected, and what Free means meanwhile', () => {
		expect(attachNotice('free')).toEqual({
			tone: 'normal',
			title: 'Plan changes open once payments are connected',
			detail: "Until then this organization stays on Free: its public repositories sync, and its private ones stay hidden.",
			code: 'billing_not_connected'
		});
	});

	it('on Pro: the same, and the plan stays as it is', () => {
		expect(attachNotice('pro')).toEqual({
			tone: 'normal',
			title: 'Plan changes open once payments are connected',
			detail: 'Until then this plan stays as it is.',
			code: 'billing_not_connected'
		});
	});
});
