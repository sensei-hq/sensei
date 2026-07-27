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
