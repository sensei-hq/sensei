import { describe, expect, it } from 'vitest';
// The kavach config is a plain ESM default export (no virtual modules), so it
// imports cleanly under vitest. Import from the dojo root (../../ from src/lib).
import config from '../../kavach.config.js';

// Guards the landing cutover: post-auth `home` must be the dojo personal home
// (`/you`), and the migration keeps BOTH the dojo zones (`/you`, `/org`) and the
// legacy fallbacks (`/orgs`, `/console`) reachable for a signed-in session.
describe('kavach config — landing cutover', () => {
	it('sends authenticated users to the dojo personal home (/you)', () => {
		expect(config.routes.home).toBe('/you');
	});

	it('keeps the dojo zones and legacy fallbacks reachable for signed-in roles', () => {
		const guarded = new Set(
			config.rules.filter((r) => r.roles === '*').map((r) => r.path)
		);
		// dojo zones
		expect(guarded.has('/you')).toBe(true);
		expect(guarded.has('/org')).toBe(true);
		// legacy fallbacks kept during migration (org picker + old console)
		expect(guarded.has('/orgs')).toBe(true);
		expect(guarded.has('/console')).toBe(true);
	});

	it('keeps the pre-auth surfaces public', () => {
		const publics = new Set(config.rules.filter((r) => r.public).map((r) => r.path));
		expect(publics.has('/')).toBe(true);
		expect(publics.has('/signin')).toBe(true);
		expect(publics.has('/v1')).toBe(true);
	});
});

// The GitHub token minted at sign-in is the ONLY way the dōjō learns whether a
// repository is private — `refreshForgeVisibility` asks the forge and records
// the answer. GitHub answers 404, not 403, for a repository the token cannot
// see, so a token without `repo` cannot distinguish "private" from "gone" and
// every private repository stays permanently uncaptured — which fails closed,
// and therefore never syncs. Private repos are precisely what the authority and
// mandate model exists for, so without this scope capture works only for the
// case that does not need it.
describe('kavach config — the forge token must be able to SEE private repos', () => {
	const github = () => config.providers.find((p) => p.name === 'github');

	it('requests `repo`, without which private visibility can never be captured', () => {
		expect(github()?.scopes).toContain('repo');
	});

	it('still requests read:org and user:email', () => {
		// `read:org` covers PRIVATE org membership (the F3c auto-join); `user:email`
		// is what Supabase resolves the profile from. Adding a scope must not
		// silently drop either.
		expect(github()?.scopes).toEqual(expect.arrayContaining(['read:org', 'user:email']));
	});
});
