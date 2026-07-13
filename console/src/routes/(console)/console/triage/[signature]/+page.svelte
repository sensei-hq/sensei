<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import EnsoRing from '$lib/components/EnsoRing.svelte';
	import { decideTriage, TriageApiError, type DecideStatus } from '$lib/triage-data';
	import {
		confidencePct,
		kindLabel,
		scopeLabel,
		similarityBand,
		similarityToneClass
	} from '$lib/triage-view';

	// Candidate detail — evaluate → decide (mockup DojoCandidate). Shows the fields
	// the triage API exposes (title · kind · scope · confidence · contributors ·
	// similarity to the nearest artifact) and the maintainer decide action:
	// approve (requires a distribution scope), revise, or decline (requires a
	// reason). The fuller artifact body (learning/cause/context) isn't returned by
	// the triage endpoint — flagged inline where the mockup shows it.
	let { data } = $props();

	const row = $derived(data.row);
	const scope = $derived(row ? scopeLabel(row.owner_scope) : '');

	// Decide state. Distribution scope defaults to the row's owner scope (its
	// binding) — the safe default; approve is blocked until a scope is set (the
	// API rejects an approve with no distribution_scope, mirrored here).
	let reason = $state('');
	let deciding = $state<DecideStatus | null>(null);
	let decideError = $state<string | null>(null);

	async function decide(status: DecideStatus) {
		if (!row) return;
		decideError = null;
		// Client-side guard mirroring the API's done-gates (surface before the round
		// trip): approve needs a distribution scope; decline needs a reason.
		if (status === 'decline' && reason.trim() === '') {
			decideError = 'Declining needs a reason.';
			return;
		}
		deciding = status;
		try {
			await decideTriage(
				data.tenantKey,
				row.signature,
				{
					status,
					// Approve binds to the row's owner scope by default (narrowing is
					// free; broadening beyond the binding would ask to confirm — deferred).
					distribution_scope: status === 'approve' ? row.owner_scope : undefined,
					reason: reason.trim() || undefined
				},
				{ fetch, accessToken: data.accessToken }
			);
			// On success the row leaves the open queue — return to the queue.
			await goto(resolve('/(console)/console/triage'));
		} catch (e) {
			decideError = e instanceof TriageApiError ? e.message : 'the decision could not be recorded';
			deciding = null;
		}
	}
</script>

