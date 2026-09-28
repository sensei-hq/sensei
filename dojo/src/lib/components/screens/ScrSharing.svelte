<script lang="ts">
	// Sharing — the DECISION surface for `dojo.all_my_repositories`.
	//
	// Every other consumer of that view reads it: the daemon through
	// `/v1/you/sync/plan`, the ingest through `sync_enabled`. This is the one
	// place a human acts on it, and until it existed the election had a write
	// path and no way to reach it — three repositories sat at
	// `not_elected_user`, refusing for want of a choice nobody could make.
	//
	// PRESENTATIONAL. It renders the verdict it is handed and reports a toggle
	// upward; it never re-derives `sync_enabled`, because the view is the single
	// place `may_share AND elected` is decided.
	import type { MyRepoWire } from '$lib/client-data';

	interface Props {
		repos: MyRepoWire[];
		/** Non-null when the read FAILED. Rendered instead of an empty list: "you
		 *  have no repositories" and "we could not ask" are different answers. */
		error?: string | null;
		/** Repo keys with a toggle in flight, so the row can disable itself. */
		pending?: ReadonlySet<string>;
		onToggle?: (repoKey: string, elected: boolean) => void;
	}
	let { repos, error = null, pending = new Set<string>(), onToggle }: Props = $props();

	/** What the forge said, or that it has not been asked. Never guessed. */
	function visibilityLabel(v: MyRepoWire['forge_visibility']): string {
		return v === null ? 'not yet captured' : v;
	}

	/** Who holds the choice. `null` means no authority is derivable yet, which is
	 *  a real state — not "user" by default. */
	function authorityLabel(r: MyRepoWire): string {
		if (r.authority === 'organization') return `${r.owning_org ?? 'the organisation'} decides`;
		if (r.authority === 'user') return 'you decide';
		return 'nobody yet';
	}
</script>

<section class="flex flex-col gap-4">
	<header class="flex flex-col gap-1">
		<h1 class="text-lg font-heading text-ink">Sharing</h1>
		<p class="text-sm text-ink-soft">
			Which repositories send metrics to a dōjō. A repository syncs only when it
			is permitted <em>and</em> whoever holds the choice has made it.
		</p>
	</header>

	{#if error}
		<!-- An error state, never an empty list. -->
		<p role="alert" class="text-sm text-danger">Could not load your repositories — {error}</p>
	{:else if repos.length === 0}
		<p class="text-sm text-ink-soft">
			No repositories yet. They appear once sensei has scanned a clone and the daemon
			has registered it.
		</p>
	{:else}
		<ul class="flex flex-col gap-2">
			{#each repos as r (r.repository_id)}
				<li class="flex flex-col gap-2 rounded border border-line p-3 md:flex-row md:items-center md:justify-between">
					<div class="flex flex-col gap-1">
						<span class="text-sm text-ink">{r.name ?? r.repo_key}</span>
						<span class="text-xs text-ink-soft">
							{r.tenant} · {visibilityLabel(r.forge_visibility)} · {authorityLabel(r)}
						</span>
						{#if !r.sync_enabled && r.reason}
							<!-- The refusal names itself AND what to do. A bare "off" here is
							     the exact ambiguity the reason registry exists to remove. -->
							<span class="text-xs text-ink-soft">
								{r.reason}{#if r.remedy} — {r.remedy}{/if}{#if r.reason_actor && !r.configurable_by_me}
									<span class="text-ink-faint"> ({r.reason_actor} can change it)</span>
								{/if}
							</span>
						{/if}
					</div>

					<div class="flex items-center gap-3">
						<span class="text-xs" class:text-success={r.sync_enabled} class:text-ink-soft={!r.sync_enabled}>
							{r.sync_enabled ? 'sharing' : 'not sharing'}
						</span>
						<label class="flex items-center gap-2 text-xs text-ink-soft">
							<input
								type="checkbox"
								checked={r.elected}
								disabled={!r.configurable_by_me || pending.has(r.repo_key)}
								aria-label={`Share ${r.name ?? r.repo_key}`}
								onchange={(e) => onToggle?.(r.repo_key, e.currentTarget.checked)}
							/>
							<!-- `elected` is what the toggle SETS; `sync_enabled` is what the
							     system decided. They differ whenever entitlement refuses, and
							     showing both is what stops "I turned it on and nothing
							     happened" from being a mystery. -->
							elect
						</label>
					</div>
				</li>
			{/each}
		</ul>
	{/if}
</section>
