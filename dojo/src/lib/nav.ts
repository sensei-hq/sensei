// The dojo two-nav information architecture (chunk 1 of the rebuild).
//
// Ported faithfully from the finalized wired mockup
// (docs/mockups/Sensei/lib/dojo/dojo2-app.jsx): a PERSONAL zone (`NAV_YOU`,
// the signed-in landing) and a role-scoped ORG zone (`NAV_ORG_BASE` filtered by
// `K2_ROLE_RANK` through `navForOrg`). Kept as a pure module — no Svelte, no
// `$app/*` — so the grouping, additive-role gating, and route wiring unit-test
// without rendering and stay the ONE source of truth the shell + placeholders
// bind to.
//
// The nav-item / -group shapes are the kit's (`KitNavGroup` / `KitNavItem`), so
// the AppShell + NavPane + TabBar consume these directly.

import type { KitNavGroup, KitNavItem } from './components/kit/types';

/* ── personal zone (NAV_YOU) ────────────────────────────────────────────── */

/** The personal nav. One **Inbox** holds every in-flight session — approve /
 *  decide / chat are actions inside a session's detail, not surfaces of their
 *  own. The Inbox is the signed-in landing (`/you`); the rest route to
 *  `/you/{id}`. */
export const NAV_YOU: KitNavGroup[] = [
	{
		group: 'Work',
		items: [
			{ id: 'inbox', icon: 'inbox', label: 'Inbox' },
			{ id: 'projects', icon: 'folder', label: 'Projects' }
		]
	},
	{
		group: 'Govern',
		items: [
			{ id: 'rules', icon: 'scale', label: 'Constitution' },
			{ id: 'packs', icon: 'box', label: 'Rule packs' }
		]
	},
	{
		group: 'Dōjōs',
		items: [
			{ id: 'dojos', icon: 'users-group-two-rounded', label: 'My dōjōs' },
			// Sharing is a DECISION surface, not a listing: three repositories sat
			// at `not_elected_user` — refusing for want of a choice nobody could
			// make, because the election had a write path and no toggle.
			{ id: 'sharing', icon: 'share-circle', label: 'Sharing' },
			{ id: 'contributions', icon: 'upload-square', label: 'Contributions' }
		]
	}
];

/* ── org zone (role-scoped NAV_ORG) ─────────────────────────────────────── */

/** A NAV_ORG group with the additive role floor it unlocks at (a group with no
 *  `role` is always shown). */
export interface OrgNavGroup extends KitNavGroup {
	/** The role a viewer must reach for this group to appear. Absent ⇒ always. */
	role?: string;
}

/** The full org nav before role filtering (mockup `NAV_ORG_BASE`). */
export const NAV_ORG_BASE: OrgNavGroup[] = [
	{
		group: 'Overview',
		items: [
			{ id: 'home', icon: 'buildings-2', label: 'Home' },
			{ id: 'ladder', icon: 'scale', label: 'Constitution' },
			{ id: 'projects', icon: 'folder', label: 'Projects' }
		]
	},
	{
		group: 'Govern',
		role: 'maintainer',
		items: [
			{ id: 'triage', icon: 'inbox', label: 'Triage' },
			{ id: 'approvals', icon: 'clipboard-check', label: 'Approvals' },
			{ id: 'knowledge', icon: 'book-2', label: 'Knowledge' }
		]
	},
	{
		group: 'Clients',
		role: 'lead',
		items: [
			{ id: 'engagements', icon: 'case-round', label: 'Engagements' },
			{ id: 'incidents', icon: 'shield-warning', label: 'Incidents' },
			{ id: 'clientaudit', icon: 'document-text', label: 'Client audit' }
		]
	},
	{
		group: 'Admin',
		role: 'admin',
		items: [
			{ id: 'members', icon: 'users-group-rounded', label: 'Members & Roles' },
			{ id: 'scopes', icon: 'shield-check', label: 'Scopes & policies' },
			{ id: 'identity', icon: 'key', label: 'Identity & SSO' },
			{ id: 'audit', icon: 'clipboard-list', label: 'Audit' },
			{ id: 'health', icon: 'pulse', label: 'Health / Monitor' },
			{ id: 'billing', icon: 'card', label: 'Plan & billing' }
		]
	}
];

/** Additive role rank — a group shows when the viewer's role reaches its floor
 *  (mockup `K2_ROLE_RANK`). */
export const K2_ROLE_RANK: Record<string, number> = {
	developer: 0,
	maintainer: 1,
	lead: 2,
	admin: 3
};

/** The rank of a role, flooring an unknown/absent role to developer (0). */
export function rankOf(role: string | null | undefined): number {
	return role != null && K2_ROLE_RANK[role] != null ? K2_ROLE_RANK[role] : 0;
}

