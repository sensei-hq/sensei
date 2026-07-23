import { describe, expect, it } from 'vitest';
import {
	buildNavGroups,
	topGroups,
	manageGroups,
	hrefForRoute,
	WIRED_ROUTES,
	type NavGroup,
	type RouteId
} from './nav-items';

// The console nav IA (Chunk 5). `buildNavGroups` is the single source of truth
// for the left-nav structure: a personal zone on top ("Relay · you" + "Me") and
// the de-emphasized management groups below (marked `manage`). Kept pure so the
// ordering, grouping, and route wiring are unit-tested without rendering.

const groups: NavGroup[] = buildNavGroups();

function idsOf(gs: NavGroup[]): string[] {
	return gs.flatMap((g) => g.items.map((it) => it.id));
}

describe('buildNavGroups — IA structure', () => {
	it('puts the personal zone (Relay · you, Me) on top, before any management group', () => {
		const firstManageIndex = groups.findIndex((g) => g.manage);
		const personal = groups.slice(0, firstManageIndex);
		const personalNames = personal.map((g) => g.group);
		expect(personalNames).toEqual(['Relay · you', 'Me']);
		// Every group before the first management group is personal (not manage).
		expect(personal.every((g) => !g.manage)).toBe(true);
	});

	it('marks the management groups (Govern, Org, Clients, Trust) as manage', () => {
		const manage = manageGroups(groups);
		expect(manage.map((g) => g.group)).toEqual(['Govern', 'Org', 'Clients', 'Trust']);
		expect(manage.every((g) => g.manage === true)).toBe(true);
	});

	it('surfaces the wired Relay entry in the Relay · you group', () => {
		const relayGroup = groups.find((g) => g.group === 'Relay · you');
		expect(relayGroup).toBeDefined();
		const relay = relayGroup!.items.find((it) => it.to === 'relay');
		expect(relay).toBeDefined();
		expect(relay!.kanji).toBe('決');
	});

	it('keeps the Me group (teams 群 · contributions 共 · downstream 贈)', () => {
		const me = groups.find((g) => g.group === 'Me');
		expect(me).toBeDefined();
		expect(me!.items.map((it) => it.to)).toEqual(['teams', 'contributions', 'downstream']);
		expect(me!.items.map((it) => it.kanji)).toEqual(['群', '共', '贈']);
	});

	it('places Library 蔵 and Effective constitution 序 under Govern', () => {
		const govern = groups.find((g) => g.group === 'Govern');
		expect(govern).toBeDefined();
		const govIds = govern!.items.map((it) => it.to);
		expect(govIds).toEqual(['overview', 'triage', 'library', 'preview']);
	});
});

describe('buildNavGroups — every wired route stays reachable', () => {
	it('exposes an entry for each currently-wired console route', () => {
		const wiredInNav = new Set(
			groups.flatMap((g) => g.items.map((it) => it.to)).filter((to): to is RouteId => to != null)
		);
		for (const route of WIRED_ROUTES) {
			expect(wiredInNav.has(route)).toBe(true);
		}
	});

	it('does not drop any previously-wired destination', () => {
		// The full set that was reachable before the reframe must remain reachable.
		const expected: RouteId[] = [
			'overview',
			'triage',
			'relay',
			'library',
			'preview',
			'teams',
			'contributions',
			'downstream',
			'members',
			'identities',
			'policies',
			'health',
			'audit',
			'engagements',
			'incidents'
		];
		const wiredInNav = new Set(
			groups.flatMap((g) => g.items.map((it) => it.to)).filter((to): to is RouteId => to != null)
		);
		for (const route of expected) {
			expect(wiredInNav.has(route)).toBe(true);
		}
	});

	it('assigns a unique id to every nav item', () => {
		const ids = idsOf(groups);
		expect(new Set(ids).size).toBe(ids.length);
	});
});

describe('topGroups / manageGroups', () => {
	it('topGroups returns only the non-manage groups, in order', () => {
		expect(topGroups(groups).map((g) => g.group)).toEqual(['Relay · you', 'Me']);
	});

	it('manageGroups returns only the manage groups, in order', () => {
		expect(manageGroups(groups).map((g) => g.group)).toEqual(['Govern', 'Org', 'Clients', 'Trust']);
	});
});

describe('hrefForRoute — pure route resolution', () => {
	// Inject an identity resolver so we can assert the exact SvelteKit route ids
	// without the $app/paths virtual module.
	const resolve = (id: string) => id;

	it('resolves overview to the console index', () => {
		expect(hrefForRoute('overview', resolve)).toBe('/(console)/console');
	});

	it('resolves a sub-route to /console/{id}', () => {
		expect(hrefForRoute('triage', resolve)).toBe('/(console)/console/triage');
		expect(hrefForRoute('library', resolve)).toBe('/(console)/console/library');
		expect(hrefForRoute('incidents', resolve)).toBe('/(console)/console/incidents');
	});

	it('resolves every wired route to a non-empty href', () => {
		for (const route of WIRED_ROUTES) {
			expect(hrefForRoute(route, resolve)).toBeTruthy();
		}
	});
});
