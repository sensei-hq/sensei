import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
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
} from './nav';

// The dojo two-nav IA (chunk 1). Ported faithfully from the finalized mockup
// (docs/mockups/Sensei/lib/dojo/dojo2-app.jsx): a personal NAV_YOU zone and a
// role-scoped NAV_ORG (filtered by K2_ROLE_RANK via navForOrg). Kept pure so the
// grouping, role-gating, and route wiring are unit-tested without rendering.

describe('NAV_YOU — the personal zone (inbox model)', () => {
	it('is the inbox model: Work · Govern · Dōjōs, no separate Relay group', () => {
		expect(NAV_YOU.map((g) => g.group)).toEqual(['Work', 'Govern', 'Dōjōs']);
	});

	it('leads with the Inbox as the first item', () => {
		expect(NAV_YOU[0].items[0].id).toBe('inbox');
		expect(NAV_YOU[0].items[0].label).toBe('Inbox');
	});

	it('has the inbox-model destinations and none of the retired relay sections', () => {
		const ids = NAV_YOU.flatMap((g) => g.items.map((it) => it.id));
		expect(ids).toEqual([
			'inbox',
			'workspaces',
			'metrics',
			'rules',
			'packs',
			'dojos',
			'sharing',
			'contributions'
		]);
		for (const gone of ['work', 'runs', 'approve', 'decide', 'chat']) {
			expect(ids).not.toContain(gone);
		}
	});

	it('assigns a unique id to every personal nav item', () => {
		const ids = NAV_YOU.flatMap((g) => g.items.map((it) => it.id));
		expect(new Set(ids).size).toBe(ids.length);
	});
});

