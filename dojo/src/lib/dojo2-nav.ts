// The dojo2 two-nav information architecture (chunk 1 of the rebuild).
//
// Ported faithfully from the finalized wired mockup
// (docs/mockups/Sensei/lib/dojo2/dojo2-app.jsx): a PERSONAL zone (`NAV_YOU`,
// the signed-in landing) and a role-scoped ORG zone (`NAV_ORG_BASE` filtered by
// `K2_ROLE_RANK` through `navForOrg`). Kept as a pure module — no Svelte, no
// `$app/*` — so the grouping, additive-role gating, and route wiring unit-test
// without rendering and stay the ONE source of truth the shell + placeholders
// bind to.
//
// The nav-item / -group shapes are the kit's (`KitNavGroup` / `KitNavItem`), so
// the AppShell + MobileShell + NavPane consume these directly.

import type { KitNavGroup, KitNavItem } from './components/kit/types';

/* ── personal zone (NAV_YOU) ────────────────────────────────────────────── */

/** The personal nav — every dōjō, across the four planes. The signed-in
 *  landing (`work`) leads; the rest route to `/you/{id}` placeholders this
 *  chunk. Ported 1:1 from the mockup `NAV_YOU`. */
export const NAV_YOU: KitNavGroup[] = [
	{
		group: 'Work',
		items: [
			{ id: 'work', icon: 'widget-4', label: 'Your work' },
			{ id: 'runs', icon: 'eye', label: 'Live runs', badge: 3 },
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
		group: 'Relay',
		items: [
			{ id: 'approve', icon: 'check-circle', label: 'Approve', badge: 2 },
			{ id: 'decide', icon: 'checklist-minimalistic', label: 'Decide', badge: 2 },
			{ id: 'chat', icon: 'chat-round-line', label: 'Chat' }
		]
	},
	{
		group: 'Dōjōs',
		items: [
			{ id: 'dojos', icon: 'users-group-two-rounded', label: 'My dōjōs' },
			{ id: 'contributions', icon: 'upload-square', label: 'Contributions', badge: 1 }
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
			{ id: 'projects', icon: 'folder', label: 'Projects', badge: 4 }
		]
	},
	{
		group: 'Govern',
		role: 'maintainer',
		items: [
			{ id: 'triage', icon: 'inbox', label: 'Triage', badge: 5 },
			{ id: 'approvals', icon: 'clipboard-check', label: 'Approvals', badge: 2 },
			{ id: 'knowledge', icon: 'book-2', label: 'Knowledge' }
		]
	},
	{
		group: 'Clients',
		role: 'lead',
		items: [
			{ id: 'engagements', icon: 'case-round', label: 'Engagements' },
			{ id: 'incidents', icon: 'shield-warning', label: 'Incidents', badge: 1 },
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

/** Personal bottom tabs (mockup `TABS_YOU`). */
export const TABS_YOU: KitNavItem[] = [
	{ id: 'work', icon: 'widget-4', label: 'Work' },
	{ id: 'runs', icon: 'eye', label: 'Runs', badge: 3 },
	{ id: 'decide', icon: 'checklist-minimalistic', label: 'Needs', badge: 4 },
	{ id: 'chat', icon: 'chat-round-line', label: 'Chat' }
];

/** Org bottom tabs (mockup `TABS_ORG`). */
export const TABS_ORG: KitNavItem[] = [
	{ id: 'home', icon: 'buildings-2', label: 'Home' },
	{ id: 'projects', icon: 'folder', label: 'Projects', badge: 4 },
	{ id: 'ladder', icon: 'scale', label: 'Rules' },
	{ id: 'members', icon: 'users-group-rounded', label: 'Members' }
];

/** The bottom tabs for a context: personal (`role == null`) ⇒ TABS_YOU. */
export function tabsFor(role: string | null | undefined): KitNavItem[] {
	return role == null ? TABS_YOU : TABS_ORG;
}

/* ── section reachability + route wiring ─────────────────────────────────── */

/** Every non-landing personal section id — the `[section]` values `/you/…`
 *  serves a placeholder for (the landing `work` is the index, not a section). */
export const YOU_SECTIONS: readonly string[] = NAV_YOU.flatMap((g) =>
	g.items.map((it) => it.id)
).filter((id) => id !== 'work');

/** Every non-home org section id — the `[section]` values `/org/{slug}/…`
 *  serves a placeholder for (the org home is the index, not a section). Also
 *  includes `audit`, which the mockup nav shares between Admin's item and the
 *  role-surfaces tab. */
export const ORG_SECTIONS: readonly string[] = NAV_ORG_BASE.flatMap((g) =>
	g.items.map((it) => it.id)
).filter((id) => id !== 'home');

const YOU_SECTION_SET = new Set(YOU_SECTIONS);
const ORG_SECTION_SET = new Set(ORG_SECTIONS);

/** The personal landing route (`work`) or a section route `/you/{section}`. */
export function youHref(section?: string): string {
	return !section || section === 'work' ? '/you' : `/you/${section}`;
}

/** The org home route (`home`) or a section route `/org/{slug}/{section}`. */
export function orgHref(slug: string, section?: string): string {
	return !section || section === 'home' ? `/org/${slug}` : `/org/${slug}/${section}`;
}

/** The active personal section for a URL pathname — `work` for the landing or
 *  any unknown tail (so nav highlighting degrades to the landing). */
export function sectionFromYouPath(pathname: string): string {
	const seg = pathname.replace(/^\/you\/?/, '').split('/')[0];
	return seg && YOU_SECTION_SET.has(seg) ? seg : 'work';
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
