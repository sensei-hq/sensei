// Pure presentation helpers for the admin Members & Roles / Policies / Audit
// surface (ScrRoleSurfaces, mockup). The surface is ONE screen with three tabs;
// the org nav shares it between two ids — `members` opens the Members tab and
// `audit` opens the Audit tab (the mockup passes the tab down as a prop).
// Side-effect-free (a tab id in → its chrome out) so the tab config + the
// nav-id → tab mapping unit-test without a DOM and the screen stays declarative.

/** One tab on the role-surfaces screen — its nav id, label, icon, and the
 *  header eyebrow/title + primary-CTA label it drives. */
export interface RoleSurfaceTab {
	id: string;
	label: string;
	icon: string;
	/** The SectionHead eyebrow suffix (after "{org} · "). */
	eyebrow: string;
	/** The SectionHead title. */
	title: string;
	/** The primary-CTA label for this tab. */
	cta: string;
	/** The SectionHead description — what this tab is. */
	description: string;
}

/** The role-surface tabs (mockup `tabs` in ScrRoleSurfaces). Members & Roles ·
 *  Viewers (#307 D8 — people invited as viewers, not members) · Teams (D7) ·
 *  Policies · Audit. */
export const ROLE_SURFACE_TABS: readonly RoleSurfaceTab[] = [
	{
		id: 'members',
		label: 'Members & Roles',
		icon: 'users-group-rounded',
		eyebrow: 'admin',
		title: 'Members & Roles',
		cta: 'Invite',
		description:
			'Roles are additive and mirror the forge, per repository. contributor → maintainer → lead → admin. A role only ever adds capability.'
	},
	{
		id: 'viewers',
		label: 'Viewers',
		icon: 'eye',
		eyebrow: 'admin',
		title: 'Viewers',
		cta: 'Invite',
		description:
			'Managers and clients invited to view — not members, with no repository access. They read only the projects you grant them.'
	},
	{
		id: 'teams',
		label: 'Teams',
		icon: 'users-group-two-rounded',
		eyebrow: 'admin',
		title: 'Teams',
		cta: 'New team',
		description:
			'Groups of members for reports by team and for granting a project to many people at once. A team grants no access — repository roles do.'
	},
	{
		id: 'policies',
		label: 'Policies',
		icon: 'shield-check',
		eyebrow: 'admin',
		title: 'Role policies',
		cta: 'New policy',
		description: 'Roles are additive — a role only ever adds capability. What each role may do in this dōjō.'
	},
	{
		id: 'audit',
		label: 'Audit',
		icon: 'clipboard-list',
		eyebrow: 'admin',
		title: 'Audit log',
		cta: 'Export',
		description: 'Who changed what in this dōjō, and when.'
	}
];

/** Resolve a tab by id, falling back to the first (Members) for an unknown id —
 *  so a hand-typed section never lands on a blank surface (mockup
 *  `tabs.find(...) || tabs[0]`). */
export function resolveTab(id: string): RoleSurfaceTab {
	return ROLE_SURFACE_TABS.find((t) => t.id === id) ?? ROLE_SURFACE_TABS[0];
}

/** The tab a nav section id opens: the Admin nav shares the surface between the
 *  `members` item (→ Members tab) and the `audit` item (→ Audit tab). Any other
 *  section id defaults to Members. This is the one place the nav-id → tab
 *  contract lives. */
export function tabForSection(section: string): string {
	return section === 'audit' ? 'audit' : 'members';
}
