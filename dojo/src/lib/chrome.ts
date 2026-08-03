// Bridge from the server view-model (`DojoOrg` — label role, capitalized kind,
// `url` tenant key, `id`) to the dojo kit chrome shapes (`KitDojo` / `KitOrg` —
// lowercase role/kind, `slug`, `route`). The kit's org switcher, context header,
// and role-scoped nav all speak the kit vocabulary; this module is the single
// mapping so the shell never hand-rolls a conversion. Pure and SSR-safe.

import type { DojoOrg } from './dojo-data';
import type { KitDojo, KitOrg, KitMe } from './components/kit/types';
import type { PersonalUser } from './personal-home-view';

/** Map a server role LABEL (Admin · Maintainer · Lead · Contributor · Member) to
 *  the lowercase rank key `navForOrg`/`K2_ROLE_RANK` expect. Contributor and
 *  Member are the base seat ⇒ `developer`; unknown labels floor to `developer`
 *  so a viewer never sees more than their rank. */
export function roleKey(label: string | null | undefined): string {
	switch ((label ?? '').toLowerCase()) {
		case 'admin':
			return 'admin';
		case 'lead':
			return 'lead';
		case 'maintainer':
			return 'maintainer';
		default:
			return 'developer';
	}
}

/** Map a capitalized org kind (Employer · Client · Community · Personal) to the
 *  lowercase kit kind, defaulting unknowns to `community`. */
export function kindKey(kind: string | null | undefined): string {
	const k = (kind ?? '').toLowerCase();
	return k === 'employer' || k === 'client' || k === 'personal' ? k : 'community';
}

/** Map a `DojoOrg` membership to the kit's `KitDojo` (switcher row). The slug is
 *  the org `id` (URL-clean); the route is the tenant key `url`; the pending
 *  count surfaces as the `needs` badge. */
export function toKitDojo(o: DojoOrg): KitDojo {
	return {
		slug: o.id,
		kanji: o.kanji,
		name: o.name,
		kind: kindKey(o.kind),
		role: roleKey(o.role),
		route: o.url,
		members: o.members,
		projects: o.projects,
		needs: o.pending
	};
}

/** Map a list of memberships, preserving order. */
export function toKitDojos(orgs: DojoOrg[]): KitDojo[] {
	return orgs.map(toKitDojo);
}

/** Map a `DojoOrg` to the `KitOrg` subset the context header / top bar consume. */
export function toKitOrg(o: DojoOrg): KitOrg {
	return {
		slug: o.id,
		kanji: o.kanji,
		name: o.name,
		kind: kindKey(o.kind),
		role: roleKey(o.role),
		route: o.url
	};
}

/** Resolve a picked switcher slug back to its `DojoOrg` (for the tenant switch). */
export function orgBySlug(orgs: DojoOrg[], slug: string): DojoOrg | undefined {
	return orgs.find((o) => o.id === slug);
}

/** The viewer identity the kit chrome monogram binds to — the display name, or
 *  the email local-part, or "You". Mirrors `soloIdentity`'s name resolution. */
export function kitMe(user: PersonalUser | undefined): KitMe {
	const name = user?.name?.trim();
	const email = user?.email?.trim();
	return { name: name || (email ? email.split('@')[0] : 'You') };
}
