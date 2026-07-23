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
}

/** The three role-surface tabs (mockup `tabs` in ScrRoleSurfaces). Members &
 *  Roles · Policies · Audit — in nav order. */
export const ROLE_SURFACE_TABS: readonly RoleSurfaceTab[] = [
	{
		id: 'members',
		label: 'Members & Roles',
		icon: 'users-group-rounded',
		eyebrow: 'admin',
		title: 'Members & Roles',
		cta: 'Invite'
	},
	{
		id: 'policies',
		label: 'Policies',
		icon: 'shield-check',
		eyebrow: 'admin',
		title: 'Role policies',
		cta: 'New policy'
	},
	{
		id: 'audit',
		label: 'Audit',
		icon: 'clipboard-list',
		eyebrow: 'admin',
		title: 'Audit log',
		cta: 'Export'
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
