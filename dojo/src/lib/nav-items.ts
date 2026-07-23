// The console left-nav IA (Chunk 5 — nav reframe). The single source of truth
// for the grouped destinations ConsoleNav renders, extracted from the component
// so the ordering, grouping, and route wiring are pure and unit-testable.
//
// Structure (mockup DojoRoleNav + RELAY_GROUP): a PERSONAL zone on top — a
// "Relay · you" group (the away-from-keyboard Relay surface) and the "Me" group
// (the individual contributor's read-mostly seat: teams 群 · contributions 共 ·
// downstream 贈) — then the MANAGEMENT groups (Govern · Org · Clients · Trust),
// marked `manage` so the nav de-emphasizes them and sits them below a divider.
// Every currently-wired console route stays reachable; role-aware filtering is
// deferred to Chunk 6 (needs the role model), so all management groups show.

/**
 * A wired destination's route id. Each maps to a `/console[/…]` sub-route via
 * `hrefForRoute`. This is the exhaustive set of console routes the nav links to.
 */
export type RouteId =
	| 'overview'
	| 'triage'
	| 'relay'
	| 'library'
	| 'preview'
	| 'teams'
	| 'contributions'
	| 'downstream'
	| 'members'
	| 'identities'
	| 'policies'
	| 'health'
	| 'audit'
	| 'engagements'
	| 'incidents';

/** Every route id the nav wires — the reachability contract the specs pin. */
export const WIRED_ROUTES: readonly RouteId[] = [
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

export interface NavItem {
	id: string;
	kanji: string;
	label: string;
	/** A wired destination's route id; absent → a "soon" placeholder. */
	to?: RouteId;
	badge?: number;
}

export interface NavGroup {
	group: string;
	items: NavItem[];
	/**
	 * Management group — de-emphasized and placed below the personal zone's
	 * divider. Absent/false for the personal groups (Relay · you, Me).
	 */
	manage?: boolean;
}

/**
 * Build the nav groups. `badges` supplies live counts (e.g. pending
 * contributions) so the pure structure stays free of view-state imports; the
 * component passes them in. Personal groups first, management groups (marked
 * `manage`) after.
 */
export function buildNavGroups(badges: { pendingContributions?: number } = {}): NavGroup[] {
	const pending = badges.pendingContributions || undefined;
	return [
		{
			// The away-from-keyboard Relay surface (mockup RELAY_GROUP). Only the
			// wired Relay entry (決) is live today; Projects/Chat land in a later
			// pass, so they render as "soon" placeholders.
			group: 'Relay · you',
			items: [
				{ id: 'relay-projects', kanji: '場', label: 'Projects' },
				{ id: 'relay', kanji: '決', label: 'Relay', to: 'relay' },
				{ id: 'relay-chat', kanji: '話', label: 'Chat' }
			]
		},
		{
			// The individual contributor's read-mostly seat (mockup DEV_NAV),
			// reachable solo (no join-gate, DJ1).
			group: 'Me',
			items: [
				{ id: 'teams', kanji: '群', label: 'My teams', to: 'teams' },
				{ id: 'contributions', kanji: '共', label: 'My contributions', to: 'contributions', badge: pending },
				{ id: 'downstream', kanji: '贈', label: 'For me', to: 'downstream' }
			]
		},
		{
			group: 'Govern',
			manage: true,
			items: [
				{ id: 'overview', kanji: '全', label: 'Overview', to: 'overview' },
				{ id: 'triage', kanji: '門', label: 'Triage', to: 'triage' },
				{ id: 'library', kanji: '蔵', label: 'Library', to: 'library' },
				{ id: 'preview', kanji: '序', label: 'Effective constitution', to: 'preview' }
			]
		},
		{
			group: 'Org',
			manage: true,
			items: [
				{ id: 'members', kanji: '任', label: 'Members & roles', to: 'members' },
				{ id: 'identities', kanji: '鍵', label: 'Identities', to: 'identities' },
				{ id: 'policies', kanji: '規', label: 'Policies', to: 'policies' }
			]
		},
		{
			group: 'Clients',
			manage: true,
			items: [
				{ id: 'engagements', kanji: '客', label: 'Engagements', to: 'engagements' },
				{ id: 'incidents', kanji: '警', label: 'Incidents', to: 'incidents' }
			]
		},
		{
			group: 'Trust',
			manage: true,
			items: [
				{ id: 'health', kanji: '観', label: 'Health', to: 'health' },
				{ id: 'audit', kanji: '録', label: 'Audit trail', to: 'audit' }
			]
		}
	];
}

/** The personal groups (top zone) in order. */
export function topGroups(groups: NavGroup[]): NavGroup[] {
	return groups.filter((g) => !g.manage);
}

/** The management groups (below the divider) in order. */
export function manageGroups(groups: NavGroup[]): NavGroup[] {
	return groups.filter((g) => g.manage);
}

/**
 * The exact SvelteKit route paths the console nav resolves. Kept as a literal
 * union so `hrefForRoute` can accept the real `$app/paths` `resolve` (which types
 * its argument as valid route ids) AND a plain `(id: string) => string` test stub
 * — the latter is contravariantly assignable to a resolver of these literals.
 */
export type ConsoleRoutePath =
	| '/(console)/console'
	| '/(console)/console/triage'
	| '/(console)/console/relay'
	| '/(console)/console/library'
	| '/(console)/console/preview'
	| '/(console)/console/teams'
	| '/(console)/console/contributions'
	| '/(console)/console/downstream'
	| '/(console)/console/members'
	| '/(console)/console/identities'
	| '/(console)/console/policies'
	| '/(console)/console/health'
	| '/(console)/console/audit'
	| '/(console)/console/engagements'
	| '/(console)/console/incidents';

/** A resolver from a console route path to its href (SvelteKit `resolve`). */
export type ConsoleRouteResolver = (id: ConsoleRoutePath) => string;

/**
 * Route path for a wired destination. Overview is the console index; every other
 * wired id is a `/console/{id}` sub-route. Takes the SvelteKit `resolve` as an
 * argument so it's pure and testable without the `$app/paths` virtual module —
 * every arm passes a literal route id so `resolve`'s build-time route check still
 * holds when the component supplies the real resolver.
 */
export function hrefForRoute(to: RouteId, resolve: ConsoleRouteResolver): string {
	switch (to) {
		case 'overview':
			return resolve('/(console)/console');
		case 'triage':
			return resolve('/(console)/console/triage');
		case 'relay':
			return resolve('/(console)/console/relay');
		case 'library':
			return resolve('/(console)/console/library');
		case 'preview':
			return resolve('/(console)/console/preview');
		case 'teams':
			return resolve('/(console)/console/teams');
		case 'contributions':
			return resolve('/(console)/console/contributions');
		case 'downstream':
			return resolve('/(console)/console/downstream');
		case 'members':
			return resolve('/(console)/console/members');
		case 'identities':
			return resolve('/(console)/console/identities');
		case 'policies':
			return resolve('/(console)/console/policies');
		case 'health':
			return resolve('/(console)/console/health');
		case 'audit':
			return resolve('/(console)/console/audit');
		case 'engagements':
			return resolve('/(console)/console/engagements');
		case 'incidents':
			return resolve('/(console)/console/incidents');
	}
}
