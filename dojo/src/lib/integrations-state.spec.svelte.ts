import { describe, it, expect, vi } from 'vitest';
import { IntegrationsState, type IntegrationsApi } from './integrations-state.svelte';
import { DojoApiError } from './dojo-api';
import type { IntegrationUnit } from './integrations-view';

// Admin → Integrations (#308 R5a): the person pastes a GitHub token, reads the
// orgs it can see, and claims or maps them. The token stays in memory for this
// screen; every act re-reads the list; a failure says why and never leaves a
// stale or invented list on screen.
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

function api(over: Partial<IntegrationsApi> = {}) {
	return {
		list: vi.fn().mockResolvedValue({ provider: 'github', units: [ACME], complete: true }),
		claim: vi.fn().mockResolvedValue({}),
		map: vi.fn().mockResolvedValue({}),
		...over
	};
}

describe('IntegrationsState', () => {
	it('reads the orgs the trimmed token sees', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		s.token = '  gh-token  ';
		await s.load();
		expect(a.list).toHaveBeenCalledWith('gh-token');
		expect(s.units).toEqual([ACME]);
		expect(s.loaded).toBe(true);
		expect(s.complete).toBe(true);
		expect(s.error).toBeNull();
	});

	it('asks for a token rather than reading with none', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		await s.load();
		expect(a.list).not.toHaveBeenCalled();
		expect(s.error).toMatch(/token/i);
	});

	it('a failed read says why and shows no list — never the previous one', async () => {
		const list = vi
			.fn()
			.mockResolvedValueOnce({ provider: 'github', units: [ACME], complete: true })
			.mockRejectedValueOnce(new DojoApiError(502, 'GitHub read failed (401)'));
		const s = new IntegrationsState(api({ list }));
		s.token = 'gh';
		await s.load();
		expect(s.units).toEqual([ACME]);
		await s.load();
		expect(s.error).toBe('GitHub read failed (401)');
		expect(s.units).toEqual([]);
		expect(s.loaded).toBe(false);
	});

	it('claims, then re-reads', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		s.token = 'gh';
		await s.claim(ACME);
		expect(a.claim).toHaveBeenCalledWith('gh', '22');
		expect(a.list).toHaveBeenCalledTimes(1);
	});

	it('maps as its own dōjō, or as a workspace here with the typed name', async () => {
		const a = api();
		const s = new IntegrationsState(a);
		s.token = 'gh';
		await s.mapAsDojo(ACME);
		expect(a.map).toHaveBeenLastCalledWith('gh', '22', { as: 'dojo' });
		await s.mapAsWorkspace(ACME);
		expect(a.map).toHaveBeenLastCalledWith('gh', '22', { as: 'workspace' });
		s.names['22'] = '  Acme Web ';
		await s.mapAsWorkspace(ACME);
		expect(a.map).toHaveBeenLastCalledWith('gh', '22', { as: 'workspace', name: 'Acme Web' });
	});

	it('forgets the token and the list when reset (another dōjō`s screen)', async () => {
		const s = new IntegrationsState(api());
		s.token = 'gh';
		await s.load();
		s.names['22'] = 'x';
		s.error = 'old';
		s.reset();
		expect([s.token, s.units, s.loaded, s.error, s.names]).toEqual(['', [], false, null, {}]);
	});

	it('a refused act says the database`s reason and keeps the list', async () => {
		const a = api({ claim: vi.fn().mockRejectedValue(new DojoApiError(403, 'only an admin of acme can claim its dōjō')) });
		const s = new IntegrationsState(a);
		s.token = 'gh';
		await s.load();
		await s.claim(ACME);
		expect(s.error).toBe('only an admin of acme can claim its dōjō');
		expect(s.units).toEqual([ACME]);
		expect(s.busy).toBeNull();
	});
});
