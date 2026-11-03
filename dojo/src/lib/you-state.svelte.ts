// The personal zone's section actions (#293) — pin a contribution, adopt a rule
// pack, set up dōjōs from GitHub, elect a repository for
// sharing. The page renders; this runs the writes with the API and the reload
// handed in, and says why each one failed. Nothing shows a decision the server
// did not record.
import { SvelteSet } from 'svelte/reactivity';
import type { MyRepoWire, ProvisionResult } from '$lib/client-data';
import type { KitDownstream, KitRulePack } from '$lib/components/kit/types';

/** The writes the section makes — `$lib/client-data`, injected so a spec can fail them. */
export interface YouApi {
	adoptContribution: (artifactId: string) => Promise<void>;
	setPackAdoption: (slug: string, adopt: boolean) => Promise<void>;
	provisionFromForge: () => Promise<ProvisionResult>;
	setRepoElection: (
		repoKey: string,
		elected: boolean
	) => Promise<{ repo_key: string; elected: boolean; sync_enabled: boolean; reason_code: string | null }>;
}

/** Re-read the section's load (the page passes invalidateAll). */
export type Reload = () => Promise<unknown>;

const reason = (e: unknown, fallback: string) => (e instanceof Error && e.message ? e.message : fallback);

/** What a GitHub sync came to. Each refusal wants different advice, so they
 *  are named apart: 'forge_token_rejected' must not read as "try again" — the
 *  token is dead, and retrying is what the daemon was already doing. */
export function forgeSyncMessage(r: ProvisionResult): string {
	if (!r.synced) {
		if (r.reason === 'forge_unreachable') return 'GitHub could not be reached — try again in a moment.';
		if (r.reason === 'forge_token_rejected') return 'GitHub rejected the stored access — sign in with GitHub again.';
		return 'Sign in with GitHub to set up your org dōjōs.';
	}
	const created = r.tenants.filter((t) => t.created).length;
	return created
		? `Set up ${created} dōjō${created > 1 ? 's' : ''} from GitHub.`
		: 'Everything from GitHub is already set up.';
}

export class YouSectionState {
	/** Why the last pin or pack change failed. */
	actionError = $state<string | null>(null);

	github = $state({ busy: false, message: null as string | null });

	/** Per-repo overrides with the server's verdict, between a write and its re-read. */
	#overrides = $state<Record<string, Partial<MyRepoWire>>>({});
	#toggleError = $state<string | null>(null);
	readonly sharingPending = new SvelteSet<string>();

	// ── Pin · packs ─────────────────────────────────────────────────────────

	async pin(api: YouApi, item: KitDownstream, reload: Reload): Promise<void> {
		if (!item.id) return;
		this.actionError = null;
		try {
			await api.adoptContribution(item.id);
			await reload();
		} catch (e) {
			this.actionError = `Not pinned: ${reason(e, 'the dōjō refused')}`;
		}
	}

	/** Adopt or drop a pack. Says whether it landed, so the screen can put the
	 *  row back when it did not. */
	async togglePack(api: YouApi, pack: KitRulePack, adopt: boolean, reload: Reload): Promise<boolean> {
		this.actionError = null;
		try {
			await api.setPackAdoption(pack.id, adopt);
			await reload();
			return true;
		} catch (e) {
			this.actionError = `${adopt ? 'Not adopted' : 'Not dropped'}: ${reason(e, 'the dōjō refused')}`;
			return false;
		}
	}

	// ── GitHub ──────────────────────────────────────────────────────────────

	/** Set up the personal dōjō and one per GitHub org the caller's token proves. */
	async syncGithub(api: YouApi, reload: Reload): Promise<void> {
		if (this.github.busy) return;
		this.github.busy = true;
		this.github.message = null;
		try {
			const r = await api.provisionFromForge();
			this.github.message = forgeSyncMessage(r);
			if (r.tenants.some((t) => t.created) || r.personal?.created) await reload();
		} catch {
			this.github.message = 'Could not sync from GitHub — try again.';
		} finally {
			this.github.busy = false;
		}
	}

	// ── Sharing ─────────────────────────────────────────────────────────────

	/** The loaded repositories with any verdict a write has returned on top. */
	sharingRepos(loaded: MyRepoWire[]): MyRepoWire[] {
		return loaded.map((r) => ({ ...r, ...(this.#overrides[r.repo_key] ?? {}) }));
	}

	/** A failed write is the more recent thing the user did, so it shows first. */
	sharingError(readError: string | null): string | null {
		return this.#toggleError ?? readError;
	}

	/** Elect a repository for sharing. The RESPONSE is the truth, not the click:
	 *  electing a repo whose entitlement refuses comes back `sync_enabled: false`.
	 *  The reload then brings the registry's reason; a failed write puts the row
	 *  back to what the server last confirmed. */
	async elect(api: YouApi, repoKey: string, elected: boolean, reload: Reload): Promise<void> {
		if (this.sharingPending.has(repoKey)) return;
		this.sharingPending.add(repoKey);
		this.#toggleError = null;
		try {
			const out = await api.setRepoElection(repoKey, elected);
			this.#overrides = {
				...this.#overrides,
				[repoKey]: { elected: out.elected, sync_enabled: out.sync_enabled, reason_code: out.reason_code }
			};
			await reload();
			this.#overrides = {};
		} catch (e) {
			const { [repoKey]: _dropped, ...rest } = this.#overrides;
			this.#overrides = rest;
			this.#toggleError = reason(e, 'could not change sharing');
		} finally {
			this.sharingPending.delete(repoKey);
		}
	}
}

/** What the forge said about a repository's visibility, or that it has not been
 *  asked — never guessed. */
export function visibilityLabel(v: MyRepoWire['forge_visibility']): string {
	return v === null ? 'not yet captured' : v;
}

/** Who holds a repository's sharing choice. No derivable authority is a real
 *  state — not "you" by default. */
export function authorityLabel(r: Pick<MyRepoWire, 'authority' | 'owning_org'>): string {
	if (r.authority === 'organization') return `${r.owning_org ?? 'the organisation'} decides`;
	if (r.authority === 'user') return 'you decide';
	return 'nobody yet';
}

