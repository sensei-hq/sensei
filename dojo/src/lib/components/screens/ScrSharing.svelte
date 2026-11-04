<script lang="ts">
	import { dojoLoadNotice } from '$lib/dojo-notices';
	import type { ReadError } from '$lib/read-error';
	import { Screen, Notice } from '@sensei/ui';
	import { SectionHead } from '$lib/components/kit';
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
	import { authorityLabel, visibilityLabel } from '$lib/you-state.svelte';
	import { GRAPH_LABELS } from '$lib/graph-labels';

	interface Props {
		repos: MyRepoWire[];
		/** Non-null when the read FAILED. Rendered instead of an empty list: "you
		 *  have no repositories" and "we could not ask" are different answers. */
		error?: ReadError | string | null;
		/** Repo keys with a write in flight, so the row can disable itself. */
		pending?: ReadonlySet<string>;
		onToggle?: (repoKey: string, elected: boolean) => void;
		onTrack?: (repoKey: string, branch: string, label: string | null) => void;
		onUntrack?: (repoKey: string, branch: string) => void;
	}
	let { repos, error = null, pending = new Set<string>(), onToggle, onTrack, onUntrack }: Props = $props();

	// Draft branch names and labels, per repository. The input starts EMPTY —
	// pre-filling `main` would be the guess the plan refuses to make.
	let drafts = $state<Record<string, string>>({});
	let labels = $state<Record<string, string>>({});
	// The `repoKey branch` awaiting its in-page confirmation, if any.
	// Untracking deletes that branch's graph, so one click is never enough.
	let confirming = $state<string | null>(null);

	function trackRepo(r: MyRepoWire): void {
		const branch = (drafts[r.repo_key] ?? '').trim();
		if (!branch) return;
		onTrack?.(r.repo_key, branch, labels[r.repo_key] || null);
		drafts[r.repo_key] = '';
	}

</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				eyebrow="You · sharing"
				title="Sharing"
				description="Which repositories send metrics to a dōjō. A repository syncs only when it is permitted and whoever holds the choice has made it."
			/>
		</div>
	{/snippet}
<section class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">

	{#if error}
		<!-- An error state, never an empty list. -->
		<Notice size="panel" {...dojoLoadNotice("your repositories", error)} data-load-error />
	{:else if repos.length === 0}
		<p class="text-sm text-ink-mute">
			No repositories yet. They appear once sensei has scanned a clone and the daemon
			has registered it.
		</p>
	{:else}
		<ul class="flex flex-col gap-2">
			{#each repos as r (r.repository_id)}
				<li class="flex flex-col gap-2 rounded border border-line p-3 md:flex-row md:items-center md:justify-between">
					<div class="flex flex-col gap-1">
						<span class="text-sm text-ink">{r.name ?? r.repo_key}</span>
						<span class="text-xs text-ink-mute">
							{r.tenant} · {visibilityLabel(r.forge_visibility)} · {authorityLabel(r)}
						</span>
						{#if !r.sync_enabled && r.reason}
							<!-- The refusal names itself AND what to do. A bare "off" here is
							     the exact ambiguity the reason registry exists to remove. -->
							<span class="text-xs text-ink-mute">
								{r.reason}{#if r.remedy} — {r.remedy}{/if}{#if r.reason_actor && !r.configurable_by_me}
									<span class="text-ink-faint"> ({r.reason_actor} can change it)</span>
								{/if}
							</span>
						{/if}
						<!-- The branches whose code graph this repository shares — what the
						     daemon will push. An empty set says so honestly: the graph does
						     not sync, and nothing here guesses `main`. -->
						{#if r.graph_branches.length === 0}
							<span class="text-xs text-ink-mute">graph: no tracked branch — its graph does not sync</span>
						{:else}
							<ul class="flex flex-wrap gap-x-3 gap-y-1">
								{#each r.graph_branches as b (b.branch)}
									<li class="flex items-center gap-1 text-xs text-ink-mute">
										<span>{b.is_default ? `${b.branch} (default)` : b.branch}</span>
										{#if confirming === `${r.repo_key} ${b.branch}`}
											<span class="text-ink-faint">deletes its graph — sure?</span>
											<button
												type="button"
												class="underline"
												aria-label={`Confirm untracking ${b.branch}`}
												disabled={pending.has(r.repo_key)}
												onclick={() => {
													onUntrack?.(r.repo_key, b.branch);
													confirming = null;
												}}
											>
												Yes, untrack
											</button>
											<button type="button" class="underline" onclick={() => (confirming = null)}>
												Cancel
											</button>
										{:else}
											<button
												type="button"
												class="underline"
												aria-label={`Untrack ${b.branch}`}
												disabled={!r.configurable_by_me || pending.has(r.repo_key)}
												onclick={() => (confirming = `${r.repo_key} ${b.branch}`)}
											>
												Untrack
											</button>
										{/if}
									</li>
								{/each}
							</ul>
						{/if}
						<div class="flex flex-wrap items-center gap-2">
							<input
								type="text"
								class="rounded border border-line bg-transparent px-2 py-1 text-xs text-ink"
								placeholder="branch name"
								aria-label={`Branch to track for ${r.name ?? r.repo_key}`}
								value={drafts[r.repo_key] ?? ''}
								disabled={!r.configurable_by_me || pending.has(r.repo_key)}
								oninput={(e) => (drafts[r.repo_key] = e.currentTarget.value)}
							/>
							<select
								class="rounded border border-line bg-transparent px-2 py-1 text-xs text-ink"
								aria-label={`Label for ${r.name ?? r.repo_key}`}
								value={labels[r.repo_key] ?? ''}
								disabled={!r.configurable_by_me || pending.has(r.repo_key)}
								onchange={(e) => (labels[r.repo_key] = e.currentTarget.value)}
							>
								<option value="">No label</option>
								{#each GRAPH_LABELS as l}
									<option value={l}>{l}</option>
								{/each}
							</select>
							<button
								type="button"
								class="rounded border border-line px-2 py-1 text-xs text-ink"
								aria-label={`Track a branch of ${r.name ?? r.repo_key}`}
								disabled={!r.configurable_by_me || pending.has(r.repo_key)}
								onclick={() => trackRepo(r)}
							>
								Track
							</button>
						</div>
					</div>

					<div class="flex items-center gap-3">
						<span class="text-xs" class:text-success={r.sync_enabled} class:text-ink-mute={!r.sync_enabled}>
							{r.sync_enabled ? 'sharing' : 'not sharing'}
						</span>
						<label class="flex items-center gap-2 text-xs text-ink-mute">
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
</Screen>
