// Unit tests for toKitBilling — the Plan & billing screen built from the live
// `…/billing` response alone (#106). It used to overlay that response on the
// kit fixture, so an org with no billing account showed the fixture's plan,
// renewal, $12 seat price and three paid invoices as its own.
import { describe, it, expect } from 'vitest';
import { toKitBilling, noBilling } from './billing-map';
import { BILLING_TIERS } from './billing-catalog';
import type { BillingResponse } from './admin-data';

const usage = (seats_used: number): BillingResponse['usage'] => ({
	seats_used,
	total_active_seats: seats_used,
	billable_users: []
});

const account = (plan: string, period_end: string | null, status = 'active'): NonNullable<BillingResponse['account']> => ({
	plan,
	status,
	seats_included: 25,
	seats_used: 7,
	seats_computed_at: null,
	period_start: null,
	period_end
});

const NOW = new Date('2026-07-15T00:00:00Z');

describe('toKitBilling', () => {
	it('with no billing account, reads Free (D46) — the live seat count, no renewal, no invoice, no price', () => {
		const out = toKitBilling({ account: null, usage: usage(3) }, NOW);
		expect(out.seatsActive).toBe(3);
		expect(out.plan).toBe('Free');
		expect(out.renews).toBeNull();
		expect(out.seatsReadonly).toBeNull();
		expect(out.invoices).toEqual([]);
		expect(out.problem).toBeNull();
		expect('perSeat' in out).toBe(false);
		expect(out.tiers.filter((t) => t.current).map((t) => t.id)).toEqual(['free']);
	});

	it('an active Pro account inside its period: Pro, its renewal, current', () => {
		const out = toKitBilling({ account: account('pro', '2026-08-01T00:00:00Z'), usage: usage(7) }, NOW);
		expect(out.plan).toBe('Pro');
		expect(out.renews).toBe('Aug 1');
		expect(out.problem).toBeNull();
		expect(out.tiers.filter((t) => t.current).map((t) => t.id)).toEqual(['pro']);
	});

	it('the old plan keys "org" and "team" read as Pro (D75)', () => {
		for (const key of ['org', 'team']) {
			expect(toKitBilling({ account: account(key, '2026-08-01T00:00:00Z'), usage: usage(2) }, NOW).plan).toBe('Pro');
		}
	});

	it('trialing counts as current, as all_my_repositories does', () => {
		expect(toKitBilling({ account: account('pro', '2026-08-01T00:00:00Z', 'trialing'), usage: usage(2) }, NOW).plan).toBe('Pro');
	});

	// M5 (r1): the screen and all_my_repositories must agree — a lapsed account is
	// not current, so its private repositories are hidden, and the screen says so.
	it('a canceled or past-due account has lapsed: Free is current, and a refusal says why', () => {
		const out = toKitBilling({ account: account('pro', '2026-08-01T00:00:00Z', 'canceled'), usage: usage(2) }, NOW);
		expect(out.plan).toBe('Free');
		expect(out.tiers.filter((t) => t.current).map((t) => t.id)).toEqual(['free']);
		expect(out.renews).toBeNull();
		expect(out.problem).toEqual({
			tone: 'refusal',
			title: 'The Pro plan is not active',
			detail: "The billing account is canceled, so this organization's private repositories stay hidden.",
			remedy: 'Plan changes open once payments are connected.'
		});
	});

	it('an account past its period end has lapsed too', () => {
		const out = toKitBilling({ account: account('pro', '2026-07-01T00:00:00Z'), usage: usage(2) }, NOW);
		expect(out.plan).toBe('Free');
		expect(out.problem?.detail).toBe("The billing period ended Jul 1, so this organization's private repositories stay hidden.");
	});

	it('a plan key sensei does not know is an error, never Free', () => {
		const out = toKitBilling({ account: account('enterprise', null), usage: usage(2) }, NOW);
		expect(out.plan).toBeNull();
		expect(out.tiers.some((t) => t.current)).toBe(false);
		expect(out.problem).toEqual({
			tone: 'fault',
			title: "This dōjō's plan isn't one sensei knows",
			detail: 'The billing account says "enterprise".',
			remedy: 'Update Sensei. If it still shows, the billing account needs fixing.'
		});
	});

	it('an account with no period end has no renewal date', () => {
		expect(toKitBilling({ account: account('free', null), usage: usage(0) }, NOW).renews).toBeNull();
	});
});

describe('the catalog (D52, D59, D75)', () => {
	const pro = BILLING_TIERS.find((t) => t.id === 'pro')!;
	it('two plans: Free and Pro', () => {
		expect(BILLING_TIERS.map((t) => [t.id, t.name])).toEqual([['free', 'Free'], ['pro', 'Pro']]);
	});

	it('the price is to be announced — no amount anywhere (D75)', () => {
		expect(pro.price).toBe('To be announced');
		for (const t of BILLING_TIERS) expect(`${t.price} ${t.sub} ${t.lines.join(' ')}`).not.toMatch(/\$\s?\d/);
	});

	it('Pro lists what it unlocks: targets on private repositories (D52)', () => {
		expect(pro.lines).toContain("Targets on an organization's private repositories");
	});

	it('Pro never lists what is free: SSO, SCIM, audit, the relay queue (D59, D77)', () => {
		for (const l of pro.lines) expect(l).not.toMatch(/audit|relay|inbox|SSO|SCIM/i);
		const free = BILLING_TIERS.find((t) => t.id === 'free')!;
		expect(free.lines.join(' ')).toMatch(/SSO, SCIM, audit/);
	});
});

describe('noBilling', () => {
	it('is the screen with nothing loaded: no plan, no seats, no invoices', () => {
		const out = noBilling();
		expect(out.plan).toBeNull();
		expect(out.seatsActive).toBeNull();
		expect(out.invoices).toEqual([]);
		expect(out.problem).toBeNull();
	});
});
