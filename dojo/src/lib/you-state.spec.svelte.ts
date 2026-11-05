import { describe, expect, it, vi } from 'vitest';
import { YouSectionState, forgeSyncMessage, visibilityLabel, authorityLabel, type YouApi } from '$lib/you-state.svelte';
import type { GraphBranchOutcomeWire, MyRepoWire } from '$lib/client-data';
import * as you from '$lib/you-state.svelte';

// The personal-zone section's actions (#293): every write says why it failed,
// and nothing shows a decision the server did not record.

function api(over: Partial<YouApi> = {}): YouApi {
	return {
		adoptContribution: vi.fn().mockResolvedValue(undefined),
		setPackAdoption: vi.fn().mockResolvedValue(undefined),
		provisionFromForge: vi.fn().mockResolvedValue({ synced: true, personal: null, tenants: [] }),
		setRepoElection: vi.fn().mockResolvedValue({ repo_key: 'r', elected: true, sync_enabled: true, reason_code: null }),
		setGraphBranch: vi.fn().mockResolvedValue({ repo_key: 'r', branch: 'b', tracked: true, is_default: false } satisfies GraphBranchOutcomeWire),
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

	it('tracks a branch and reloads; a refusal says why', async () => {
		const s = new YouSectionState();
		const r = reload();
		const setGraphBranch = vi.fn().mockResolvedValue({ repo_key: 'r', branch: 'develop', tracked: true, is_default: false });
		await s.track(api({ setGraphBranch }), 'r', 'develop', 'development', r);
		expect(setGraphBranch).toHaveBeenCalledWith('r', 'develop', true, 'development');
		expect(r).toHaveBeenCalledOnce();
		expect(s.sharingError(null)).toBeNull();
		expect(s.sharingPending.has('r')).toBe(false);
		await s.track(api({ setGraphBranch: boom('forbidden') }), 'r', 'develop', null, r);
		expect(s.sharingError(null)).toBe('forbidden');
		expect(r).toHaveBeenCalledOnce();
	});

	it('untracks a branch and reloads; a refusal says why', async () => {
		const s = new YouSectionState();
		const r = reload();
		const setGraphBranch = vi.fn().mockResolvedValue({ repo_key: 'r', branch: 'develop', tracked: false, is_default: false });
		await s.untrack(api({ setGraphBranch }), 'r', 'develop', r);
		expect(setGraphBranch).toHaveBeenCalledWith('r', 'develop', false, null);
		expect(r).toHaveBeenCalledOnce();
		await s.untrack(api({ setGraphBranch: boom('forbidden') }), 'r', 'develop', r);
		expect(s.sharingError(null)).toBe('forbidden');
		expect(r).toHaveBeenCalledOnce();
	});
});

// Audit item 15 (2026-10-28): the Sharing screen built these labels itself.
describe('sharing labels', () => {
	it('says what the forge said, or that it was not asked — never a guess', () => {
		expect(visibilityLabel('private')).toBe('private');
		expect(visibilityLabel(null)).toBe('not yet captured');
	});

	it('says who holds the choice; no authority is a real state, not "you"', () => {
		expect(authorityLabel({ authority: 'organization', owning_org: 'acme' } as never)).toBe('acme decides');
		expect(authorityLabel({ authority: 'organization', owning_org: null } as never)).toBe('the organisation decides');
		expect(authorityLabel({ authority: 'user' } as never)).toBe('you decide');
		expect(authorityLabel({ authority: null } as never)).toBe('nobody yet');
	});
});


// billing build · WP8 (reviews/Design/L4-billing-build-tests-r2.md H1, L0 rulings 2026-11-05):
// Sharing's hidden line groups the rows the SERVER flagged (`billing_hidden`) per organization.
// The helper reads the flag and takes no predicate: the rule lives once, in
// `isBillingHiddenRepo` ($lib/server/repositories.ts), and nothing here re-derives it.
// ── test-local shape the build adds; phase B imports it directly ──
type HiddenOrg = { org: string; count: number; href: string };
type Pending = { splitBillingHidden(rows: MyRepoWire[]): { shown: MyRepoWire[]; hidden: HiddenOrg[] } };
const { splitBillingHidden } = you as typeof you & Pending;

const wire = (id: string, org: string | null, billing_hidden: boolean, reason_code: string | null = null): MyRepoWire => ({
	repository_id: id,
	repo_key: `github.com/${org ?? 'jdoe'}/${id}`,
	name: id,
	tenant: `github/${org ?? 'jdoe'}`,
	owning_org: org,
	forge_visibility: 'private',
	authority: org ? 'organization' : 'user',
	may_share: !billing_hidden,
	elected: true,
	sync_enabled: false,
	configurable_by_me: false,
	reason_code,
	reason: null,
	remedy: null,
	reason_actor: null,
	last_synced_at: null,
	metric_rows: 0,
	graph_branches: [],
	billing_hidden
});

describe('splitBillingHidden · groups by the flag (WP8)', () => {
	it.skip('pending: billing-build — shown rows keep their order; flagged rows are counted per org, first seen first, linked to its Plan & billing', () => {
		const { shown, hidden } = splitBillingHidden([
			wire('a', 'acme', true),
			wire('p', 'acme', false),
			wire('g', 'globex', true),
			wire('b', 'acme', true),
			wire('n', null, false)
		]);
		expect(shown.map((r) => r.repository_id)).toEqual(['p', 'n']);
		expect(hidden).toEqual([
			{ org: 'acme', count: 2, href: '/org/acme/billing' },
			{ org: 'globex', count: 1, href: '/org/globex/billing' }
		]);
	});

	it.skip('pending: billing-build — the flag decides, never reason_code: false + not_subscribed is shown, true + null is hidden', () => {
		const { shown, hidden } = splitBillingHidden([wire('kept', 'acme', false, 'not_subscribed'), wire('gone', 'globex', true, null)]);
		expect(shown.map((r) => r.repository_id)).toEqual(['kept']);
		expect(hidden).toEqual([{ org: 'globex', count: 1, href: '/org/globex/billing' }]);
	});

	it.skip('pending: billing-build — nothing flagged: every row shown, no line', () => {
		const rows = [wire('a', 'acme', false), wire('n', null, false)];
		expect(splitBillingHidden(rows)).toEqual({ shown: rows, hidden: [] });
	});
});
