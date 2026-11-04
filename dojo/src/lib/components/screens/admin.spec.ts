import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import ScrScopes from './ScrScopes.svelte';
import ScrIdentity from './ScrIdentity.svelte';
import ScrHealth from './ScrHealth.svelte';
import ScrBilling from './ScrBilling.svelte';
import { toKitBilling } from '$lib/billing-map';
import {
	membersFor,
	rolePoliciesFor,
	auditLogFor,
	scopeOwnersFor,
	identityFor,
	healthFor,
	billingFor,
	me
} from '$lib/components/kit/fixtures';

// Chunk-6 admin (Admin group) org consoles — the /org/[slug]/[section] admin
// screens that complete NAV_ORG. Each renders off the ported admin fixtures
// (presentational — real /v1 wiring is a later chunk). We assert the tabbed
// role surfaces (members table + role tags · tab switching · audit-from-section),
// the scope-ownership groups + SLA + unowned fallback, the identity mappings +
// IdP/SCIM cards, the health signal cards + bar chart + alerts, and the billing
// tiers + relay rows + invoices.

const members = membersFor('acme');
const policies = rolePoliciesFor('acme');
const audit = auditLogFor('acme');
const owners = scopeOwnersFor('acme');
const identity = identityFor('acme');
const health = healthFor('acme');
const billing = billingFor('acme');

describe('ScrRoleSurfaces — Members & Roles / Policies / Audit tabs', () => {
	afterEach(cleanup);

	it('opens on the Members tab and lists a member with its git-derived role', () => {
		const { getByText, getAllByText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme Corp', tab: 'members', members, policies, audit, me: me.name }
		});
		expect(getByText('Acme Corp · admin')).toBeTruthy();
		// "Members & Roles" is both the tab label and the SectionHead title.
		expect(getAllByText('Members & Roles').length).toBe(2);
		expect(getByText('Keiko Tanaka')).toBeTruthy();
		expect(getByText('git: Org owner · all')).toBeTruthy();
		// the viewer's own row carries a "you" chip.
		expect(getByText('you')).toBeTruthy();
	});

	it('renders a role tag for each member (git-derived + override roles)', () => {
		const { getAllByText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme Corp', tab: 'members', members, policies, audit, me: me.name }
		});
		// three maintainers in the fixture → three "maintainer" role tags.
		expect(getAllByText('maintainer').length).toBe(3);
		expect(getAllByText('admin').length).toBeGreaterThanOrEqual(1);
		expect(getAllByText('contributor').length).toBe(2);
	});

	it('the members header CTA reads Invite', () => {
		const { getByText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme Corp', tab: 'members', members, policies, audit, me: me.name }
		});
		expect(getByText('Invite')).toBeTruthy();
	});

	it('opens on the Audit tab when the section maps to audit', () => {
		const { getByText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme Corp', tab: 'audit', members, policies, audit, me: me.name }
		});
		expect(getByText('Audit log')).toBeTruthy();
		expect(getByText('Export')).toBeTruthy();
	});

	it('switching to Policies shows the additive-roles grid; switching to Audit shows the log', async () => {
		const { getByText, getAllByText, queryByText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme Corp', tab: 'members', members, policies, audit, me: me.name }
		});
		// Policies tab — the additive-role banner + a role policy card.
		await fireEvent.click(getByText('Policies'));
		// Now part of the section header's description run, not a Banner title.
		expect(getByText(/Roles are additive — a role only ever adds capability\./)).toBeTruthy();
		expect(getByText('authors governance · triages')).toBeTruthy();
		expect(getByText('Role policies')).toBeTruthy();
		// the members table is gone.
		expect(queryByText('git: Org owner · all')).toBeNull();
		// Audit tab — log lines rendered from the shared thread, attributed to the
		// viewer (not sensei) where who != sensei, and to "sensei" otherwise.
		await fireEvent.click(getByText('Audit'));
		expect(getByText('Audit log')).toBeTruthy();
		// the viewer's turns are attributed to their name (one line per viewer turn).
		const viewerTurns = audit.filter((t) => t.who !== 'sensei');
		expect(getAllByText(new RegExp('^' + me.name + ' ·')).length).toBe(viewerTurns.length);
		// sensei's turns are attributed to "sensei".
		const senseiTurns = audit.filter((t) => t.who === 'sensei');
		expect(getAllByText(/^sensei ·/).length).toBe(senseiTurns.length);
	});

	it('re-follows the tab prop when it changes (URL is the source of truth)', async () => {
		// members↔audit share this screen; a nav changes the `tab` prop without a
		// re-mount, so the active tab must follow the prop.
		const { getByText, rerender } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme Corp', tab: 'members', members, policies, audit, me: me.name }
		});
		expect(getByText('git: Org owner · all')).toBeTruthy();
		await rerender({ orgName: 'Acme Corp', tab: 'audit', members, policies, audit, me: me.name });
		await tick();
		expect(getByText('Audit log')).toBeTruthy();
	});
});

