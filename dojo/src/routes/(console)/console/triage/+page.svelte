<script lang="ts">
	import { resolve } from '$app/paths';
	import { invalidateAll } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import Confidence from '$lib/components/Confidence.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { promoteSweep, TriageApiError, type PromoteResult } from '$lib/triage-data';
	import {
		groupByScope,
		kindKanji,
		kindLabel,
		relativeAge,
		similarityBand,
		similarityToneClass
	} from '$lib/triage-view';

	// Triage queue (mockup DojoTriage): the tenant's open rows grouped by scope,
	// ranked strongest-first. A maintainer can run the promotion sweep on demand
	// (POST …/triage/promote) to flush anything that now clears the auto bar.
	let { data } = $props();

	const groups = $derived(groupByScope(data.queue));

	let sweeping = $state(false);
	let sweepError = $state<string | null>(null);
	let sweepResult = $state<PromoteResult[] | null>(null);

	async function runSweep() {
		sweeping = true;
		sweepError = null;
		sweepResult = null;
		try {
			sweepResult = await promoteSweep(data.tenantKey, { fetch, accessToken: data.accessToken });
			await invalidateAll();
		} catch (e) {
			sweepError = e instanceof TriageApiError ? e.message : 'promotion sweep failed';
		} finally {
			sweeping = false;
		}
	}

	const sweepSummary = $derived.by(() => {
		if (!sweepResult) return null;
		const counts = { published: 0, merged: 0, queued: 0, no_op: 0 };
		for (const r of sweepResult) counts[r.result.outcome]++;
		return counts;
	});
</script>

<svelte:head>
	<title>Triage · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="門"
		eyebrow="Govern · triage"
		title="What's waiting for review"
		sub="Your scopes by default, ranked by projected impact then age. Nothing publishes without a decision."
	>
		{#snippet right()}
			<div class="flex items-center gap-2">
				<button
					type="button"
					onclick={runSweep}
					disabled={sweeping}
					class="bg-ink text-on-primary inline-flex items-center gap-2 rounded-lg text-xs font-medium"
					style="padding: 8px 13px; cursor: pointer; border: none"
					style:opacity={sweeping ? 0.6 : 1}
				>
					<span class="kanji" style="font-size: 12px">選</span>
					{sweeping ? 'Running sweep…' : 'Run promotion sweep'}
				</button>
			</div>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 8px 28px 28px">
		{#if data.error}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-warning">検</span>
				<span
					>Live triage is unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span></span
				>
			</div>
		{/if}

		{#if sweepError}
			<div
				class="bg-warning-soft border-warning-edge text-ink-soft flex items-center gap-2 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-warning">検</span>
				<span>Sweep failed. <span class="mono text-ink-mute text-xs">{sweepError}</span></span>
			</div>
		{/if}

		{#if sweepSummary}
			<div
				class="bg-success-soft border-success-edge text-ink-soft flex items-center gap-3 rounded-xl border text-sm"
				style="padding: 12px 16px; margin-top: 16px"
			>
				<span class="kanji text-success">決</span>
				<span
					>Sweep complete — <b class="font-semibold">{sweepSummary.published}</b> published,
					<b class="font-semibold">{sweepSummary.merged}</b> merged,
					<b class="font-semibold">{sweepSummary.queued}</b> still queued.</span
				>
			</div>
		{/if}

		{#if groups.length === 0 && !data.error}
			<div
				class="border-ink-faint text-ink-mute flex flex-col items-center gap-2 rounded-xl border border-dashed text-center"
				style="padding: 48px 28px; margin-top: 24px"
			>
				<span class="kanji text-ink-faint" style="font-size: 30px">門</span>
				<div class="text-ink-soft text-sm">The queue is clear</div>
				<div class="text-ink-mute text-xs">Nothing is waiting for review across your scopes.</div>
			</div>
		{/if}

		{#each groups as g (g.scope)}
			<div style="margin-top: 18px">
				<div class="flex items-center gap-2" style="margin-bottom: 8px">
					<span class="kanji text-ink-mute text-sm">規</span>
					<span class="text-ink-mute font-semibold uppercase" style="font-size: 11px; letter-spacing: 0.12em"
						>{g.scope}</span
					>
					<span class="mono text-ink-faint text-xs">{g.rows.length}</span>
				</div>
				<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
					{#each g.rows as c, i (c.signature)}
						{@const band = similarityBand(c.similarity)}
						<a
							href={resolve('/(console)/console/triage/[signature]', {
								signature: c.signature
							})}
							class="grid items-center gap-4 no-underline {i < g.rows.length - 1 ? 'border-paper-edge border-b' : ''}"
							style="grid-template-columns: auto 1fr auto auto auto; padding: 13px 16px"
						>
							<span class="kanji text-accent text-center" style="font-size: 18px; width: 22px">{kindKanji(c.kind)}</span>
							<div style="min-width: 0">
								<div class="text-ink text-sm">{c.title}</div>
								<div class="flex flex-wrap items-center gap-2" style="margin-top: 5px">
									<DojoChip>{kindLabel(c.kind)}</DojoChip>
									<span class="mono text-ink-faint text-xs">
										{c.contributor_count}
										{c.contributor_count === 1 ? 'contributor' : 'contributors'} · {relativeAge(c.created_at)}
									</span>
								</div>
							</div>
							<div class="flex items-center justify-end gap-2" style="min-width: 96px">
								{#if band === 'merge' || band === 'flagged'}
									<DojoChip toneClass={similarityToneClass(c.similarity)}>
										sim {(c.similarity ?? 0).toFixed(2)}
									</DojoChip>
								{/if}
							</div>
							<Confidence value={c.confidence} />
							<span class="text-ink-faint text-sm">→</span>
						</a>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>
