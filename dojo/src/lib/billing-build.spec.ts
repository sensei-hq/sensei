// billing build (Design, phase A r2) — the Plan & billing tab as the approved mockup
// draws it (coord/mockups/L4-billing-r3, reviews/Design/L4-billing-mockup-r3.md).
// Pure: billing-map, the catalog, and the seat breakdown, without a DOM.
//
// Acceptance items pinned here: M1 (D88 in full for past due and lapse), M3 (who
// is counted: names from a join, never invented; role per workspace; the seat
// remedy says only what is built), L3 (past due keeps Pro with a tag; the unknown
// key says the mockup's copy), L2 ("organization", never "organisation").
import { describe, it, expect } from 'vitest';
import { toKitBilling, noBilling } from './billing-map';
import { BILLING_TIERS } from './billing-catalog';
import * as view from './billing-view';
import type { BillingResponse } from './admin-data';

// ── test-local shapes the build adds; phase B deletes these and uses the real types ──
type Tone = 'normal' | 'refusal' | 'fault';
type Built = Omit<ReturnType<typeof toKitBilling>, 'problem'> & {
	tag: 'Current plan' | 'Trial' | 'Past due' | null;
	problem: { tone: Tone; title: string; detail: string; remedy: string; code: string } | null;
};
type Named = { user_id: string; display_name: string | null; workspaces: { name: string; slug: string; role: string }[] };
type Pending = {
	countedSeats: (users: Named[]) => { name: string; workspaces: string }[];
	SEAT_COPY: Record<string, unknown>;
};
const { countedSeats, SEAT_COPY } = view as unknown as typeof view & Pending;
const built = (res: BillingResponse, now: Date): Built => toKitBilling(res, now) as unknown as Built;

const usage = (n = 3): BillingResponse['usage'] => ({ seats_used: n, total_active_seats: n, billable_users: [] });
const account = (plan: string, status: string, period_end: string | null = '2026-12-01T00:00:00Z'): NonNullable<BillingResponse['account']> => ({
	plan,
	status,
	seats_included: 0,
	seats_used: 0,
	seats_computed_at: null,
	period_start: null,
	period_end
});
const NOW = new Date('2026-11-04T00:00:00Z');
const current = (b: Built) => b.tiers.filter((t) => t.current).map((t) => t.id);

