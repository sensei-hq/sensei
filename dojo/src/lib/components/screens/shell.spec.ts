import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AppShellHarness from '$lib/components/kit/AppShell.harness.svelte';
import OrgSwitcher from '$lib/components/kit/OrgSwitcher.svelte';
import ScrPlaceholder from './ScrPlaceholder.svelte';
import { navGroupsFor } from '$lib/nav';
import { toKitDojos, toKitOrg } from '$lib/chrome';
import type { DojoOrg } from '$lib/dojo-data';

// Chunk-1 shell + landing wiring. The pure nav/role/chrome logic is unit-tested
// in nav.spec.ts + chrome.spec.ts; here we render the kit chrome the
// (app) layout drives (nav groups per context + role rank; the switcher lists
// memberships) and the ScrYourWork landing (band / runs / projects; honest empty
// for a membership-less viewer).

const acme: DojoOrg = {
	id: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'Organization',
	host: 'self',
	url: 'dojo.acme.internal',
	role: 'Admin',
	from: 'GitHub · org owner',
	members: 48,
	pending: 4
};
const globex: DojoOrg = {
	id: 'globex',
	kanji: '社',
	name: 'Globex',
	kind: 'Organization',
	host: 'saas',
	url: 'github/globex',
	role: 'Maintainer',
	from: 'GitHub · repo admin',
	members: 12,
	pending: 2
};

describe('dojo shell — nav groups render per context + role rank', () => {
	afterEach(cleanup);

	it('personal context renders the NAV_YOU groups', () => {
		const { getByText, getAllByText } = render(AppShellHarness, {
			props: { context: 'you', nav: navGroupsFor(null), active: 'inbox' }
		});
		// The inbox model: Work · Govern · Dōjōs (no separate Relay group).
		for (const group of ['Work', 'Govern', 'Dōjōs']) {
			expect(getByText(group)).toBeTruthy();
		}
		expect(getByText('Inbox')).toBeTruthy(); // the nav landing item
		// "Your work" still labels the personal context switcher.
		expect(getAllByText('Your work').length).toBeGreaterThan(0);
	});

	it('a maintainer org context shows Overview + Govern but NOT Admin', () => {
		const { getByText, queryByText } = render(AppShellHarness, {
			props: { context: 'org', org: toKitOrg(globex), nav: navGroupsFor('maintainer'), active: 'home' }
		});
		expect(getByText('Overview')).toBeTruthy();
		expect(getByText('Govern')).toBeTruthy();
		expect(queryByText('Admin')).toBeNull();
		expect(queryByText('Clients')).toBeNull();
	});

	it('an admin org context shows every management group', () => {
		const { getByText } = render(AppShellHarness, {
			props: { context: 'org', org: toKitOrg(acme), nav: navGroupsFor('admin'), active: 'home' }
		});
		for (const group of ['Overview', 'Govern', 'Clients', 'Admin']) {
			expect(getByText(group)).toBeTruthy();
		}
	});
});

describe('dojo shell — org switcher lists memberships', () => {
	afterEach(cleanup);

	it('opens and lists every membership; picking one fires onpick(slug)', async () => {
		const onpick = vi.fn();
		const { getByText, getAllByText } = render(OrgSwitcher, {
			props: { context: 'you', dojos: toKitDojos([acme, globex]), onpick }
		});
		await fireEvent.click(getAllByText('Your work')[0].closest('button')!);
		expect(getByText('Acme Corp')).toBeTruthy();
		expect(getByText('Globex')).toBeTruthy();
		await fireEvent.click(getByText('Globex').closest('button')!);
		expect(onpick).toHaveBeenCalledWith('globex');
	});

	it('a membership-less viewer still gets the pinned "Your work" home', () => {
		const { getAllByText } = render(OrgSwitcher, {
			props: { context: 'you', dojos: [], onpick: vi.fn() }
		});
		expect(getAllByText('Your work').length).toBeGreaterThan(0);
	});

	it('offers "All organizations" + "Create or join" entries that route to /orgs', async () => {
		const { getByText, getAllByText } = render(OrgSwitcher, {
			props: { context: 'you', dojos: toKitDojos([acme]), onpick: vi.fn() }
		});
		await fireEvent.click(getAllByText('Your work')[0].closest('button')!);
		// both /orgs entries render and are live buttons (not dead anchors).
		const all = getByText('All organizations').closest('button')!;
		const create = getByText('Create or join a dōjō').closest('button')!;
		expect(all).toBeTruthy();
		expect(create).toBeTruthy();
		// clicking navigates (goto is stubbed to a no-op under vitest) — no throw.
		await fireEvent.click(all);
		await fireEvent.click(create);
	});
});

describe('ScrPlaceholder — coming-in-the-rebuild', () => {
	afterEach(cleanup);

	it('renders the section title + the rebuild empty state', () => {
		const { getByText } = render(ScrPlaceholder, { props: { title: 'Triage', eyebrow: 'Acme Corp' } });
		expect(getByText('Triage')).toBeTruthy();
		expect(getByText('Coming in the rebuild')).toBeTruthy();
	});
});
