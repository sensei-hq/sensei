<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import ConsoleBanner from '$lib/components/ConsoleBanner.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { actionLabel, actionToneClass, clockTime, relativeAge, shortId } from '$lib/admin-view';

	// Audit (mockup DojoAudit, dojo-console.jsx l.934): the immutable admin audit
	// log (data.events from list_audit) — every admin action, who, and when.
	// Read-only, most recent first. Gated by the backend ADMIN floor; a 403 or an
	// unreachable service surfaces in the banner.
	let { data } = $props();
</script>

<svelte:head>
	<title>Audit · Dōjō console</title>
</svelte:head>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="録"
		eyebrow="Trust · audit"
		title="Audit trail"
		sub="An immutable record of every admin action — who did what, and when. Non-repudiation for members, identities, and policies; the proof the console's decisions are accountable."
	>
		{#snippet right()}
			<span class="mono text-ink-mute text-xs">{data.events.length} events</span>
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto" style="padding: 24px">
		{#if data.error}
			<ConsoleBanner>
				Live audit is unavailable. <span class="mono text-ink-mute text-xs">{data.error}</span>
			</ConsoleBanner>
		{/if}

		<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border" style="margin-top: 18px">
			<div
				class="border-paper-edge text-ink-mute grid gap-3 border-b font-semibold uppercase text-xs" style="grid-template-columns: 78px 150px 1.4fr 130px 0.9fr; padding: 12px 16px; letter-spacing: 0.1em"
			>
				<span>Time</span><span>Action</span><span>Target</span><span>Actor</span><span>When</span>
			</div>

			{#if data.events.length === 0}
				<div class="text-ink-mute text-sm" style="padding: 16px 16px">
					{data.error ? 'Could not load the audit trail.' : 'No admin actions recorded yet.'}
				</div>
			{:else}
				{#each data.events as e, i (e.id)}
					<div
						class="grid items-center gap-3 {i < data.events.length - 1 ? 'border-paper-edge border-b' : ''}"
						style="grid-template-columns: 78px 150px 1.4fr 130px 0.9fr; padding: 12px 16px"
					>
						<span class="mono text-ink-faint text-xs">{clockTime(e.ts)}</span>
						<span>
							<DojoChip toneClass={actionToneClass(e.action)}>{actionLabel(e.action)}</DojoChip>
						</span>
						<span class="mono text-ink truncate text-xs" title={e.target ?? ''}>{e.target ?? '—'}</span>
						<span class="mono text-ink-soft truncate text-xs" title={e.actor_id ?? ''}>{shortId(e.actor_id, 12)}</span>
						<span class="mono text-ink-faint text-xs">{relativeAge(e.ts)}</span>
					</div>
				{/each}
			{/if}
		</div>
	</div>
</div>