describe('billing build · each account state says what it means (mockup r3)', () => {
	it.skip('pending: billing-build — no account: Free, tagged "Current plan", nothing to say', () => {
		const b = built({ account: null, usage: usage() }, NOW);
		expect(current(b)).toEqual(['free']);
		expect(b.tag).toBe('Current plan');
		expect(b.problem).toBeNull();
	});

	it.skip('pending: billing-build — Pro active: Pro is attached, renews, nothing to do (L4: no Repositories-tab clause)', () => {
		const b = built({ account: account('pro', 'active'), usage: usage() }, NOW);
		expect(current(b)).toEqual(['pro']);
		expect(b.renews).toBe('Dec 1');
		expect(b.problem).toEqual({
			tone: 'normal',
			title: 'Pro is attached',
			detail: "Private repositories are on and sync at each member's next sync pass. Renews Dec 1.",
			remedy: 'Nothing to do.',
			code: 'active'
		});
	});

	// L1: the current plan's tag, per state.
	it.skip.each([
		['active', 'Current plan'],
		['trialing', 'Trial'],
		['past_due', 'Past due']
	] as const)('pending: billing-build — Pro %s: Pro is current and tagged "%s"', (status, tag) => {
		const b = built({ account: account('pro', status), usage: usage() }, NOW);
		expect(current(b)).toEqual(['pro']);
		expect(b.tag).toBe(tag);
	});

	// L4: a trial ends; it never "renews".
	it.skip('pending: billing-build — trial: its end date in the title, no renewal', () => {
		const b = built({ account: account('pro', 'trialing', '2026-11-20T00:00:00Z'), usage: usage() }, NOW);
		expect(b.renews).toBeNull();
		expect(b.problem).toEqual({
			tone: 'normal',
			title: 'Pro trial — ends Nov 20',
			detail: 'Private repositories sync during the trial.',
			remedy: 'Nothing to do now. Plan changes open once payments are connected.',
			code: 'trialing'
		});
	});

	// L3: past due is still Pro (billing_accounts.plan); M1: D88 in full — hidden at once, back as they were.
	it.skip('pending: billing-build — past due: Pro stays current; hidden now, back as they were once paid', () => {
		const b = built({ account: account('pro', 'past_due'), usage: usage() }, NOW);
		expect(b.plan).toBe('Pro');
		expect(b.renews).toBeNull();
		expect(b.problem).toEqual({
			tone: 'refusal',
			title: 'Pro payment is past due',
			detail: "This organization's private repositories are hidden until the payment is settled. They come back as they were once it is paid.",
			remedy: 'Settle the payment once payments are connected.',
			code: 'not_subscribed'
		});
	});

	// L2: all_my_repositories checks the status before the period, so a past-due account
	// whose period has also ended is still past due.
	it.skip('pending: billing-build — past due with an ended period stays past due (the view\'s order)', () => {
		const b = built({ account: account('pro', 'past_due', '2026-11-01T00:00:00Z'), usage: usage() }, NOW);
		expect(current(b)).toEqual(['pro']);
		expect(b.tag).toBe('Past due');
		expect(b.problem?.code).toBe('not_subscribed');
	});

	it.skip('pending: billing-build — period ended: Free is current; hidden again, back as they were (D88), code subscription_expired', () => {
		const b = built({ account: account('pro', 'active', '2026-11-01T00:00:00Z'), usage: usage() }, NOW);
		expect(current(b)).toEqual(['free']);
		expect(b.tag).toBe('Current plan');
		expect(b.problem).toEqual({
			tone: 'refusal',
			title: 'Pro is not active',
			detail: "The billing period ended Nov 1, so this organization's private repositories are hidden again. They come back as they were once Pro is renewed.",
			remedy: 'Renew once payments are connected.',
			code: 'subscription_expired'
		});
	});

	it.skip('pending: billing-build — canceled: Free is current, the same promise and remedy, code not_subscribed', () => {
		const b = built({ account: account('pro', 'canceled'), usage: usage() }, NOW);
		expect(current(b)).toEqual(['free']);
		expect(b.problem).toEqual({
			tone: 'refusal',
			title: 'Pro is not active',
			detail: "The billing account is canceled, so this organization's private repositories are hidden again. They come back as they were once Pro is renewed.",
			remedy: 'Renew once payments are connected.',
			code: 'not_subscribed'
		});
	});

	// L6: the mockup hides the plan cards for an unknown key and shows "Plan unknown".
	it.skip('pending: billing-build — unknown key: the mockup\'s fault, no current plan, no tag, the cards hidden, "Plan unknown"', () => {
		const b = built({ account: account('enterprise', 'active', null), usage: usage() }, NOW) as Built & { badge: string | null; showPlans: boolean };
		expect(b.plan).toBeNull();
		expect(current(b)).toEqual([]);
		expect(b.tag).toBeNull();
		expect(b.badge).toBe('Plan unknown');
		expect(b.showPlans).toBe(false);
		expect(b.problem).toEqual({
			tone: 'fault',
			title: "This organization's plan could not be read",
			detail: 'The billing account says "enterprise", which isn\'t a plan sensei knows.',
			remedy: 'Ask whoever set up billing to correct the plan key. Until then the plan is unknown.',
			code: 'unknown_plan'
		});
	});

	// WP8: nothing loaded (a failed read) claims no plan — never "Free".
	it.skip('pending: billing-build — nothing loaded: no plan, no tag, no badge, no current card', () => {
		const b = noBilling() as unknown as Built & { badge: string | null };
		expect(b.plan).toBeNull();
		expect(b.tag).toBeNull();
		expect(b.badge).toBeNull();
		expect(current(b)).toEqual([]);
	});

	// L5: every state and the catalog.
	it.skip('pending: billing-build — every state and the catalog spell "organization" (L2)', () => {
		const states: BillingResponse['account'][] = [
			null,
			account('pro', 'active'),
			account('pro', 'trialing'),
			account('pro', 'past_due'),
			account('pro', 'active', '2026-11-01T00:00:00Z'),
			account('pro', 'canceled'),
			account('x', 'active')
		];
		for (const a of states) expect(JSON.stringify(built({ account: a, usage: usage() }, NOW))).not.toMatch(/organisation/i);
		expect(JSON.stringify(BILLING_TIERS)).not.toMatch(/organisation/i);
	});
});

