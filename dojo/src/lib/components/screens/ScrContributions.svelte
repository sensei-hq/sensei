<script lang="ts" module>
	// status → chip tone/label/icon (mockup `K2_CSTAT`). The mockup rode raw
	// oklch vars; here every status resolves to named token classes so the chip
	// stays theme-free. `pending` reads as "in triage".
	const CSTAT: Record<
		string,
		{ toneClass: string; softClass: string; edgeClass: string; label: string; icon: string }
	> = {
		approved: {
			toneClass: 'text-success',
			softClass: 'bg-success-soft',
			edgeClass: 'border-success-soft',
			label: 'approved',
			icon: 'check-circle'
		},
		pending: {
			toneClass: 'text-accent',
			softClass: 'bg-accent-soft',
			edgeClass: 'border-accent-soft',
			label: 'in triage',
			icon: 'hourglass'
		},
		declined: {
			toneClass: 'text-danger',
			softClass: 'bg-danger-soft',
			edgeClass: 'border-danger-soft',
			label: 'declined',
			icon: 'close-circle'
		}
	};
</script>

<script lang="ts">
	import { SectionHead, Banner, StatBadge, ListSection, KanjiToken, Chip, Btn } from '$lib/components/kit';
	import type { KitContribution, KitDownstream, KitContribStat } from '$lib/components/kit/types';
	import { tallyContributions } from '$lib/personal-view';

	// Contributions (mockup ScrContributions) — merges the old "contributions +
	// downstream" into one screen. The stat row (approved · in triage — the
	// "devs helped" tile is dropped, F5 Q4: no real metric, no fabricated number),
	// then what you've shared upstream with its per-destination status, then
	// what's been approved for you downstream (never silently merged — you pin it
	// in). You propose; a maintainer decides — nothing publishes without their
	// named approval; every source is dereferenced before it leaves (universal,
	// always-on — the `anonymous` marker is credit-only). Presentational: the page
	// supplies the rows; `onPin` fires the real adopt write for a pinned item.
	let {
		mine = [],
		downstream = [],
		stat,
		onPin
	}: {
		mine?: KitContribution[];
		downstream?: KitDownstream[];
		stat?: KitContribStat;
		onPin?: (item: KitDownstream) => void;
	} = $props();

	// Tally the stat row off the shared rows so it stays honest against the list
	// it summarizes; the lifetime `helped` count carries from the fixture stat.
	const tally = $derived(tallyContributions(mine, stat ?? { approved: 0, pending: 0, helped: 0 }));
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow="You · sharing" title="Contributions">
		{#snippet right()}
			<div class="flex items-center gap-4">
				<StatBadge n={tally.approved} label="approved" toneClass="text-success" />
				<StatBadge n={tally.pending} label="in triage" toneClass="text-accent" />
			</div>
		{/snippet}
	</SectionHead>

	<Banner
		kanji="共"
		tone="neutral"
		title="You propose; a maintainer decides — nothing publishes without their named approval."
	>
		You share from a session's ready-to-share lane; it lands in the bound dōjō's triage queue.
		Every source is dereferenced before it leaves — the lesson travels, the source never does.
	</Banner>

	<ListSection icon="upload-square" title="What you've shared" count={mine.length}>
		{#each mine as c, i (i)}
			{@const s = CSTAT[c.status] ?? CSTAT.pending}
			<div class="border-paper-edge flex items-center gap-4 border-b px-4 py-3">
				<KanjiToken char={c.kanji} size="lg" toneClass="text-accent" w={22} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm" style="line-height: 1.3">{c.title}</div>
					<div class="flex flex-wrap items-center gap-2" style="margin-top: 2px">
						{#if c.anonymous}
							<Chip
								icon="eye-closed"
								toneClass="text-accent"
								softClass="bg-accent-soft"
								edgeClass="border-accent-soft">{c.dest} · anonymous</Chip
							>
						{:else}
							<Chip>{c.dest}</Chip>
						{/if}
						<span class="mono text-ink-faint text-xs">{c.scope} · {c.note}</span>
					</div>
				</div>
				<Chip
					icon={s.icon}
					toneClass={s.toneClass}
					softClass={s.softClass}
					edgeClass={s.edgeClass}>{s.label}</Chip
				>
				<span class="mono text-ink-faint text-xs" style="width: 28px; text-align: right">{c.when}</span>
			</div>
		{/each}
	</ListSection>

	<ListSection
		icon="download-square"
		iconToneClass="text-success"
		title="Approved for you"
		count={downstream.length}
	>
		{#snippet right()}
			<span class="mono text-ink-mute text-xs">from your dōjōs · never silently merged</span>
		{/snippet}
		{#each downstream as it, i (i)}
			<div class="border-paper-edge flex items-center gap-4 border-b px-4 py-3">
				<KanjiToken char={it.kanji} size="lg" toneClass="text-accent" w={22} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm" style="line-height: 1.3">{it.title}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 2px">
						{it.from} · {it.scope} · {it.when} ago
					</div>
				</div>
				{#if it.adopted}
					<Chip
						icon="check-circle"
						toneClass="text-success"
						softClass="bg-success-soft"
						edgeClass="border-success-soft">adopted</Chip
					>
				{:else}
					<Chip toneClass="text-accent" softClass="bg-accent-soft" edgeClass="border-accent-soft"
						>new</Chip
					>
					<Btn size="sm" variant="ghost" icon="pin" onclick={() => onPin?.(it)}>Pin</Btn>
				{/if}
			</div>
		{/each}
	</ListSection>
</div>
