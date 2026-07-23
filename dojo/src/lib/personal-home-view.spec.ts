import { describe, expect, it } from 'vitest';
import {
	personalGreeting,
	soloIdentity,
	LIBRARY_HREF,
	type PersonalUser
} from '$lib/personal-home-view';

describe('personal-home-view — soloIdentity', () => {
	it('prefers the display name, falling back to the email local-part, then "you"', () => {
		expect(soloIdentity({ name: 'Rin Saito', email: 'rin@x.dev' }).label).toBe('Rin Saito');
		expect(soloIdentity({ email: 'rin.saito@x.dev' }).label).toBe('rin.saito');
		expect(soloIdentity({}).label).toBe('you');
		expect(soloIdentity(undefined).label).toBe('you');
	});

	it('derives initials from the name (or the email local-part)', () => {
		expect(soloIdentity({ name: 'Rin Saito' }).initials).toBe('RS');
		expect(soloIdentity({ email: 'jerry.thomas@x.dev' }).initials).toBe('JT');
		expect(soloIdentity({}).initials).toBe('?');
	});
});

describe('personal-home-view — greeting', () => {
	it('greets the solo user by their derived label', () => {
		const user: PersonalUser = { name: 'Rin Saito', email: 'rin@x.dev' };
		expect(personalGreeting(user)).toContain('Rin Saito');
		expect(personalGreeting(user)).toContain('working solo');
	});

	it('never crashes on a missing user', () => {
		expect(typeof personalGreeting(undefined)).toBe('string');
	});
});

describe('personal-home-view — library link', () => {
	it('points at the console library route (arrives in the next chunk)', () => {
		expect(LIBRARY_HREF).toBe('/console/library');
	});
});