describe('billing build · the plan cards are the mockup\'s (D46, D52, D59, D85, D86)', () => {
	const free = BILLING_TIERS.find((t) => t.id === 'free')!;
	const pro = BILLING_TIERS.find((t) => t.id === 'pro')!;

	it.skip('pending: billing-build — Free reads "No charge · for every organization", not "Free" twice', () => {
		expect([free.price, free.sub]).toEqual(['No charge', 'for every organization']);
	});

	it.skip('pending: billing-build — Free lists public repositories, personal dōjōs, warn targets, governance, public-sync control and the free admin features', () => {
		expect(free.lines).toEqual([
			"This organization's public repositories, on by default",
			"Members' personal dōjōs, private repositories and blocking targets included",
			'Targets that warn, on every repository',
			"Governance rules and standards sent to every member's sensei",
			'Admin control over which public repositories sync',
			'SSO, SCIM and the audit trail',
			"The relay's team on-call queue",
			'Unlimited members; viewers and contributors to public repositories are never counted'
		]);
	});

	it.skip('pending: billing-build — Pro lists only what it adds, scoped to private repositories (B3, L3)', () => {
		expect([pro.price, pro.sub]).toEqual(['To be announced', 'per seat on private repositories']);
		expect(pro.lines).toEqual([
			'Everything in Free',
			"This organization's private repositories, all on by default",
			'Blocking targets on those private repositories',
			'Integrations and consolidated metrics for those private repositories',
			'Admin control over which private repositories sync',
			'Options: extended audit retention and an SLA'
		]);
	});
});

describe('billing build · who is counted (M3)', () => {
	const users: Named[] = [
		{ user_id: '7d3c9a10-0000-0000-0000-000000000001', display_name: 'Alex Doe', workspaces: [{ name: 'ws-alpha', slug: 'ws-alpha', role: 'contributor' }, { name: 'ws-beta', slug: 'ws-beta', role: 'maintainer' }] },
		{ user_id: '0b1e2f30-0000-0000-0000-000000000002', display_name: null, workspaces: [{ name: 'ws-gamma', slug: 'ws-gamma', role: 'contributor' }] }
	];

	it.skip('pending: billing-build — a name comes from the identity join; with none, the short id, never an invented handle', () => {
		expect(countedSeats(users).map((u: { name: string }) => u.name)).toEqual(['Alex Doe', '0b1e2f30']);
	});

	it.skip('pending: billing-build — the role is shown per workspace', () => {
		expect(countedSeats(users)[0].workspaces).toBe('ws-alpha (contributor), ws-beta (maintainer)');
	});

	it.skip('pending: billing-build — the seat copy says only what is built: seats open on a relay session; closing is not built', () => {
		expect(SEAT_COPY).toEqual({
			label: 'active relay seats on private workspaces',
			basis: 'What the dōjō counts today: people with an active relay session on a private workspace.',
			remedy: "A seat opens when someone starts a relay session on a private workspace. Closing a seat isn't built yet.",
			proposed: { label: 'people working on private repositories', note: 'The count Pro is priced by (D59). Proposed — not in code.' }
		});
		expect(JSON.stringify(SEAT_COPY)).not.toMatch(/next refresh|drops? off/i);
	});
});