describe('ScrScopes — scope ownership, SLA and fallback', () => {
	afterEach(cleanup);

	it('groups scopes and shows an owner with role, queue and SLA', () => {
		const { getByText, queryByText } = render(ScrScopes, { props: { orgName: 'Acme Corp', owners } });
		expect(getByText('Scopes & policies')).toBeTruthy();
		// grouped by the ladder: dōjō → workspace → repository (D25).
		expect(getByText('Dōjō')).toBeTruthy();
		expect(getByText('Workspace')).toBeTruthy();
		expect(getByText('Repository')).toBeTruthy();
		expect(queryByText('Teams')).toBeNull();
		expect(queryByText('Stacks')).toBeNull();
		expect(getByText('payments')).toBeTruthy();
		expect(getByText('5 in queue · SLA 12h')).toBeTruthy();
		expect(getByText('Marco Diaz')).toBeTruthy();
	});

	it('warns when a scope is unowned and marks it fallback', () => {
		const { getByText } = render(ScrScopes, { props: { orgName: 'Acme Corp', owners } });
		// the acme/ledger repository scope has no owner → the warning banner + the fallback chip.
		expect(getByText(/scope has no owner/)).toBeTruthy();
		expect(getByText('unowned · fallback')).toBeTruthy();
	});

	it('states the fallback rule in the header and shows no warning when all owned', () => {
		const allOwned = owners.filter((o) => o.owner);
		const { getByText, queryByText } = render(ScrScopes, {
			props: { orgName: 'Acme Corp', owners: allOwned }
		});
		// The standing explanation lives in the section header now, so it is one
		// run of text with the lead sentence — matched by pattern, not equality.
		expect(
			getByText(/Anything unowned routes to a fallback maintainer so nothing stalls\./)
		).toBeTruthy();
		expect(queryByText('unowned · fallback')).toBeNull();
		// ...and with nothing unowned there is no warning band at all: the band
		// means "something needs you" rather than being permanent chrome.
		expect(queryByText(/no owner/)).toBeNull();
	});

	it('fires onAssign with the scope row (Assign for unowned, Reassign for owned)', async () => {
		const onAssign = vi.fn();
		const { getByText } = render(ScrScopes, { props: { orgName: 'Acme Corp', owners, onAssign } });
		await fireEvent.click(getByText('Assign'));
		const unowned = owners.find((o) => !o.owner);
		expect(onAssign).toHaveBeenCalledWith(unowned);
	});
});

describe('ScrIdentity — IdP, SCIM and git mappings', () => {
	afterEach(cleanup);

	it('shows the IdP protocol/status and SCIM state', () => {
		const { getByText } = render(ScrIdentity, { props: { orgName: 'Acme Corp', identity } });
		expect(getByText('Identity & SSO')).toBeTruthy();
		expect(getByText('Okta')).toBeTruthy();
		expect(getByText('OIDC')).toBeTruthy();
		expect(getByText('connected')).toBeTruthy();
		expect(getByText('acme.okta.com')).toBeTruthy();
		expect(getByText('Enabled — members sync from your directory')).toBeTruthy();
	});

	it('lists the identity mappings with their targets and member counts', () => {
		const { getByText } = render(ScrIdentity, { props: { orgName: 'Acme Corp', identity } });
		expect(getByText('Identity mappings')).toBeTruthy();
		expect(getByText('GitHub org · acme')).toBeTruthy();
		expect(getByText('→ auto-join · role from repo access')).toBeTruthy();
		expect(getByText('41 members')).toBeTruthy();
		expect(getByText('Device code')).toBeTruthy();
	});

	it('reads SCIM as disabled when off', () => {
		const off = { ...identity, scim: false };
		const { getByText } = render(ScrIdentity, { props: { orgName: 'Acme Corp', identity: off } });
		expect(getByText('Disabled')).toBeTruthy();
	});
});

describe('ScrHealth — signal cards, bar chart and alerts', () => {
	afterEach(cleanup);

	it('says the anomaly feed is not measured rather than showing 0 (#241)', () => {
		const { container } = render(ScrHealth, { props: { orgName: 'Acme Corp', health: { ...health, alerts: null } } });
		expect(container.querySelector('[data-alerts-unmeasured]')?.textContent).toMatch(/not measured/i);
	});

	it('renders a signal card per vital sign with its number and sub', () => {
		const { getByText } = render(ScrHealth, { props: { orgName: 'Acme Corp', health } });
		expect(getByText('Health / Monitor')).toBeTruthy();
		expect(getByText('Sessions this week')).toBeTruthy();
		expect(getByText('312')).toBeTruthy();
		expect(getByText('↑ 14%')).toBeTruthy();
		expect(getByText('Adoption rate')).toBeTruthy();
		expect(getByText('68%')).toBeTruthy();
	});

	it('shows the contributions-vs-approvals chart weeks and the alert feed', () => {
		const { getByText } = render(ScrHealth, { props: { orgName: 'Acme Corp', health } });
		expect(getByText('Contributions vs approvals')).toBeTruthy();
		expect(getByText('W1')).toBeTruthy();
		expect(getByText('W4')).toBeTruthy();
		expect(getByText('Leak-guard & anomalies')).toBeTruthy();
		expect(getByText('acme/ledger scope queue has no owner')).toBeTruthy();
	});
});