describe('navForOrg — additive role rank filtering', () => {
	it('a contributor sees only the role-free Overview group', () => {
		const groups = navForOrg('contributor');
		expect(groups.map((g) => g.group)).toEqual(['Overview']);
	});

	it('a maintainer adds Govern', () => {
		expect(navForOrg('maintainer').map((g) => g.group)).toEqual(['Overview', 'Govern']);
	});

	it('a lead sees what a maintainer sees in the nav — the Teams view is what widens (#307 R8, D38)', () => {
		expect(navForOrg('lead').map((g) => g.group)).toEqual(['Overview', 'Govern']);
	});

	it('every member has Teams in the Overview (totals below lead, people from lead up)', () => {
		for (const role of ['contributor', 'maintainer', 'lead', 'admin']) {
			expect(navForOrg(role)[0].items.map((i) => i.id)).toContain('teams');
		}
	});

	it('an admin sees every group', () => {
		expect(navForOrg('admin').map((g) => g.group)).toEqual(['Overview', 'Govern', 'Admin']);
	});

	it('names no engagement, incident or client-audit section (D34)', () => {
		const ids = NAV_ORG_BASE.flatMap((g) => g.items.map((i) => i.id));
		for (const gone of ['engagements', 'incidents', 'clientaudit']) expect(ids).not.toContain(gone);
	});

	it('an unknown role floors to contributor rank', () => {
		expect(navForOrg('nobody').map((g) => g.group)).toEqual(['Overview']);
		expect(navForOrg(undefined).map((g) => g.group)).toEqual(['Overview']);
	});

	it('ranks contributor < maintainer < lead < admin (D1)', () => {
		expect(K2_ROLE_RANK.contributor).toBeLessThan(K2_ROLE_RANK.maintainer);
		expect(K2_ROLE_RANK.developer).toBeUndefined();
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
	it('YOU_SECTIONS covers every NAV_YOU item id except the inbox landing', () => {
		const ids = NAV_YOU.flatMap((g) => g.items.map((it) => it.id)).filter((id) => id !== 'inbox');
		for (const id of ids) expect(YOU_SECTIONS).toContain(id);
		expect(YOU_SECTIONS).not.toContain('inbox'); // the landing is not a section
	});

	it('ORG_SECTIONS covers every NAV_ORG item id', () => {
		const ids = NAV_ORG_BASE.flatMap((g) => g.items.map((it) => it.id)).filter(
			(id) => id !== 'home'
		);
		for (const id of ids) expect(ORG_SECTIONS).toContain(id);
	});
});

describe('href builders — clean dojo URLs', () => {
	it('the personal landing is /you; a section is /you/{section}', () => {
		expect(youHref()).toBe('/you');
		expect(youHref('inbox')).toBe('/you');
		expect(youHref('workspaces')).toBe('/you/workspaces');
		expect(youHref('runs')).toBe('/you/runs'); // run-detail base still resolves
	});

	it('an org home is /org/{slug}; a section is /org/{slug}/{section}', () => {
		expect(orgHref('acme')).toBe('/org/acme');
		expect(orgHref('acme', 'home')).toBe('/org/acme');
		expect(orgHref('acme', 'triage')).toBe('/org/acme/triage');
	});
});

describe('active-section resolution from a URL path', () => {
	it('maps /you → inbox (the landing)', () => {
		expect(sectionFromYouPath('/you')).toBe('inbox');
		expect(sectionFromYouPath('/you/')).toBe('inbox');
	});

	it('maps a real section like /you/workspaces → workspaces', () => {
		expect(sectionFromYouPath('/you/workspaces')).toBe('workspaces');
	});

	it('a retired relay section (approve/decide/chat/runs) falls back to the inbox', () => {
		for (const gone of ['approve', 'decide', 'chat', 'runs']) {
			expect(sectionFromYouPath(`/you/${gone}`)).toBe('inbox');
		}
	});

	it('an unknown you-section falls back to the inbox', () => {
		expect(sectionFromYouPath('/you/bogus')).toBe('inbox');
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
		expect(labelForSection('workspaces', 'you')).toBe('Workspaces');
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

// The org section route dispatches on `data.section`. Every id in ORG_SECTIONS
// needs its own named branch: the route used to end in an unlabelled `{:else}`
// holding Plan & billing, which was only correct by coincidence — the branches
// happened to cover ORG_SECTIONS exactly. Add a nav section without a screen and
// that fallthrough would have shown a money screen instead. Read from source so
// the two files can't drift apart silently.
describe('org section dispatch covers every nav section', () => {
	// Resolved from the vitest root (the dojo package), not import.meta.url —
	// under this config that isn't a file: URL.
	const route = readFileSync(
		resolve(process.cwd(), 'src/routes/(app)/org/[slug]/[section]/+page.svelte'),
		'utf8',
	);

	it.each([...ORG_SECTIONS])('%s has an explicit branch', (section) => {
		expect(route).toContain(`data.section === '${section}'`);
	});

	it('does not leave billing as the unnamed fallthrough', () => {
		// The last branch must be a placeholder, not a real console screen.
		const tail = route.slice(route.lastIndexOf('{:else}'));
		expect(tail).toContain('ScrPlaceholder');
		expect(tail).not.toContain('ScrBilling');
	});
});

// #133: every nav item and tab carries BOTH a kanji and an icon, so the glyph
// preference can show either without an empty slot.
describe('nav marks', () => {
	const items = [
		...NAV_YOU.flatMap((g) => g.items),
		...NAV_ORG_BASE.flatMap((g) => g.items),
		...TABS_YOU,
		...TABS_ORG
	];
	it('gives every item a kanji and an icon', () => {
		expect(items.length).toBeGreaterThan(20);
		for (const it of items) {
			expect(it.kanji, it.id).toMatch(/^\p{Script=Han}$/u);
			expect(it.icon, it.id).toBeTruthy();
		}
	});
	it('gives a tab the same marks as the nav item it opens', () => {
		const byId = new Map([...NAV_YOU, ...NAV_ORG_BASE].flatMap((g) => g.items).map((i) => [i.id, i]));
		for (const t of [...TABS_YOU, ...TABS_ORG]) {
			expect({ kanji: t.kanji, icon: t.icon }, t.id).toEqual({ kanji: byId.get(t.id)?.kanji, icon: byId.get(t.id)?.icon });
		}
	});
});
