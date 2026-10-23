// A value sealed for the browser to carry and the server alone to read (#308):
// AES-GCM under a key HKDF-derived from the server secret AND a purpose, bound to
// one login and an expiry. What R5a's GitHub token cookie used, generalised so
// the Azure DevOps connect flow (its OAuth state, its pending grant) seals with
// the same primitive under keys of its own.
import { describe, it, expect } from 'vitest';
import { openSealed, seal } from './sealed-value';

const SECRET = 'sealed-value-test-secret';
const NOW = 1_800_000_000_000;

describe('seal / openSealed', () => {
	it('round-trips a value for the login and purpose it was sealed for', async () => {
		const v = await seal({ state: 's1', verifier: 'v1' }, 'auth-1', SECRET, 'purpose-a', 600, NOW);
		expect(await openSealed(v, 'auth-1', SECRET, 'purpose-a', NOW + 1000)).toEqual({ state: 's1', verifier: 'v1' });
	});

	it('opens to nothing for another login, another purpose, another secret, after expiry, or tampered', async () => {
		const v = await seal('payload', 'auth-1', SECRET, 'purpose-a', 600, NOW);
		expect(await openSealed(v, 'auth-2', SECRET, 'purpose-a', NOW)).toBeNull();
		expect(await openSealed(v, 'auth-1', SECRET, 'purpose-b', NOW)).toBeNull();
		expect(await openSealed(v, 'auth-1', 'another-secret', 'purpose-a', NOW)).toBeNull();
		expect(await openSealed(v, 'auth-1', SECRET, 'purpose-a', NOW + 601_000)).toBeNull();
		// Always a DIFFERENT character in a byte-carrying position (the last one may
		// carry padding bits only).
		const at = v.length - 2;
		const flipped = v.slice(0, at) + (v[at] === 'A' ? 'B' : 'A') + v.slice(at + 1);
		expect(flipped).not.toBe(v);
		expect(await openSealed(flipped, 'auth-1', SECRET, 'purpose-a', NOW)).toBeNull();
		expect(await openSealed(undefined, 'auth-1', SECRET, 'purpose-a', NOW)).toBeNull();
		expect(await openSealed('not base64!', 'auth-1', SECRET, 'purpose-a', NOW)).toBeNull();
	});

	it('refuses to seal without a secret', async () => {
		await expect(seal('x', 'auth-1', '', 'purpose-a', 600, NOW)).rejects.toThrow();
	});
});
