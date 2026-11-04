// billing build (Design, phase A) — ScrBilling as the approved mockup draws it
// (coord/mockups/L4-billing-r3; reviews/Design/L4-billing-mockup-r3.md M1–M4, L1–L4).
import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import ScrBilling from './ScrBilling.svelte';
import { toKitBilling } from '$lib/billing-map';
import type { BillingResponse } from '$lib/admin-data';

const NOW = new Date('2026-11-04T00:00:00Z');
const usage: BillingResponse['usage'] = {
	seats_used: 2,
	total_active_seats: 4,
	billable_users: [
		{ user_id: '7d3c9a10-0000-0000-0000-000000000001', display_name: 'Priya K', workspaces: [{ name: 'platform-core', slug: 'platform-core', role: 'contributor' }, { name: 'delivery-portal', slug: 'delivery-portal', role: 'maintainer' }] },
		{ user_id: '0b1e2f30-0000-0000-0000-000000000002', display_name: null, workspaces: [{ name: 'mobile-app', slug: 'mobile-app', role: 'contributor' }] }
	]
};
const acct = (status: string, plan = 'pro'): NonNullable<BillingResponse['account']> => ({
	plan, status, seats_included: 0, seats_used: 0, seats_computed_at: null, period_start: null, period_end: '2026-12-01T00:00:00Z'
});
const screen = (account: BillingResponse['account']) =>
	render(ScrBilling, { props: { orgName: 'senecaglobal', billing: toKitBilling({ account, usage }, NOW) } });

describe('ScrBilling · billing build', () => {
	afterEach(cleanup);

	// M2: the ink-filled tag is the CTA pair, bg-primary text-on-primary — not bg-ink, not an accent tint.
	it.skip('pending: billing-build — the current plan carries a "Current plan" tag on the primary pair', () => {
		const { container } = screen(null);
		const tag = container.querySelector('[data-tier="free"] [data-plan-tag]') as HTMLElement;
		expect(tag.textContent?.trim()).toBe('Current plan');
		expect(tag.className).toMatch(/\bbg-primary\b/);
		expect(tag.className).toMatch(/\btext-on-primary\b/);
		expect(container.querySelector('[data-tier="pro"] [data-plan-tag]')).toBeNull();
	});

	it.skip('pending: billing-build — past due: the Pro card is tagged "Past due" and the refusal says D88 in full (M1, L3)', () => {
		const { container } = screen(acct('past_due'));
		expect((container.querySelector('[data-tier="pro"] [data-plan-tag]') as HTMLElement).textContent?.trim()).toBe('Past due');
		const n = container.querySelector('[data-billing-problem] [data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('refusal');
		expect(n.textContent).toContain('They come back as they were once it is paid.');
		expect(n.textContent).toContain('Settle the payment once payments are connected.');
		// Pro is attached, so nothing offers to attach it.
		expect(container.querySelector('[data-action="attach-pro"]')).toBeNull();
	});

	it.skip('pending: billing-build — Pro active: a quiet "Pro is attached" notice, no tag on Pro', () => {
		const { container } = screen(acct('active'));
		const n = container.querySelector('[data-billing-problem] [data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('normal');
		expect(n.textContent).toContain('Pro is attached');
		expect(n.textContent).not.toMatch(/Repositories tab|in Repositories/);
	});

	it.skip('pending: billing-build — a plan button is disabled and described by the note that says why', () => {
		const { container } = screen(null);
		const attach = container.querySelector('[data-action="attach-pro"]') as HTMLButtonElement;
		expect(attach.disabled).toBe(true);
		const id = attach.getAttribute('aria-describedby');
		expect(id).toBeTruthy();
		expect(container.querySelector('#' + id)?.textContent).toContain('Plan changes open once payments are connected');
	});

	it.skip('pending: billing-build — seats: the count, its basis, and the proposed count marked as such', () => {
		const { container } = screen(null);
		const seats = container.querySelector('[data-billing-seats]') as HTMLElement;
		expect(seats.textContent).toContain('active relay seats on private workspaces');
		expect(seats.textContent).toContain('What the dōjō counts today: people with an active relay session on a private workspace.');
		const proposed = container.querySelector('[data-billing-seats-proposed]') as HTMLElement;
		expect(proposed.textContent).toContain('—');
		expect(proposed.textContent).toContain('Proposed — not in code.');
	});

	it.skip('pending: billing-build — "Who is counted" lists each seat by its joined name (or short id), role per workspace, and the built remedy (M3)', () => {
		const { container } = screen(null);
		const who = container.querySelector('details[data-billing-counted]') as HTMLDetailsElement;
		expect(who.querySelector('summary')?.textContent).toContain('Who is counted');
		const items = [...who.querySelectorAll('li')].map((li) => li.textContent?.replace(/\s+/g, ' ').trim());
		expect(items).toEqual([
			'Priya K — platform-core (contributor), delivery-portal (maintainer)',
			'0b1e2f30 — mobile-app (contributor)'
		]);
		expect(who.textContent).toContain("Closing a seat isn't built yet.");
		expect(who.textContent).not.toMatch(/@\w/);
	});

	it.skip('pending: billing-build — with no seats, "Who is counted" says nobody is, not an empty list', () => {
		const { container } = render(ScrBilling, {
			props: { orgName: 'senecaglobal', billing: toKitBilling({ account: null, usage: { seats_used: 0, total_active_seats: 0, billable_users: [] } }, NOW) }
		});
		const who = container.querySelector('details[data-billing-counted]') as HTMLElement;
		expect(who.querySelectorAll('li').length).toBe(0);
		expect(who.textContent).toContain('No one has a seat on a private workspace.');
	});

	// M2: tokens only — no literal colour in the screen's source.
	it.skip('pending: billing-build — no literal colour in ScrBilling', () => {
		const src = readFileSync(join(process.cwd(), 'src/lib/components/screens/ScrBilling.svelte'), 'utf8');
		// 6/8-digit hex, an arbitrary-value class, or a colour function (issue refs like #106 are fine).
		expect(src).not.toMatch(/#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b|-\[#|rgba?\(|oklch\(|hsla?\(/);
		expect(src).not.toMatch(/\bbg-ink\b/);
	});
});
