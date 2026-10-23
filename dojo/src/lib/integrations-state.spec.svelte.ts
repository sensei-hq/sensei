import { describe, it, expect, vi } from 'vitest';
import { IntegrationsState, type IntegrationsApi } from './integrations-state.svelte';
import { DojoApiError } from './dojo-api';
import type { IntegrationUnit } from './integrations-view';

// Admin → Integrations (#308 R5a): the screen reads the orgs the person's
// CONNECTED GitHub sees. With no connection (or one GitHub stopped honouring) the
// server answers 428 and the screen offers "Connect GitHub" — a re-authorization
// through the dōjō's sign-in, never a pasted token. Every act re-reads the list;
// a failure says why and never leaves a stale or invented list on screen.
const ACME: IntegrationUnit = {
	provider: 'github',
	id: '22',
	login: 'acme',
	role: 'admin',
	dojo: null,
	mapping: null,
	canClaim: true,
	canMapAsDojo: true,
	canMapAsWorkspace: true
};

const NOT_CONNECTED = new DojoApiError(428, 'Connect GitHub to read your organizations');

function api(over: Partial<IntegrationsApi> = {}) {
	return {
		list: vi.fn().mockResolvedValue({ provider: 'github', units: [ACME], complete: true }),
		claim: vi.fn().mockResolvedValue({}),
		map: vi.fn().mockResolvedValue({}),
		connect: vi.fn().mockResolvedValue(undefined),
		...over
	};
}

describe('IntegrationsState', () => {
	it('reads the orgs the connected GitHub sees', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		await s.load();
		expect(a.list).toHaveBeenCalledWith();
		expect(s.units).toEqual([ACME]);
		expect(s.loaded).toBe(true);
		expect(s.needsConnect).toBe(false);
		expect(s.error).toBeNull();
	});

	it('with no GitHub connection, offers Connect GitHub — not an error, and no list', async () => {
		const s = new IntegrationsState(api({ list: vi.fn().mockRejectedValue(NOT_CONNECTED) }));
		await s.load();
		expect(s.needsConnect).toBe(true);
		expect(s.error).toBeNull();
		expect(s.units).toEqual([]);
		expect(s.loaded).toBe(false);
	});

	it('Connect GitHub runs the re-authorization', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		await s.connect();
		expect(a.connect).toHaveBeenCalledTimes(1);
	});

	it('a failed read says why and shows no list — never the previous one', async () => {
		const list = vi
			.fn()
			.mockResolvedValueOnce({ provider: 'github', units: [ACME], complete: true })
			.mockRejectedValueOnce(new DojoApiError(502, 'GitHub read failed (503)'));
		const s = new IntegrationsState(api({ list }));
		await s.load();
		expect(s.units).toEqual([ACME]);
		await s.load();
		expect(s.error).toBe('GitHub read failed (503)');
		expect(s.units).toEqual([]);
		expect(s.loaded).toBe(false);
	});

	it('claims, then re-reads', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		await s.claim(ACME);
		expect(a.claim).toHaveBeenCalledWith('22');
		expect(a.list).toHaveBeenCalledTimes(1);
	});

	it('maps as its own dōjō, or as a workspace here with the typed name', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		await s.mapAsDojo(ACME);
		expect(a.map).toHaveBeenLastCalledWith('22', { as: 'dojo' });
		await s.mapAsWorkspace(ACME);
		expect(a.map).toHaveBeenLastCalledWith('22', { as: 'workspace' });
		s.names['22'] = '  Acme Web ';
		await s.mapAsWorkspace(ACME);
		expect(a.map).toHaveBeenLastCalledWith('22', { as: 'workspace', name: 'Acme Web' });
	});

	it('an act whose connection lapsed offers Connect GitHub again', async () => {
		const s = new IntegrationsState(api({ claim: vi.fn().mockRejectedValue(NOT_CONNECTED) }));
		await s.load();
		await s.claim(ACME);
		expect(s.needsConnect).toBe(true);
		expect(s.units).toEqual([]);
	});

	it('forgets the list when reset (another dōjō`s screen)', async () => {
		const s = new IntegrationsState(api());
		await s.load();
		s.names['22'] = 'x';
		s.error = 'old';
		s.reset();
		expect([s.units, s.loaded, s.error, s.names, s.needsConnect]).toEqual([[], false, null, {}, false]);
	});

	it('a refused act says the database`s reason and keeps the list', async () => {
		const a = api({ claim: vi.fn().mockRejectedValue(new DojoApiError(403, 'only an admin of acme can claim its dōjō')) });
		const s = new IntegrationsState(a);
		await s.load();
		await s.claim(ACME);
		expect(s.error).toBe('only an admin of acme can claim its dōjō');
		expect(s.units).toEqual([ACME]);
		expect(s.busy).toBeNull();
	});
});
