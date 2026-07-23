import { describe, expect, it } from 'vitest';
import {
	NAV_YOU,
	NAV_ORG_BASE,
	K2_ROLE_RANK,
	TABS_YOU,
	TABS_ORG,
	navForOrg,
	navGroupsFor,
	tabsFor,
	YOU_SECTIONS,
	ORG_SECTIONS,
	youHref,
	orgHref,
	sectionFromYouPath,
	sectionFromOrgPath,
	labelForSection
} from './dojo2-nav';

// The dojo2 two-nav IA (chunk 1). Ported faithfully from the finalized mockup
// (docs/mockups/Sensei/lib/dojo2/dojo2-app.jsx): a personal NAV_YOU zone and a
// role-scoped NAV_ORG (filtered by K2_ROLE_RANK via navForOrg). Kept pure so the
// grouping, role-gating, and route wiring are unit-tested without rendering.

describe('NAV_YOU — the personal zone', () => {
	it('keeps the mockup groups in order (Work · Govern · Relay · Dōjōs)', () => {
		expect(NAV_YOU.map((g) => g.group)).toEqual(['Work', 'Govern', 'Relay', 'Dōjōs']);
	});

	it('leads with "Your work" as the first item', () => {
		expect(NAV_YOU[0].items[0].id).toBe('work');
		expect(NAV_YOU[0].items[0].label).toBe('Your work');
	});

	it('assigns a unique id to every personal nav item', () => {
		const ids = NAV_YOU.flatMap((g) => g.items.map((it) => it.id));
		expect(new Set(ids).size).toBe(ids.length);
	});
});

describe('navForOrg — additive role rank filtering', () => {
	it('a developer sees only the role-free Overview group', () => {
		const groups = navForOrg('developer');
		expect(groups.map((g) => g.group)).toEqual(['Overview']);
	});

	it('a maintainer adds Govern', () => {
		expect(navForOrg('maintainer').map((g) => g.group)).toEqual(['Overview', 'Govern']);
	});

	it('a lead adds Clients (and keeps Govern)', () => {
		expect(navForOrg('lead').map((g) => g.group)).toEqual(['Overview', 'Govern', 'Clients']);
	});

	it('an admin sees every group', () => {
		expect(navForOrg('admin').map((g) => g.group)).toEqual([
			'Overview',
			'Govern',
			'Clients',
			'Admin'
		]);
	});

	it('an unknown role floors to developer rank', () => {
		expect(navForOrg('nobody').map((g) => g.group)).toEqual(['Overview']);
		expect(navForOrg(undefined).map((g) => g.group)).toEqual(['Overview']);
	});

	it('ranks developer < maintainer < lead < admin', () => {
		expect(K2_ROLE_RANK.developer).toBeLessThan(K2_ROLE_RANK.maintainer);
		expect(K2_ROLE_RANK.maintainer).toBeLessThan(K2_ROLE_RANK.lead);
		expect(K2_ROLE_RANK.lead).toBeLessThan(K2_ROLE_RANK.admin);
	});
});

describe('navGroupsFor / tabsFor — context switch', () => {
	it('personal context returns NAV_YOU', () => {
		expect(navGroupsFor(null)).toBe(NAV_YOU);
	});

	it('org context returns the role-scoped groups', () => {
		expect(navGroupsFor('maintainer').map((g) => g.group)).toEqual(['Overview', 'Govern']);
	});

	it('personal tabs are TABS_YOU; org tabs are TABS_ORG', () => {
		expect(tabsFor(null)).toBe(TABS_YOU);
		expect(tabsFor('admin')).toBe(TABS_ORG);
	});
});

describe('section reachability — every nav destination is a known section', () => {
	it('YOU_SECTIONS covers every NAV_YOU item id', () => {
		const ids = NAV_YOU.flatMap((g) => g.items.map((it) => it.id)).filter((id) => id !== 'work');
		for (const id of ids) expect(YOU_SECTIONS).toContain(id);
	});

	it('ORG_SECTIONS covers every NAV_ORG item id', () => {
		const ids = NAV_ORG_BASE.flatMap((g) => g.items.map((it) => it.id)).filter(
			(id) => id !== 'home'
		);
		for (const id of ids) expect(ORG_SECTIONS).toContain(id);
	});
});

describe('href builders — clean dojo2 URLs', () => {
	it('the personal landing is /you; a section is /you/{section}', () => {
		expect(youHref()).toBe('/you');
		expect(youHref('work')).toBe('/you');
		expect(youHref('runs')).toBe('/you/runs');
	});

	it('an org home is /org/{slug}; a section is /org/{slug}/{section}', () => {
		expect(orgHref('acme')).toBe('/org/acme');
		expect(orgHref('acme', 'home')).toBe('/org/acme');
		expect(orgHref('acme', 'triage')).toBe('/org/acme/triage');
	});
});

describe('active-section resolution from a URL path', () => {
	it('maps /you → work (the landing)', () => {
		expect(sectionFromYouPath('/you')).toBe('work');
		expect(sectionFromYouPath('/you/')).toBe('work');
	});

	it('maps /you/runs → runs', () => {
		expect(sectionFromYouPath('/you/runs')).toBe('runs');
	});

	it('an unknown you-section falls back to work', () => {
		expect(sectionFromYouPath('/you/bogus')).toBe('work');
	});

	it('maps /org/acme → home', () => {
		expect(sectionFromOrgPath('/org/acme')).toBe('home');
	});

	it('maps /org/acme/triage → triage', () => {
		expect(sectionFromOrgPath('/org/acme/triage')).toBe('triage');
	});

	it('an unknown org-section falls back to home', () => {
		expect(sectionFromOrgPath('/org/acme/bogus')).toBe('home');
	});
});

describe('labelForSection — placeholder header copy', () => {
	it('resolves a personal section label from NAV_YOU', () => {
		expect(labelForSection('runs', 'you')).toBe('Live runs');
		expect(labelForSection('contributions', 'you')).toBe('Contributions');
	});

	it('resolves an org section label from NAV_ORG', () => {
		expect(labelForSection('triage', 'org')).toBe('Triage');
		expect(labelForSection('members', 'org')).toBe('Members & Roles');
	});

	it('title-cases an unknown section id', () => {
		expect(labelForSection('mystery', 'you')).toBe('Mystery');
	});
});