<svelte:head>
	<title>{row?.title ?? 'Candidate'} · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<!-- breadcrumb -->
	<div class="border-paper-edge flex flex-shrink-0 items-center gap-2 border-b" style="padding: 12px 28px">
		<a
			href={resolve('/(console)/console/triage')}
			class="mono text-accent text-xs no-underline">← Triage</a
		>
		{#if row}
			<span class="text-ink-faint text-xs">/</span>
			<span class="mono text-ink-mute text-xs">{scope}</span>
		{/if}
	</div>

	{#if !row}
		<div class="flex flex-1 flex-col items-center justify-center gap-3 text-center" style="padding: 40px">
			<span class="kanji text-ink-faint" style="font-size: 34px">検</span>
			{#if data.error}
				<div class="text-ink-soft text-sm">Live triage is unavailable</div>
				<div class="mono text-ink-mute text-xs">{data.error}</div>
			{:else}
				<div class="text-ink-soft text-sm">This candidate isn't in the open queue</div>
				<div class="text-ink-mute text-xs" style="max-width: 380px; line-height: 1.5">
					It may already have been decided, or the signature is unknown. Head back to the queue.
				</div>
			{/if}
			<a
				href={resolve('/(console)/console/triage')}
				class="bg-paper border-paper-edge text-ink mono rounded-lg border text-xs no-underline"
				style="padding: 8px 14px; margin-top: 4px">← Back to triage</a
			>
		</div>
	{:else}
		{@const band = similarityBand(row.similarity)}
		<div class="grid min-h-0 flex-1" style="grid-template-columns: 1fr 332px">
			<!-- main -->
			<div class="overflow-auto" style="padding: 24px 28px 32px">
				<div class="flex flex-wrap gap-2" style="margin-bottom: 12px">
					<DojoChip>{kindLabel(row.kind)}</DojoChip>
					<DojoChip toneClass="text-ink-soft">{scope}</DojoChip>
					<DojoChip toneClass="text-ink-mute">{row.state}</DojoChip>
				</div>
				<h1
					class="display text-ink font-light"
					style="font-size: 27px; letter-spacing: -0.015em; line-height: 1.18; margin: 0 0 20px"
				>
					{row.title}
				</h1>

				<!-- evidence -->
				<div class="bg-paper-soft border-paper-edge rounded-xl border" style="padding: 13px 16px; margin-bottom: 16px">
					<div class="flex items-center gap-2">
						<span class="kanji text-ink-mute text-sm">証</span>
						<span class="text-ink text-sm"
							><b class="font-semibold">{row.contributor_count}</b>
							{row.contributor_count === 1 ? 'contributor' : 'contributors'} support this</span
						>
					</div>
				</div>

				<!-- similarity / near-duplicate (Evaluate — the merge suggestion band) -->
				{#if band === 'merge' || band === 'flagged'}
					<div class="bg-paper-soft border-paper-edge rounded-xl border" style="padding: 13px 15px; margin-bottom: 16px">
						<div class="flex items-center gap-2" style="margin-bottom: 9px">
							<span class="kanji text-ink-mute text-sm">双</span>
							<span class="text-ink-mute font-semibold uppercase" style="font-size: 11px; letter-spacing: 0.12em"
								>Near-duplicate · {band === 'merge' ? 'merge suggested' : 'flagged for review'}</span
							>
						</div>
						<div class="border-paper-edge flex items-center gap-3 border-b" style="padding: 7px 0">
							<span class="text-ink text-sm" style="flex: 1"
								>Nearest artifact{row.nearest_artifact_id
									? ` · ${row.nearest_artifact_id.slice(0, 8)}`
									: ''}</span
							>
							<span class="mono text-xs {similarityToneClass(row.similarity)}">sim {(row.similarity ?? 0).toFixed(2)}</span>
						</div>
						<div class="text-ink-soft text-xs" style="margin-top: 10px; line-height: 1.55">
							<span class="text-ink-mute">Auto-dedupe · </span>≥ 0.90 merges automatically; 0.75–0.90 is flagged here for
							you to confirm.
						</div>
					</div>
				{/if}

				<!-- the fuller artifact body isn't exposed by the triage API -->
				<div
					class="border-ink-faint text-ink-mute rounded-xl border border-dashed text-xs"
					style="padding: 13px 16px; line-height: 1.55"
				>
					<span class="kanji text-ink-faint text-sm">芽</span>
					The learning, cause and context (and the evidence sessions) live on the artifact body, which the triage
					endpoint doesn't return yet — evaluate from the signals here, or open the artifact once that endpoint lands.
				</div>
			</div>

			<!-- decide rail -->
			<div
				class="border-paper-edge bg-paper-soft flex flex-col gap-5 overflow-auto border-l"
				style="padding: 22px 20px"
			>
				<div class="flex flex-col items-center gap-2">
					<EnsoRing progress={row.confidence ?? 0} label={confidencePct(row.confidence)} />
					<span class="text-ink-mute uppercase" style="font-size: 11px; letter-spacing: 0.12em">Confidence</span>
				</div>

				<div>
					<div
						class="text-ink-faint font-semibold uppercase"
						style="font-size: 10.5px; letter-spacing: 0.14em; margin-bottom: 6px"
					>
						Contributors
					</div>
					<div class="text-ink-soft text-xs" style="line-height: 1.5">
						{row.contributor_count} distinct
						{row.contributor_count === 1 ? 'contributor' : 'contributors'} behind this cluster.
					</div>
				</div>

				<div>
					<div class="flex items-center" style="margin-bottom: 6px">
						<span class="text-ink-faint font-semibold uppercase" style="font-size: 10.5px; letter-spacing: 0.14em"
							>Distribute to</span
						>
					</div>
					<div
						class="bg-paper border-paper-edge flex items-center gap-2 rounded-lg border"
						style="padding: 9px 11px"
					>
						<span class="kanji text-accent text-sm">規</span>
						<span class="text-ink text-sm" style="flex: 1">{scope}</span>
						<span class="mono text-ink-faint text-xs uppercase" style="letter-spacing: 0.06em">binding</span>
					</div>
					<div class="text-ink-faint text-xs" style="margin-top: 7px; line-height: 1.45">
						Approve binds to the contribution's scope. Narrowing is free; broadening beyond it asks you to confirm.
					</div>
				</div>

				<!-- reason (required to decline; optional note otherwise) -->
				<div>
					<label
						for="decide-reason"
						class="text-ink-faint font-semibold uppercase"
						style="font-size: 10.5px; letter-spacing: 0.14em; display: block; margin-bottom: 6px">Decision note</label
					>
					<textarea
						id="decide-reason"
						bind:value={reason}
						placeholder="Reason (required to decline)…"
						rows="3"
						class="bg-paper border-paper-edge text-ink w-full rounded-lg border text-sm"
						style="padding: 9px 11px; box-sizing: border-box; resize: vertical; font-family: inherit"
					></textarea>
				</div>

				{#if decideError}
					<div class="bg-warning-soft border-warning-edge text-ink-soft rounded-lg border text-xs" style="padding: 9px 11px">
						{decideError}
					</div>
				{/if}

				<div
					class="border-paper-edge flex flex-col gap-2 border-t"
					style="padding-top: 16px; margin-top: auto"
				>
					<button
						type="button"
						onclick={() => decide('approve')}
						disabled={deciding !== null}
						class="bg-ink text-on-primary inline-flex items-center justify-center gap-2 rounded-lg text-sm font-medium"
						style="padding: 11px; border: none; cursor: pointer"
						style:opacity={deciding !== null ? 0.6 : 1}
					>
						<span class="kanji text-accent" style="font-size: 13px">決</span>
						{deciding === 'approve' ? 'Approving…' : 'Approve & publish'}
					</button>
					<div class="flex gap-2">
						<button
							type="button"
							onclick={() => decide('revise')}
							disabled={deciding !== null}
							class="bg-paper border-paper-edge text-ink-soft flex-1 rounded-lg border text-sm"
							style="padding: 9px; cursor: pointer"
							style:opacity={deciding !== null ? 0.6 : 1}
						>
							{deciding === 'revise' ? 'Revising…' : 'Revise'}
						</button>
						<button
							type="button"
							onclick={() => decide('decline')}
							disabled={deciding !== null}
							class="bg-paper border-paper-edge text-ink-soft flex-1 rounded-lg border text-sm"
							style="padding: 9px; cursor: pointer"
							style:opacity={deciding !== null ? 0.6 : 1}
						>
							{deciding === 'decline' ? 'Declining…' : 'Decline'}
						</button>
					</div>
					<div class="text-ink-faint text-center text-xs" style="margin-top: 2px">
						A named decision, with a note, lands in the audit trail.
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