/** The org nav groups visible to a viewer of the given role — additive: a group
 *  shows when the viewer's rank reaches its floor (mockup `navForOrg`). Returned
 *  as plain `KitNavGroup[]` (the `role` floor is an internal gate). */
export function navForOrg(role: string | null | undefined): KitNavGroup[] {
	const rank = rankOf(role);
	return NAV_ORG_BASE.filter((g) => !g.role || rank >= K2_ROLE_RANK[g.role]).map((g) => ({
		group: g.group,
		items: g.items
	}));
}

/** The nav groups for a context: personal (`role == null`) ⇒ NAV_YOU, else the
 *  role-scoped org groups. */
export function navGroupsFor(role: string | null | undefined): KitNavGroup[] {
	return role == null ? NAV_YOU : navForOrg(role);
}

/* ── mobile tabs ────────────────────────────────────────────────────────── */

/** Personal bottom tabs — Inbox leads (the landing). */
export const TABS_YOU: KitNavItem[] = [
	{ id: 'inbox', icon: 'inbox', label: 'Inbox' },
	{ id: 'projects', icon: 'folder', label: 'Projects' },
	{ id: 'rules', icon: 'scale', label: 'Rules' },
	{ id: 'dojos', icon: 'users-group-two-rounded', label: 'Dōjōs' }
];

/** Org bottom tabs (mockup `TABS_ORG`). */
export const TABS_ORG: KitNavItem[] = [
	{ id: 'home', icon: 'buildings-2', label: 'Home' },
	{ id: 'projects', icon: 'folder', label: 'Projects' },
	{ id: 'ladder', icon: 'scale', label: 'Rules' },
	{ id: 'members', icon: 'users-group-rounded', label: 'Members' }
];

/** The bottom tabs for a context: personal (`role == null`) ⇒ TABS_YOU. */
export function tabsFor(role: string | null | undefined): KitNavItem[] {
	return role == null ? TABS_YOU : TABS_ORG;
}

/* ── section reachability + route wiring ─────────────────────────────────── */

/** Every non-landing personal section id — the `[section]` values `/you/…`
 *  serves (the landing `inbox` is the index, not a section). */
export const YOU_SECTIONS: readonly string[] = NAV_YOU.flatMap((g) =>
	g.items.map((it) => it.id)
).filter((id) => id !== 'inbox');

/** Every non-home org section id — the `[section]` values `/org/{slug}/…`
 *  serves a placeholder for (the org home is the index, not a section). Also
 *  includes `audit`, which the mockup nav shares between Admin's item and the
 *  role-surfaces tab. */
export const ORG_SECTIONS: readonly string[] = NAV_ORG_BASE.flatMap((g) =>
	g.items.map((it) => it.id)
).filter((id) => id !== 'home');

const YOU_SECTION_SET = new Set(YOU_SECTIONS);
const ORG_SECTION_SET = new Set(ORG_SECTIONS);

/** The personal landing route (`inbox`) or a section route `/you/{section}`. */
export function youHref(section?: string): string {
	return !section || section === 'inbox' ? '/you' : `/you/${section}`;
}

/** The org home route (`home`) or a section route `/org/{slug}/{section}`. */
export function orgHref(slug: string, section?: string): string {
	return !section || section === 'home' ? `/org/${slug}` : `/org/${slug}/${section}`;
}

/** The active personal section for a URL pathname — `inbox` for the landing or
 *  any unknown tail (so nav highlighting degrades to the Inbox). */
export function sectionFromYouPath(pathname: string): string {
	const seg = pathname.replace(/^\/you\/?/, '').split('/')[0];
	return seg && YOU_SECTION_SET.has(seg) ? seg : 'inbox';
}

/** The active org section for a URL pathname — `home` for the org index or any
 *  unknown tail. Expects `/org/{slug}[/{section}]`. */
export function sectionFromOrgPath(pathname: string): string {
	const seg = pathname.replace(/^\/org\/[^/]+\/?/, '').split('/')[0];
	return seg && ORG_SECTION_SET.has(seg) ? seg : 'home';
}

const YOU_LABELS = new Map(NAV_YOU.flatMap((g) => g.items.map((it) => [it.id, it.label] as const)));
const ORG_LABELS = new Map(
	NAV_ORG_BASE.flatMap((g) => g.items.map((it) => [it.id, it.label] as const))
);

/** The human label for a section id in a context (personal ⇒ NAV_YOU labels,
 *  org ⇒ NAV_ORG labels), falling back to a title-cased id when unknown. Backs
 *  the placeholder screens' headers. */
export function labelForSection(section: string, context: 'you' | 'org'): string {
	const found = (context === 'org' ? ORG_LABELS : YOU_LABELS).get(section);
	return found ?? section.charAt(0).toUpperCase() + section.slice(1);
}
