<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { allDownstream, downstreamSourceCount } from '$lib/developer-view';

	// For me · downstream (mockup DojoDevDownstream): the practice the
	// contributor's teams approved, distributed down to every scope they're in. It
	// arrives in their observatory's Today & Upgrades; mute or pin anything that
	// doesn't fit. Read-mostly + this is a personal surface, so there is NO
	// join-gate (DJ1). Approved teachings only flow to members, so a membership-
	// less contributor sees the honest empty state ("no teachings yet"). Mute/pin
	// are presentational for now (no `/v1` mutation wired).
	let { hasMembership }: { hasMembership: boolean } = $props();

	const rows = $derived(hasMembership ? allDownstream() : []);
	const sources = downstreamSourceCount();
</script>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="贈"
		eyebrow="You · downstream"
		title="Approved for you"
		sub="Practice your teams approved, distributed to every scope you're in. It arrives in your observatory's Today & Upgrades — mute or pin anything that doesn't fit your work."
	>
		{#snippet right()}
			{#if hasMembership}
				<DojoChip toneClass="text-ink-soft">across {sources} memberships</DojoChip>
			{/if}
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto p-4 md:p-6">
		{#if rows.length === 0}
			<div
				class="bg-paper-soft border-paper-edge flex flex-col items-center rounded-xl border text-center"
				style="padding: 32px 24px; max-width: 460px; margin: 24px auto"
			>
				<span class="kanji text-accent text-3xl" style="line-height: 1">贈</span>
				<h2
					class="display text-ink text-lg font-normal"
					style="letter-spacing: -0.01em; margin: 16px 0 0"
				>
					no teachings yet
				</h2>
				<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 8px 0 0">
					once your teams approve practice, it flows down to every scope you're in and lands here.
					no memberships yet — join or create a Dōjō from the switcher.
				</p>
			</div>
		{:else}
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
				{#each rows as it, i (it.id)}
					<div
						class="grid items-center gap-3 {i < rows.length - 1 ? 'border-paper-edge border-b' : ''}"
						style="grid-template-columns: auto 1fr auto auto; padding: 12px 16px"
					>
						<span class="kanji text-accent text-lg" style="width: 22px; text-align: center"
							>{it.kanji}</span
						>
						<div style="min-width: 0">
							<div class="text-ink text-sm">{it.title}</div>
							<div class="flex flex-wrap items-center gap-2" style="margin-top: 4px">
								<DojoChip toneClass="text-ink-soft">{it.from}</DojoChip>
								<span class="mono text-ink-faint text-xs">{it.scope} · {it.when} ago</span>
							</div>
						</div>
						{#if it.adopted}
							<DojoChip toneClass="text-success">✓ adopted</DojoChip>
						{:else}
							<DojoChip toneClass="text-accent">new</DojoChip>
						{/if}
						<div class="flex gap-2">
							<button
								type="button"
								class="mono text-ink-mute border-paper-edge bg-paper rounded border text-xs"
								style="padding: 4px 8px; cursor: pointer">mute</button
							>
							<button
								type="button"
								class="mono text-accent border-accent-edge bg-paper rounded border text-xs"
								style="padding: 4px 8px; cursor: pointer">pin</button
							>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>
