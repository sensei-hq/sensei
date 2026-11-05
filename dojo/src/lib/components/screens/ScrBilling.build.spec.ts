// billing build (Design) — ScrBilling as the approved mockup draws it
// (coord/mockups/L4-billing-r3; reviews/Design/L4-billing-mockup-r3.md M1–M4, L1–L4),
// on rokkit's ScreenShell (D87; L0 ruling 2026-11-04: adopt now, view id dojo.org.billing).
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { tick } from 'svelte';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import ScrBilling from './ScrBilling.svelte';
import { toKitBilling, noBilling } from '$lib/billing-map';
import type { BillingResponse } from '$lib/admin-data';

type Usage = BillingResponse['usage'];
const kit = (account: BillingResponse['account'], u: Usage) => toKitBilling({ account, usage: u }, NOW);

const NOW = new Date('2026-11-04T00:00:00Z');
const usage: Usage = {
	seats_used: 2,
	total_active_seats: 4,
	billable_users: [
		{ user_id: '7d3c9a10-0000-0000-0000-000000000001', display_name: 'Alex Doe', email: 'alex@example.com', workspaces: [{ name: 'ws-alpha', slug: 'ws-alpha', role: 'contributor' }, { name: 'ws-beta', slug: 'ws-beta', role: 'maintainer' }] },
		{ user_id: '0b1e2f30-0000-0000-0000-000000000002', display_name: null, email: null, workspaces: [{ name: 'ws-gamma', slug: 'ws-gamma', role: 'contributor' }] }
	]
};
const acct = (status: string, plan = 'pro'): NonNullable<BillingResponse['account']> => ({
	plan, status, seats_included: 0, seats_used: 0, seats_computed_at: null, period_start: null, period_end: '2026-12-01T00:00:00Z'
});
const screen = (account: BillingResponse['account'], u: Usage = usage) =>
	render(ScrBilling, { props: { orgName: 'acme', billing: kit(account, u) } });
const tagOf = (c: HTMLElement, tier: string) => (c.querySelector(`[data-tier="${tier}"] [data-plan-tag]`) as HTMLElement | null)?.textContent?.trim() ?? null;

describe('ScrBilling · billing build · the screen shell (D87)', () => {
	afterEach(cleanup);

	it('renders through ScreenShell with view id dojo.org.billing: mark, title, org and plan as context, description', () => {
		const { container } = screen(acct('active'));
		const shell = container.querySelector('[data-screen-shell]') as HTMLElement;
		expect(shell.dataset.view).toBe('dojo.org.billing');
		expect(shell.querySelector('[data-screen-mark]')?.textContent).toBe('円');
		expect(shell.querySelector('[data-screen-title]')?.textContent).toBe('Plan & billing');
		expect(shell.querySelector('[data-screen-context]')?.textContent).toBe('acme · Pro');
		expect(shell.querySelector('[data-screen-description]')?.textContent).toMatch(/public repositories/i);
	});

	it('an unknown plan key reads "Plan unknown" in the context and hides the plan cards (L6)', () => {
		const { container } = screen(acct('active', 'enterprise'));
		expect(container.querySelector('[data-screen-context]')?.textContent).toBe('acme · Plan unknown');
		expect(container.querySelector('[data-tier]')).toBeNull();
	});

	// The first-session look: framing column (eyebrow, kanji, title, lead), steps, a stat column.
	it('the info band is the first-session look on the accent wash', () => {
		const { container } = screen(null);
		const info = container.querySelector('[data-screen-info]') as HTMLElement;
		expect(info.dataset.role).toBe('accent');
		const cols = [...info.querySelectorAll('[data-screen-info-column]')];
		expect(cols.length).toBe(3);
		expect(cols[0].querySelector('[data-screen-info-label]')).toBeTruthy();
		expect(cols[0].querySelector('[data-screen-info-title]')).toBeTruthy();
		expect(cols[1].querySelectorAll('[data-screen-info-step]').length).toBeGreaterThanOrEqual(2);
		expect(cols[2].querySelector('[data-screen-info-value]')?.textContent).toBe('2');
	});

	it('search sits in the filter band and narrows "Who is counted" by name or workspace', async () => {
		const { container } = screen(null);
		const input = container.querySelector('[data-screen-filters] [data-screen-search] input') as HTMLInputElement;
		expect(input.placeholder).toBe('Search who is counted');
		await fireEvent.input(input, { target: { value: 'gamma' } });
		await new Promise((r) => setTimeout(r, 300));
		await tick();
		const items = [...container.querySelectorAll('details[data-billing-counted] li')].map((li) => li.textContent?.replace(/\s+/g, ' ').trim());
		expect(items).toEqual(['Name not shared yet — ws-gamma (contributor)']);
	});
});

