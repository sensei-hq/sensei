<script lang="ts">
	import ConsoleHead from '$lib/components/ConsoleHead.svelte';
	import DojoChip from '$lib/components/DojoChip.svelte';
	import { allContributions, contributionTally, statusMeta } from '$lib/developer-view';
	import { CONTRIB_DEVS_HELPED } from '$lib/developer-data';

	// My contributions (mockup DojoDevContributions): the lessons the contributor
	// sent upstream and where each one stands per destination. Read-mostly — the
	// contributor proposes; a maintainer decides. This is a personal surface, so
	// there is NO join-gate (DJ1). Upstream sends only exist once you belong to a
	// Dōjō, so a membership-less contributor sees the honest empty state ("nothing
	// shared upstream yet") rather than fabricated rows.
	let { hasMembership }: { hasMembership: boolean } = $props();

	const rows = $derived(hasMembership ? allContributions() : []);
	const tally = contributionTally();
</script>

<div class="bg-paper flex h-full w-full flex-col overflow-hidden">
	<ConsoleHead
		kanji="共"
		eyebrow="You · upstream"
		title="What you've shared"
		sub="Lessons you sent up to a Dōjō, and where each one stands. You propose; a maintainer decides — nothing publishes without their named approval."
	>
		{#snippet right()}
			{#if hasMembership}
				<div class="mono text-ink-mute text-xs" style="text-align: right; line-height: 1.7">
					<div>
						<b class="text-success">{tally.approved}</b> approved ·
						<b class="text-accent">{tally.pending}</b> pending
					</div>
					<div>{CONTRIB_DEVS_HELPED} devs helped · lifetime</div>
				</div>
			{/if}
		{/snippet}
	</ConsoleHead>

	<div class="flex-1 overflow-auto p-4 md:p-6">
		{#if rows.length === 0}
			<div
				class="bg-paper-soft border-paper-edge flex flex-col items-center rounded-xl border text-center"
				style="padding: 32px 24px; max-width: 460px; margin: 24px auto"
			>
				<span class="kanji text-accent text-3xl" style="line-height: 1">共</span>
				<h2
					class="display text-ink text-lg font-normal"
					style="letter-spacing: -0.01em; margin: 16px 0 0"
				>
					nothing shared upstream yet
				</h2>
				<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 8px 0 0">
					share from the observatory's ready-to-share lane and it lands in a bound Dōjō's triage
					queue. no memberships yet — join or create a Dōjō from the switcher to start
					contributing.
				</p>
			</div>
		{:else}
			<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-xl border">
				{#each rows as c, i (c.id)}
					{@const sm = statusMeta(c.status)}
					<div
						class="grid items-center gap-3 {i < rows.length - 1 ? 'border-paper-edge border-b' : ''}"
						style="grid-template-columns: auto 1fr auto auto; padding: 12px 16px"
					>
						<span class="kanji text-accent text-lg" style="width: 22px; text-align: center"
							>{c.kanji}</span
						>
						<div style="min-width: 0">
							<div class="text-ink text-sm">{c.title}</div>
							<div class="flex flex-wrap items-center gap-2" style="margin-top: 4px">
								<DojoChip toneClass={c.client ? 'text-accent' : 'text-ink-soft'}>
									{#if c.client}盾 {/if}{c.dest}
								</DojoChip>
								<span class="mono text-ink-faint text-xs">{c.scope} · {c.note}</span>
							</div>
						</div>
						<DojoChip toneClass={sm.toneClass}>{sm.label}</DojoChip>
						<span class="mono text-ink-faint text-xs" style="width: 28px; text-align: right"
							>{c.when}</span
						>
					</div>
				{/each}
			</div>

			<div
				class="bg-paper-soft border-paper-edge flex items-center gap-2 rounded-xl border"
				style="margin-top: 16px; padding: 12px 16px"
			>
				<span class="kanji text-accent text-base">芽</span>
				<span class="text-ink-soft text-sm flex-1" style="line-height: 1.5">
					You share from the observatory's
					<b class="text-ink font-semibold">ready-to-share</b> lane; it lands in the bound Dōjō's
					triage queue. Track the outcome here.
				</span>
			</div>
		{/if}
	</div>
</div>