describe('ScrBilling — Free and Pro, no price, an honest attach', () => {
	afterEach(cleanup);
	const usage = { seats_used: 18, total_active_seats: 25, billable_users: [] };
	const free = () => toKitBilling({ account: null, usage });

	it('with no billing account, reads Free and shows no renewal or invoice it does not have (#106)', () => {
		const { getAllByText, getByText, queryByText } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing: free() } });
		expect(getAllByText('Free').length).toBeGreaterThanOrEqual(1);
		expect(getByText('No invoices — no payment provider is connected.')).toBeTruthy();
		expect(queryByText(/Renews/)).toBeNull();
	});

	it('shows both plans, Pro priced "to be announced", and no dollar amount anywhere (D75)', () => {
		const { container, getByText, getAllByText } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing: free() } });
		expect(getAllByText('Pro').length).toBeGreaterThanOrEqual(1);
		expect(getByText('To be announced')).toBeTruthy();
		expect(getByText('current')).toBeTruthy();
		expect(container.textContent).not.toMatch(/\$\s?\d/);
	});

	it('says what is always free, and that Pro is what targets on private repositories need', () => {
		const { container } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing: free() } });
		expect(container.textContent).toMatch(/SSO, SCIM, audit/);
		expect(container.textContent).toContain("Blocking targets on an organization's private repositories");
	});

	it('the seat card says what it counts (L2)', () => {
		const { getByText } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing: free() } });
		expect(getByText('seats on private workspaces')).toBeTruthy();
	});

	it('Attach Pro is offered but cannot run until payments are connected — and says so', () => {
		const { container, getByText } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing: free() } });
		const attach = container.querySelector('[data-action="attach-pro"]') as HTMLButtonElement;
		expect(attach.textContent).toContain('Attach Pro');
		expect(attach.disabled).toBe(true);
		const n = container.querySelector('[data-attach-notice] [data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('normal');
		expect(getByText('Plan changes open once payments are connected')).toBeTruthy();
	});

	it('on Pro: its renewal, no attach, and the disabled Move to Free says why (M6)', () => {
		const { container, getByText } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing } });
		expect(getByText(/Renews Aug 1/)).toBeTruthy();
		expect(container.querySelector('[data-action="attach-pro"]')).toBeNull();
		expect((container.querySelector('[data-action="move-free"]') as HTMLButtonElement).disabled).toBe(true);
		expect(container.querySelector('[data-attach-notice]')!.textContent).toContain('Until then this plan stays as it is.');
	});

	it('a lapsed account says why its private repositories are hidden (M5)', () => {
		const lapsed = toKitBilling({ account: { plan: 'pro', status: 'past_due', seats_included: 0, seats_used: 0, seats_computed_at: null, period_start: null, period_end: '2026-12-01T00:00:00Z' }, usage }, new Date('2026-11-01T00:00:00Z'));
		const { container } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing: lapsed } });
		const n = container.querySelector('[data-billing-problem] [data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('refusal');
		expect(n.textContent).toContain('past due');
	});
});

describe('ScrBilling — a read that was refused (M3, r1)', () => {
	afterEach(cleanup);
	it('a 403 is a refusal that says who to ask, never "Retry"', () => {
		const { container } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing, error: { status: 403, message: 'admins only' } } });
		const n = container.querySelector('[data-load-error]') as HTMLElement;
		expect(n.dataset.tone).toBe('refusal');
		expect(n.textContent).toContain('Ask an admin for access.');
		expect(n.textContent).not.toContain('Retry');
	});

	it('an unreachable dōjō is a fault that says so', () => {
		const { container } = render(ScrBilling, { props: { orgName: 'Acme Corp', billing, error: { status: 0, message: 'could not reach the dojo service' } } });
		expect((container.querySelector('[data-load-error]') as HTMLElement).dataset.tone).toBe('fault');
	});
});

describe('ScrScopes — with no scope owners recorded (#106)', () => {
	afterEach(cleanup);

	it('says no scope has an owner rather than rendering a blank page', () => {
		const { getByText } = render(ScrScopes, { props: { orgName: 'sensei-hq', owners: [] } });
		expect(getByText("No scope has an owner yet — every queue is triaged by the dōjō's maintainers.")).toBeTruthy();
	});
});