// WP8 (handoff; reviews/L3/wp8-tests-r1.md For L0): no account reads Free; a FAILED read is an
// error and never "Free" (no fabricated plan).
describe('ScrBilling · billing build · WP8', () => {
	afterEach(cleanup);

	it('billing_reads_free_with_no_account', () => {
		const { container } = screen(null);
		expect(container.querySelector('[data-screen-context]')?.textContent).toBe('acme · Free');
		expect(tagOf(container, 'free')).toBe('Current plan');
	});

	it.each([
		['a refusal (403)', { status: 403, message: 'admins only' }],
		['an unreachable dōjō', { status: 0, message: 'could not reach the dojo service' }],
		['a server error', { status: 500, message: 'boom' }]
	] as const)('a failed billing read (%s) is an error state, never "Free"', (_label, error) => {
		const { container } = render(ScrBilling, { props: { orgName: 'acme', billing: noBilling(), error } });
		expect(container.querySelector('[data-load-error]')).toBeTruthy();
		expect(container.querySelector('[data-screen-context]')?.textContent).toBe('acme');
		expect(container.querySelector('[data-plan-tag]')).toBeNull();
		expect(container.querySelector('[data-tier]')).toBeNull();
		expect(container.textContent).not.toMatch(/\bFree\b/);
	});
	// An error over a plan that WAS loaded still claims no plan: the context is the org alone
	// and nothing from the earlier read is drawn under the error (D83).
	it('a failed read over a loaded Pro plan claims no plan either', () => {
		const { container } = render(ScrBilling, { props: { orgName: 'acme', billing: kit(acct('active'), usage), error: { status: 403, message: 'admins only' } } });
		expect(container.querySelector('[data-load-error]')).toBeTruthy();
		expect(container.querySelector('[data-screen-context]')?.textContent).toBe('acme');
		expect(container.querySelector('[data-tier]')).toBeNull();
		expect(container.querySelector('[data-plan-tag]')).toBeNull();
		expect(container.querySelector('[data-billing-problem]')).toBeNull();
	});
});

