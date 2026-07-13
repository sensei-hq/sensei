// Static demo data for the Dōjō SaaS entry screens (sign-in + org picker).
//
// The mockup (docs/mockups/Sensei/lib/dojo-saas.jsx) reads these from a global
// `window.DOJO`. R6 is a static-render scaffold, so this is fixed placeholder
// content — the live values come from PUBLIC_DOJO_API_URL once R9–R11 wire the
// console screens. Kept in a plain module so it can be imported in SSR/prerender
// and unit tests without any runtime dependency.

export interface DojoMetrics {
	contribWeek: number;
	contribSpark: number[];
	approvedWeek: number;
	dereferenced: number;
	adoptionLift: number;
}

export const metrics: DojoMetrics = {
	contribWeek: 34,
	contribSpark: [8, 11, 9, 14, 12, 18, 22],
	approvedWeek: 12,
	dereferenced: 47,
	adoptionLift: 0.11
};

export type OrgKind = 'Employer' | 'Client' | 'Community' | 'Personal';
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
	members: number;
	pending: number;
	last?: boolean;
}

export const orgs: DojoOrg[] = [
	{
		id: 'acme',
		kanji: '社',
		name: 'Acme Corp',
		kind: 'Employer',
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
		kind: 'Client',
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
		kind: 'Client',
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
		kind: 'Community',
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
	Employer: 'text-ink-soft',
	Client: 'text-accent',
	Community: 'text-success',
	Personal: 'text-ink-mute'
};
