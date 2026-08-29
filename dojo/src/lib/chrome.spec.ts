import { describe, expect, it } from 'vitest';
import {
	roleKey,
	kindKey,
	toKitDojo,
	toKitDojos,
	toKitOrg,
	orgBySlug,
	kitMe
} from './chrome';
import type { DojoOrg } from './dojo-data';

// Bridge from the server view-model (`DojoOrg`: label role, capitalized kind,
// `url` tenant key, `id`) to the kit chrome shapes (`KitDojo` / `KitOrg`:
// lowercase role/kind, `slug`, `route`). Kept pure so the mapping the shell's
// org switcher + context header bind to is unit-tested without a component.

const acme: DojoOrg = {
	id: 'acme',
	kanji: '社',
	name: 'Acme Corp',
	kind: 'Organization',
	host: 'self',
	url: 'dojo.acme.internal',
	role: 'Admin',
	from: 'GitHub · org owner',
	members: 48,
	pending: 7
};
const globex: DojoOrg = {
	id: 'globex',
	kanji: '社',
	name: 'Globex',
	kind: 'Organization',
	host: 'saas',
	url: 'github/globex',
	role: 'Maintainer',
	from: 'GitHub · repo admin',
	members: 12,
	pending: 2
};

describe('roleKey — label → lowercase role rank key', () => {
	it('lowercases the known role labels', () => {
		expect(roleKey('Admin')).toBe('admin');
		expect(roleKey('Maintainer')).toBe('maintainer');
		expect(roleKey('Lead')).toBe('lead');
	});

	it('maps Contributor/Member → developer (the base rank)', () => {
		expect(roleKey('Contributor')).toBe('developer');
		expect(roleKey('Member')).toBe('developer');
	});

	it('floors an unknown role to developer', () => {
		expect(roleKey('Overlord')).toBe('developer');
		expect(roleKey(undefined)).toBe('developer');
	});
});

describe('kindKey — capitalized kind → lowercase kit kind', () => {
	it('maps to the two buckets that exist', () => {
		expect(kindKey('Organization')).toBe('organization');
		expect(kindKey('Personal')).toBe('personal');
	});

	it('groups a row still carrying the old `employer` tag as an organisation', () => {
		// `dojo.memberships.kind` is a NOT NULL enum that still contains employer /
		// client / community, so rows with those values exist. None of them may
		// fall into a bucket `DOJO_GROUPS` no longer has, or the dōjō vanishes from
		// the screen entirely rather than merely being mislabelled.
		for (const stale of ['employer', 'client', 'community', 'Alliance', '', null]) {
			expect(kindKey(stale as never)).toBe('organization');
		}
	});
});

describe('toKitDojo / toKitOrg — view-model mapping', () => {
	it('maps a DojoOrg to a KitDojo (slug=id, route=url, lowercase role/kind)', () => {
		const d = toKitDojo(globex);
		expect(d).toMatchObject({
			slug: 'globex',
			kanji: '社',
			name: 'Globex',
			kind: 'organization',
			role: 'maintainer',
			route: 'github/globex',
			members: 12
		});
	});

	it('carries the pending count into `needs`', () => {
		expect(toKitDojo(acme).needs).toBe(7);
	});

	it('maps a list preserving order', () => {
		expect(toKitDojos([acme, globex]).map((d) => d.slug)).toEqual(['acme', 'globex']);
	});

	it('toKitOrg produces the KitOrg subset', () => {
		expect(toKitOrg(acme)).toEqual({
			slug: 'acme',
			kanji: '社',
			name: 'Acme Corp',
			kind: 'organization',
			role: 'admin',
			route: 'dojo.acme.internal'
		});
	});
});

describe('orgBySlug — resolve a picked slug back to its DojoOrg', () => {
	it('finds the membership whose id matches the slug', () => {
		expect(orgBySlug([acme, globex], 'globex')).toBe(globex);
	});

	it('returns undefined for an unknown slug', () => {
		expect(orgBySlug([acme, globex], 'initech')).toBeUndefined();
	});
});

describe('kitMe — the viewer monogram source', () => {
	it('carries the display name', () => {
		expect(kitMe({ name: 'Rin Saito', email: 'rin@acme.dev' }).name).toBe('Rin Saito');
	});

	it('falls back to the email local-part, then "You"', () => {
		expect(kitMe({ email: 'jerry.thomas@acme.dev' }).name).toBe('jerry.thomas');
		expect(kitMe(undefined).name).toBe('You');
	});
});
