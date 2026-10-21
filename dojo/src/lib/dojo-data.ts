// Static demo data for the Dōjō SaaS entry screens (sign-in + org picker).
//
// The mockup (docs/mockups/Sensei/lib/dojo-saas.jsx) reads these from a global
// `window.DOJO`. R6 is a static-render scaffold, so this is fixed placeholder
// content — the live values come from PUBLIC_DOJO_API_URL once R9–R11 wire the
// console screens. Kept in a plain module so it can be imported in SSR/prerender
// and unit tests without any runtime dependency.

/** What a dōjō IS, derived from `dojo.tenants.origin` — the forge's own answer.
 *
 *  There was a fourth-way taxonomy here (Employer · Client · Community) and it
 *  There was a relationship taxonomy here — Employer · Client · Community — and
 *  it is gone for two independent reasons.
 *
 *  It could not be DERIVED. Provisioning tagged
 *  every discovered org `employer` because GitHub cannot say which of your
 *  organisations employs you — live, that made an employer, a personal venture
 *  and a product org all "Employer".
 *
 *  And it earned nothing. The one behaviour hanging off it was
 *  `isAnonymizedMembership` — client dōjōs anonymised a contributor's sends.
 *  ALL insights are anonymised, so there was never anything client-specific to
 *  protect; the function had no callers and the distinction was decorative.
 *
 *  What remains is what the forge actually states — organisation, or person.
 *  There is no third kind: no dōjō is made by hand (#307 D11). */
export type OrgKind = 'Organization' | 'Personal';
export type OrgHost = 'self' | 'saas';

export interface DojoOrg {
	id: string;
	kanji: string;
	name: string;
	kind: OrgKind;
	host: OrgHost;
	url: string;
	role: string;
	from: string;
	// Counts are OPTIONAL — undefined when not (yet) computed. The row omits the
	// chip rather than showing a fabricated 0 (honest-empty, not a masked zero).
	members?: number;
	projects?: number;
	pending?: number;
	last?: boolean;
}

/** `dojo.tenants.origin` → the app's `OrgKind`, or null for any other value.
 *
 *  A dōjō is an organization's or a person's — nothing else (#307 D11). The
 *  kind comes from the origin, which the forge states, never from the
 *  unsubstantiated membership `kind` tag. Only an exact `organization` is an
 *  Organization — the PROVEN kind — and an unrecognised value is no kind at all:
 *  the caller says so rather than showing a guess. */
export function originToOrgKind(origin: string | null | undefined): OrgKind | null {
	if (origin === 'personal') return 'Personal';
	if (origin === 'organization') return 'Organization';
	return null;
}

/** The identity glyph per kind (社 organisation · 己 personal), matching the
 *  ladder kanji in `constitution-map`. */
export function orgKindKanji(kind: OrgKind): string {
	return kind === 'Personal' ? '己' : '社';
}

export const orgs: DojoOrg[] = [
	{
		id: 'acme',
		kanji: '社',
		name: 'Acme Corp',
		kind: 'Organization',
		host: 'self',
		url: 'dojo.acme.internal',
		role: 'Org admin',
		from: 'GitHub · org owner',
		members: 48,
		pending: 7,
		last: true
	},
	{
		id: 'globex',
		kanji: '客',
		name: 'Globex',
		kind: 'Organization',
		host: 'saas',
		url: 'github/globex',
		role: 'Maintainer',
		from: 'GitHub · repo admin',
		members: 12,
		pending: 2
	},
	{
		id: 'initech',
		kanji: '客',
		name: 'Initech',
		kind: 'Organization',
		host: 'saas',
		url: 'other/initech',
		role: 'Contributor',
		from: 'Magic link · invited',
		members: 6,
		pending: 0
	},
	{
		id: 'rustco',
		kanji: '群',
		name: 'Rust Guild',
		kind: 'Organization',
		host: 'saas',
		url: 'github/rust-guild',
		role: 'Read-only',
		from: 'GitHub · member',
		members: 410,
		pending: 0
	},
	{
		id: 'self',
		kanji: '己',
		name: 'Personal',
		kind: 'Personal',
		host: 'saas',
		url: 'github/keiko-t',
		role: 'Owner',
		from: 'GitHub · you',
		members: 1,
		pending: 0
	}
];

// kind → semantic token role for the chip tint (mockup used raw --ink-2/--accent
// /--success/--ink-3; map onto named tokens).
export const kindToneClass: Record<OrgKind, string> = {
	Organization: 'text-ink-soft',
	Personal: 'text-ink-mute'
};
