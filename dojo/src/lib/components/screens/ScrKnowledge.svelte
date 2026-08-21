<script lang="ts" module>
	// Catalog kind → row icon (mockup `K2_EXT_ICON`). A screen-local map because
	// the catalog's agent/command/skill glyphs only appear here.
	const EXT_ICON: Record<string, string> = {
		agent: 'user-hands',
		command: 'command',
		skill: 'star'
	};
</script>

<script lang="ts">
	import { SectionHead, Banner, Btn, Chip, Icon, ListSection, KanjiToken } from '$lib/components/kit';
	import type { KitKnowledge } from '$lib/components/kit/types';

	// The maintainer knowledge library (mockup ScrKnowledge) — the published
	// teachings the dōjō has adopted (Active), what's due to age out (Pending
	// prune), and the catalog of shareable extensions (skills · agents ·
	// commands). The section header carries the prune-policy readout.
	// Presentational: the page supplies the library (kit fixtures this chunk).
	let {
		orgName,
		knowledge,
		mobile = false
	}: {
		orgName: string;
		knowledge: KitKnowledge;
		mobile?: boolean;
	} = $props();
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · govern'} title="Knowledge">
		{#snippet right()}
			<div
				class="bg-paper border-paper-edge inline-flex items-center gap-1 rounded-lg border"
				style="padding: 6px 10px"
			>
				<Icon name="alt-arrow-down" size={14} toneClass="text-ink-mute" />
				<span class="text-ink-soft text-sm whitespace-nowrap">{knowledge.prunePolicy}</span>
			</div>
		{/snippet}
	</SectionHead>

	<Banner
		kanji="蔵"
		tone="neutral"
		title="The published library the dōjō has adopted — and what's due to be pruned."
	>
		Adopted knowledge distributes to every machine in scope. Unused teachings age out by the prune
		policy so the shared mind stays lean.
	</Banner>

	<ListSection
		icon="check-circle"
		iconToneClass="text-success"
		title="Active"
		count={knowledge.active.length}
	>
		{#each knowledge.active as r (r.title)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<KanjiToken char={r.kanji} size="lg" toneClass="text-accent" w={22} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">{r.title}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
						{r.scope} · {r.adopted} · {r.age}
					</div>
				</div>
				<Btn size="sm" variant="ghost" icon="pen-2">Edit</Btn>
			</div>
		{/each}
	</ListSection>

	<ListSection
		icon="trash-bin-minimalistic"
		title="Pending prune"
		count={knowledge.pending.length}
	>
		{#each knowledge.pending as r (r.title)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<KanjiToken char={r.kanji} size="lg" toneClass="text-ink-mute" w={22} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink-soft text-sm">{r.title}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 1px">{r.scope} · {r.age}</div>
				</div>
				<Btn size="sm" variant="ghost" icon="restart">Keep</Btn>
			</div>
		{/each}
	</ListSection>

	<ListSection icon="widget-4" title="Catalog · skills, agents & commands" count={knowledge.catalog.length}>
		{#each knowledge.catalog as c (c.title)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<Icon name={EXT_ICON[c.kind] ?? 'widget-4'} size={18} toneClass="text-accent" />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm">{c.title}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 1px">{c.scope}</div>
				</div>
				<Chip mono toneClass="text-ink-mute">{c.kind}</Chip>
			</div>
		{/each}
	</ListSection>
</div>
