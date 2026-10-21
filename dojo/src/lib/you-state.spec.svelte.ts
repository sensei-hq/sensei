import { describe, expect, it, vi } from 'vitest';
import { YouSectionState, forgeSyncMessage, type YouApi } from '$lib/you-state.svelte';
import type { MyRepoWire } from '$lib/client-data';

// The personal-zone section's actions (#293): every write says why it failed,
// and nothing shows a decision the server did not record.

function api(over: Partial<YouApi> = {}): YouApi {
	return {
		adoptContribution: vi.fn().mockResolvedValue(undefined),
		setPackAdoption: vi.fn().mockResolvedValue(undefined),
		createDojo: vi.fn().mockResolvedValue({ id: 't1', key: 'k', name: 'Guild' }),
		provisionFromForge: vi.fn().mockResolvedValue({ synced: true, personal: null, tenants: [] }),
		setRepoElection: vi.fn().mockResolvedValue({ repo_key: 'r', elected: true, sync_enabled: true, reason_code: null }),
		...over
	} as YouApi;
}
const boom = (msg: string) => vi.fn().mockRejectedValue(new Error(msg));
const reload = () => vi.fn().mockResolvedValue(undefined);

describe('pinning and packs', () => {
	it('pins a contribution and reloads; a refusal says why', async () => {
		const s = new YouSectionState();
		const r = reload();
		await s.pin(api(), { id: 'a1' } as never, r);
		expect(r).toHaveBeenCalledOnce();
		expect(s.actionError).toBeNull();
		await s.pin(api({ adoptContribution: boom('forbidden') }), { id: 'a1' } as never, r);
		expect(s.actionError).toContain('forbidden');
		expect(r).toHaveBeenCalledOnce();
	});

	it('reports whether a pack adoption landed, so the screen can put it back', async () => {
		const s = new YouSectionState();
		expect(await s.togglePack(api(), { id: 'owasp' } as never, true, reload())).toBe(true);
		expect(await s.togglePack(api({ setPackAdoption: boom('nope') }), { id: 'owasp' } as never, true, reload())).toBe(false);
		expect(s.actionError).toContain('nope');
	});
});

describe('creating a dōjō', () => {
	it('opens a clean form, creates, closes and reloads', async () => {
		const s = new YouSectionState();
		s.openCreate();
		s.create.name = '  Guild ';
		const a = api();
		const r = reload();
		await s.submitCreate(a, r);
		expect(a.createDojo).toHaveBeenCalledWith('Guild', 'employer');
		expect(s.create.open).toBe(false);
		expect(r).toHaveBeenCalledOnce();
	});

	it('keeps the form open with the reason when the create fails', async () => {
		const s = new YouSectionState();
		s.openCreate();
		s.create.name = 'Guild';
		await s.submitCreate(api({ createDojo: boom('name taken') }), reload());
		expect(s.create.open).toBe(true);
		expect(s.create.error).toBe('name taken');
		expect(s.create.busy).toBe(false);
	});
});

describe('syncing from GitHub', () => {
	it('names each refusal apart', () => {
		expect(forgeSyncMessage({ synced: false, reason: 'forge_unreachable', personal: null, tenants: [] })).toContain('could not be reached');
		expect(forgeSyncMessage({ synced: false, reason: 'forge_token_rejected', personal: null, tenants: [] })).toContain('sign in with GitHub again');
		expect(forgeSyncMessage({ synced: false, reason: 'no_forge_token', personal: null, tenants: [] })).toContain('Sign in with GitHub');
		const t = (created: boolean) => ({ created }) as never;
		expect(forgeSyncMessage({ synced: true, personal: null, tenants: [t(true), t(true), t(false)] })).toBe('Set up 2 dōjōs from GitHub.');
		expect(forgeSyncMessage({ synced: true, personal: null, tenants: [t(false)] })).toBe('Everything from GitHub is already set up.');
	});

	it('reloads only when something was made, and says when the call failed', async () => {
		const s = new YouSectionState();
		const r = reload();
		await s.syncGithub(api(), r);
		expect(r).not.toHaveBeenCalled();
		await s.syncGithub(api({ provisionFromForge: vi.fn().mockResolvedValue({ synced: true, personal: { created: true }, tenants: [] }) }), r);
		expect(r).toHaveBeenCalledOnce();
		await s.syncGithub(api({ provisionFromForge: boom('x') }), r);
		expect(s.github.message).toBe('Could not sync from GitHub — try again.');
		expect(s.github.busy).toBe(false);
	});
});

describe('sharing', () => {
	const repo = (key: string, elected: boolean) => ({ repo_key: key, elected, sync_enabled: elected, reason_code: null }) as unknown as MyRepoWire;

	it('shows the server’s verdict, not the click', async () => {
		const s = new YouSectionState();
		let release!: () => void;
		const r = vi.fn(() => new Promise<void>((res) => (release = res)));
		const out = { repo_key: 'r', elected: true, sync_enabled: false, reason_code: 'not_subscribed' };
		const done = s.elect(api({ setRepoElection: vi.fn().mockResolvedValue(out) }), 'r', true, r);
		await vi.waitFor(() => expect(r).toHaveBeenCalled());
		// Between the write and the re-read, the row shows what the server said.
		expect(s.sharingRepos([repo('r', false)])[0]).toMatchObject({ elected: true, sync_enabled: false, reason_code: 'not_subscribed' });
		release();
		await done;
		expect(s.sharingRepos([repo('r', false)])[0].elected).toBe(false);
	});

	it('a failed toggle puts the row back and says why', async () => {
		const s = new YouSectionState();
		await s.elect(api({ setRepoElection: boom('denied') }), 'r', true, reload());
		expect(s.sharingRepos([repo('r', false)])[0].elected).toBe(false);
		expect(s.sharingError(null)).toBe('denied');
		expect(s.sharingPending.has('r')).toBe(false);
	});
});
