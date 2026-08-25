import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import ScrScopes from './ScrScopes.svelte';
import ScrIdentity from './ScrIdentity.svelte';
import ScrHealth from './ScrHealth.svelte';
import ScrBilling from './ScrBilling.svelte';
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
		expect(getAllByText('developer').length).toBe(2);
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
		expect(getByText(/Roles are additive and derived from git\./)).toBeTruthy();
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
		const { getByText } = render(ScrScopes, { props: { orgName: 'Acme Corp', owners } });
		expect(getByText('Scopes & policies')).toBeTruthy();
		expect(getByText('Company')).toBeTruthy();
		expect(getByText('Teams')).toBeTruthy();
		expect(getByText('Stacks')).toBeTruthy();
		expect(getByText('Payments')).toBeTruthy();
		expect(getByText('5 in queue · SLA 12h')).toBeTruthy();
		expect(getByText('Marco Diaz')).toBeTruthy();
	});

	it('warns when a scope is unowned and marks it fallback', () => {
		const { getByText } = render(ScrScopes, { props: { orgName: 'Acme Corp', owners } });
		// the Postgres scope has no owner → the warning banner + the fallback chip.
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
		expect(getByText('Postgres scope queue has no owner')).toBeTruthy();
	});
});

describe('ScrBilling — plan, tiers, relay rows and invoices', () => {
	afterEach(cleanup);

	it('shows the plan summary with the live monthly total (seats × per-seat)', () => {
		const { getByText, getAllByText } = render(ScrBilling, {
			props: { orgName: 'Acme Corp', billing }
		});
		expect(getByText('Plan & billing')).toBeTruthy();
		// the plan appears in the header chip and the current-plan card.
		expect(getAllByText('Team · private').length).toBeGreaterThanOrEqual(1);
		// 34 active × $12 = $408.
		expect(getByText('$408')).toBeTruthy();
		expect(getByText('34 × $12')).toBeTruthy();
	});

	it('renders the three tiers, marking the current one and offering CTAs for the rest', () => {
		const { getByText, getAllByText } = render(ScrBilling, {
			props: { orgName: 'Acme Corp', billing }
		});
		// "Free" is both the tier name and the free tier's price → two matches.
		expect(getAllByText('Free').length).toBe(2);
		expect(getByText('Enterprise')).toBeTruthy();
		// the current (Team) tier shows a "current" chip; the others show a CTA
		// each (Free → Downgrade, Enterprise → Contact sales).
		expect(getByText('current')).toBeTruthy();
		expect(getByText('Downgrade')).toBeTruthy();
		expect(getByText('Contact sales')).toBeTruthy();
	});

	it('shows the relay free-vs-paid rows and the invoice history', () => {
		const { getByText, getAllByText } = render(ScrBilling, {
			props: { orgName: 'Acme Corp', billing }
		});
		expect(getByText("Relay — free for individuals, paid where it's shared")).toBeTruthy();
		// two free rows + two paid rows in the fixture.
		expect(getAllByText('free · individuals').length).toBe(2);
		expect(getAllByText('paid · team').length).toBe(2);
		expect(getByText('Invoices')).toBeTruthy();
		expect(getByText('Jul 1, 2026')).toBeTruthy();
		expect(getByText('$408.00')).toBeTruthy();
	});
});
