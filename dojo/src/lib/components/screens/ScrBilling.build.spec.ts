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
import { NAV_ORG_BASE } from '$lib/nav';
import type { ComponentProps } from 'svelte';
import type { BillingHiddenOrg } from '$lib/billing-hidden';

type Usage = BillingResponse['usage'];
const kit = (account: BillingResponse['account'], u: Usage) => toKitBilling({ account, usage: u }, NOW);

const NOW = new Date('2026-11-04T00:00:00Z');
const usage: Usage = {
	seats_used: 2,
	total_active_seats: 4,
	billable_users: [
		{ user_id: '7d3c9a10-0000-0000-0000-000000000001', display_name: 'Alex Doe', handle: null, email: 'alex@example.com', workspaces: [{ name: 'ws-alpha', slug: 'ws-alpha', role: 'contributor' }, { name: 'ws-beta', slug: 'ws-beta', role: 'maintainer' }] },
		{ user_id: '0b1e2f30-0000-0000-0000-000000000002', display_name: null, handle: null, email: null, workspaces: [{ name: 'ws-gamma', slug: 'ws-gamma', role: 'contributor' }] }
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
		expect(shell.querySelector('[data-screen-mark]')?.textContent).toBe(navBilling.kanji);
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
			billable_users: [{ user_id: '5c6d7e80-0000-0000-0000-000000000003', display_name: null, handle: null, email: 'sam@example.com', workspaces: [{ name: 'ws-delta', slug: 'ws-delta', role: 'contributor' }] }]
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

// billing follow-up (user rulings 2026-11-05; reviews/Design/L4-billing-build-impl-r1.md L1, L2, L3).
type HiddenGroup = BillingHiddenOrg;
type NextProps = ComponentProps<typeof ScrBilling>;
const navBilling = NAV_ORG_BASE.flatMap((g) => g.items).find((i) => i.id === 'billing')!;
const lapsedAcct = (): NonNullable<BillingResponse['account']> => ({ ...acct('active'), period_end: '2026-11-01T00:00:00Z' });
const twoHidden: HiddenGroup = { org: 'acme', count: 2, repos: ['core', 'portal'] };
const withHidden = (account: BillingResponse['account'], hidden: HiddenGroup | null, extra: Partial<NextProps> = {}) =>
	render(ScrBilling, { props: { orgName: 'acme', billing: kit(account, usage), hidden, ...extra } });
const hiddenItems = (c: HTMLElement) => [...c.querySelectorAll('[data-billing-hidden] li')].map((li) => li.textContent?.trim());

describe('ScrBilling · billing follow-up', () => {
	afterEach(cleanup);

	// User ruling (1) / L2: one glyph for one destination — the nav's.
	it('the title mark is the nav\'s Plan & billing kanji (金), and 円 is gone from the screen', () => {
		const { container } = screen(acct('active'));
		expect(navBilling.kanji).toBe('金');
		expect(container.querySelector('[data-screen-shell] [data-screen-mark]')?.textContent).toBe(navBilling.kanji);
		const src = readFileSync(join(process.cwd(), 'src/lib/components/screens/ScrBilling.svelte'), 'utf8');
		expect(src).not.toContain('円');
	});

	// L1 (impl r1): over a failed read there is nothing for the info band to describe (a guard).
	it('no info band over a failed read, even over a loaded plan', () => {
		const { container } = render(ScrBilling, { props: { orgName: 'acme', billing: kit(acct('active'), usage), error: { status: 500, message: 'boom' } } });
		expect(container.querySelector('[data-load-error]')).toBeTruthy();
		expect(container.querySelector('[data-screen-info]')).toBeNull();
	});

	// L3 (impl r1): search acts on a VISIBLE list — a query opens "Who is counted".
	it('a search query opens "Who is counted", so the narrowed list is visible', async () => {
		const { container } = screen(null);
		const who = container.querySelector('details[data-billing-counted]') as HTMLDetailsElement;
		expect(who.open).toBe(false);
		const input = container.querySelector('[data-screen-filters] [data-screen-search] input') as HTMLInputElement;
		await fireEvent.input(input, { target: { value: 'gamma' } });
		await new Promise((r) => setTimeout(r, 300));
		await tick();
		expect(who.open).toBe(true);
	});

	// User ruling (2): admins see the org's hidden private repositories here — count, list and
	// the reinstatement wording for the account's state.
	it('past due: the hidden private repositories, counted and listed, back as they were once it is paid', () => {
		const { container } = withHidden(acct('past_due'), twoHidden);
		const sec = container.querySelector('[data-billing-hidden]') as HTMLElement;
		expect(sec.textContent).toContain('2 private repositories are hidden from everyone in this organization.');
		expect(hiddenItems(container)).toEqual(['core', 'portal']);
		expect(sec.textContent).toContain('They come back as they were once it is paid.');
	});

	it('lapsed: Pro has lapsed', () => {
		const { container } = withHidden(lapsedAcct(), twoHidden);
		expect(container.querySelector('[data-billing-hidden]')?.textContent ?? '').toContain('Pro has lapsed.');
	});

	// billing wording ruling (2026-11-06): a trial that ran and ended lapsed too.
	it('an ended trial lists the hidden repositories, Pro has lapsed', () => {
		const endedTrial = { ...acct('trialing'), period_end: '2026-11-01T00:00:00Z' };
		const { container } = withHidden(endedTrial, twoHidden);
		expect(container.querySelector('[data-billing-hidden]')?.textContent ?? '').toContain('Pro has lapsed.');
	});

	// billing wording ruling (2026-11-06): a Pro plan that has not started never ran,
	// so it is not enabled and gets no hidden list even with hidden repositories.
	it('a future start: Pro is not enabled, and no hidden list', () => {
		const future = { ...acct('active'), period_start: '2026-12-01T00:00:00Z', period_end: '2027-01-01T00:00:00Z' };
		const { container } = withHidden(future, twoHidden);
		expect(container.querySelector('[data-billing-problem]')?.textContent ?? '').toContain('Pro is not enabled.');
		expect(container.querySelector('[data-billing-hidden]')).toBeNull();
	});

	it('nothing hidden: no hidden-repositories section', () => {
		const { container } = withHidden(acct('active'), null);
		expect(container.querySelector('[data-billing-hidden]')).toBeNull();
	});

	// A failed repositories read is its own error, never "nothing hidden" (D83).
	it('a failed hidden-repositories read is an error in that section, with no count or list', () => {
		const { container } = withHidden(acct('past_due'), null, { hiddenError: { status: 0, message: 'could not reach the dojo service' } });
		const sec = container.querySelector('[data-billing-hidden]') as HTMLElement;
		expect(sec.querySelector('[data-load-error]')).toBeTruthy();
		expect(sec.querySelectorAll('li').length).toBe(0);
		expect(sec.textContent).not.toMatch(/are hidden from everyone|is hidden from everyone/);
	});

	// The page passes the billing section's hidden group to the screen (the load itself is
	// pinned by billing-section-load.spec and the wiring test below).
	it('the org page passes the hidden group and its error to ScrBilling', () => {
		const page = readFileSync(join(process.cwd(), 'src/routes/(app)/org/[slug]/[section]/+page.svelte'), 'utf8');
		expect(page).toMatch(/<ScrBilling[^>]*\bhidden=\{data\.billingHidden\}/);
		expect(page).toMatch(/<ScrBilling[^>]*\bhiddenError=\{data\.billingHiddenError\}/);
	});

	it('over a failed billing read, no hidden-repositories section', () => {
		const { container } = withHidden(acct('past_due'), twoHidden, { error: { status: 500, message: 'boom' } });
		expect(container.querySelector('[data-billing-hidden]')).toBeNull();
	});
});

// billing polish (user ruling 2026-11-05 + reviews/Design/L4-billing-followup-r1.md L1, L4):
// the hidden list shows ONLY for an account that had Pro and lost it — past due, or lapsed
// (canceled, or a period ended). On Free, never subscribed, unclaimed or an unknown
// plan there is no list; the private repositories stay hidden for everyone all the same.
// L4: the statuses are the schema's own (dojo.billing_status: trialing, active, past_due,
// canceled). A lapsed period is `active` with a period_end in the past — not a made-up key.
const withStatus = (status: string) => ({ ...acct('active'), status });
describe('ScrBilling · billing polish', () => {
	afterEach(cleanup);

	it.each([
		['canceled (not_subscribed)', withStatus('canceled')],
		['a period that ended (subscription_expired)', lapsedAcct()]
	] as const)('%s: listed, Pro has lapsed', (_s, account) => {
		const { container } = withHidden(account, twoHidden);
		const text = container.querySelector('[data-billing-hidden]')?.textContent ?? '';
		expect(text).toContain('2 private repositories are hidden from everyone in this organization.');
		expect(text).toContain('Pro has lapsed.');
	});

	it.each([
		['Free with no account', null],
		['Free by key', acct('active', 'free')],
		['an unknown plan key', acct('active', 'enterprise')]
	] as const)('%s: no hidden list, even with hidden repositories', (_s, account) => {
		const { container } = withHidden(account, twoHidden);
		expect(container.querySelector('[data-billing-hidden]')).toBeNull();
		expect(container.textContent).not.toMatch(/once Pro is attached|hidden from everyone/);
	});

	// No list to show means no read failure to show either.
	it('on Free, a failed hidden read draws no section', () => {
		const { container } = withHidden(null, null, { hiddenError: { status: 0, message: 'could not reach the dojo service' } });
		expect(container.querySelector('[data-billing-hidden]')).toBeNull();
	});

	// L4: clearing the search gives the disclosure back to the user — it stays as they left it.
	it('clearing the search keeps a hand-opened "Who is counted" open', async () => {
		const { container } = screen(null);
		const who = container.querySelector('details[data-billing-counted]') as HTMLDetailsElement;
		who.open = true;
		who.dispatchEvent(new Event('toggle'));
		const input = container.querySelector('[data-screen-filters] [data-screen-search] input') as HTMLInputElement;
		for (const q of ['gamma', '']) {
			await fireEvent.input(input, { target: { value: q } });
			await new Promise((r) => setTimeout(r, 300));
			await tick();
		}
		expect(who.open).toBe(true);
	});

	// L2: a browser fires `toggle` when a search opens the disclosure. That is not the
	// user's choice, so it must not be recorded — clearing the search closes it again.
	it('a toggle during a search is not recorded: clearing the search closes "Who is counted"', async () => {
		const { container } = screen(null);
		const who = container.querySelector('details[data-billing-counted]') as HTMLDetailsElement;
		const input = container.querySelector('[data-screen-filters] [data-screen-search] input') as HTMLInputElement;
		await fireEvent.input(input, { target: { value: 'gamma' } });
		await new Promise((r) => setTimeout(r, 300));
		await tick();
		expect(who.open).toBe(true);
		who.dispatchEvent(new Event('toggle'));
		await fireEvent.input(input, { target: { value: '' } });
		await new Promise((r) => setTimeout(r, 300));
		await tick();
		expect(who.open).toBe(false);
	});

	// L2: a real wiring check — the page hands the billing section to loadBillingSection with
	// the caller's repositories read, so a stub like `async () => []` fails here. The billing
	// read itself must stay guarded for the 'billing' section, not another one.
	it('the org page loads the billing section through loadBillingSection with listMyRepos', () => {
		const load = readFileSync(join(process.cwd(), 'src/routes/(app)/org/[slug]/[section]/+page.ts'), 'utf8');
		expect(load).toMatch(/section === 'billing'\s*\?\s*await loadBillingSection\(\s*slug\s*,[\s\S]*?\(\)\s*=>\s*listMyRepos\(opts\)\s*\)/);
		expect(load).toMatch(/guardedFor<\s*BillingResponse\s*,\s*KitBilling\s*>\s*\(\s*'billing'/);
		expect(load).not.toMatch(/loadOrgHidden\(/);
	});
});