describe('ScrBilling · billing build · the plan tab', () => {
	afterEach(cleanup);

	// M2: the tag is the CTA pair, bg-primary text-on-primary — not bg-ink, not an accent tint.
	it('the current plan carries its tag on the primary pair; the other plan has none', () => {
		const { container } = screen(null);
		const tag = container.querySelector('[data-tier="free"] [data-plan-tag]') as HTMLElement;
		expect(tag.textContent?.trim()).toBe('Current plan');
		expect(tag.className).toMatch(/\bbg-primary\b/);
		expect(tag.className).toMatch(/\btext-on-primary\b/);
		expect(tagOf(container, 'pro')).toBeNull();
	});

	// L1: the tag per state; Free is untagged when Pro is current.
	it.each([
		['active', 'Current plan'],
		['trialing', 'Trial'],
		['past_due', 'Past due']
	] as const)('Pro %s: the Pro card reads "%s", Free is untagged', (status, tag) => {
		const { container } = screen(acct(status));
		expect(tagOf(container, 'pro')).toBe(tag);
		expect(tagOf(container, 'free')).toBeNull();
	});

	it('past due: the refusal says D88 in full and nothing offers to attach Pro (M1, L3)', () => {
		const { container } = screen(acct('past_due'));
		const n = container.querySelector('[data-billing-problem] [data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('refusal');
		expect(n.textContent).toContain('They come back as they were once it is paid.');
		expect(n.textContent).toContain('Settle the payment once payments are connected.');
		expect(container.querySelector('[data-action="attach-pro"]')).toBeNull();
	});

	it('Pro active: a quiet "Pro is attached" notice, with no Repositories-tab clause (L4)', () => {
		const { container } = screen(acct('active'));
		const n = container.querySelector('[data-billing-problem] [data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('normal');
		expect(n.textContent).toContain('Pro is attached');
		expect(n.textContent).not.toMatch(/Repositories tab|in Repositories/);
	});

	it('a plan button is disabled and described by the note that says why', () => {
		const { container } = screen(null);
		const attach = container.querySelector('[data-action="attach-pro"]') as HTMLButtonElement;
		expect(attach.disabled).toBe(true);
		const id = attach.getAttribute('aria-describedby');
		expect(id).toBeTruthy();
		expect(container.querySelector('#' + id)?.textContent).toContain('Plan changes open once payments are connected');
	});

	it('seats: the count, its basis, and the proposed count marked as such', () => {
		const { container } = screen(null);
		const seats = container.querySelector('[data-billing-seats]') as HTMLElement;
		expect(seats.textContent).toContain('active relay seats on private workspaces');
		expect(seats.textContent).toContain('What the dōjō counts today: people with an active relay session on a private workspace.');
		const proposed = container.querySelector('[data-billing-seats-proposed]') as HTMLElement;
		expect(proposed.textContent).toContain('—');
		expect(proposed.textContent).toContain('Proposed — not in code.');
	});

	// L9's rule (L0 ruling r3): personLabel — name, then email, then words; never an id fragment.
	it('"Who is counted" lists each seat by personLabel (name, email or words, never an id fragment), role per workspace, and the built remedy (M3)', () => {
		const { container } = screen(null);
		const who = container.querySelector('details[data-billing-counted]') as HTMLDetailsElement;
		expect(who.querySelector('summary')?.textContent).toContain('Who is counted');
		const items = [...who.querySelectorAll('li')].map((li) => li.textContent?.replace(/\s+/g, ' ').trim());
		expect(items).toEqual([
			'Alex Doe — ws-alpha (contributor), ws-beta (maintainer)',
			'Name not shared yet — ws-gamma (contributor)'
		]);
		expect(who.textContent).toContain("Closing a seat isn't built yet.");
		expect(who.textContent).not.toMatch(/0b1e2f30|7d3c9a10/);
		// No invented handle: the only "@" is a real email, never "@word" on its own.
		expect(who.textContent?.replace(/[\w.+-]+@[\w-]+\.[\w.]+/g, '')).not.toMatch(/@\w/);
	});

	it('a seat with no name but an email reads the email', () => {
		const emailOnly: Usage = {
			seats_used: 1,
			total_active_seats: 1,
			billable_users: [{ user_id: '5c6d7e80-0000-0000-0000-000000000003', display_name: null, email: 'sam@example.com', workspaces: [{ name: 'ws-delta', slug: 'ws-delta', role: 'contributor' }] }]
		};
		const { container } = screen(null, emailOnly);
		const items = [...container.querySelectorAll('details[data-billing-counted] li')].map((li) => li.textContent?.replace(/\s+/g, ' ').trim());
		expect(items).toEqual(['sam@example.com — ws-delta (contributor)']);
	});

	it('with no seats, "Who is counted" says nobody is, not an empty list', () => {
		const { container } = screen(null, { seats_used: 0, total_active_seats: 0, billable_users: [] });
		const who = container.querySelector('details[data-billing-counted]') as HTMLElement;
		expect(who.querySelectorAll('li').length).toBe(0);
		expect(who.textContent).toContain('No one has a seat on a private workspace.');
	});

	// M2: tokens only — no literal colour in the screen's source (a guard: it passes today).
	it('no literal colour in ScrBilling', () => {
		const src = readFileSync(join(process.cwd(), 'src/lib/components/screens/ScrBilling.svelte'), 'utf8');
		// 6/8-digit hex, an arbitrary-value class, or a colour function (issue refs like #106 are fine).
		expect(src).not.toMatch(/#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?\b|-\[#|rgba?\(|oklch\(|hsla?\(/);
		expect(src).not.toMatch(/\bbg-ink\b/);
	});

	// L3 (r2): the data-screen-* assertions alone pass for a hand-rolled div; the shell must be rokkit's.
	// L1 (r3): an unused import passes the import check alone — it must also render the shell.
	it('ScrBilling imports ScreenShell from @rokkit/app and renders it', () => {
		const src = readFileSync(join(process.cwd(), 'src/lib/components/screens/ScrBilling.svelte'), 'utf8');
		expect(src).toMatch(/import\s*\{[^}]*\bScreenShell\b[^}]*\}\s*from\s*['"]@rokkit\/app['"]/);
		expect(src).toMatch(/<ScreenShell\b/);
	});
});
