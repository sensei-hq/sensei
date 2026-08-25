<script lang="ts">
	import { SectionHead, Banner, Btn, Icon, ListSection, KanjiToken } from '$lib/components/kit';
	import type { KitEngagement, KitConfidentiality } from '$lib/components/kit/types';

	// The lead client-engagement register (mockup ScrEngagements) — the clients
	// this dōjō serves, each with the kept-vs-dropped confidentiality count
	// (lessons that crossed the boundary anonymized vs identifiers/source
	// stripped). Below the register, the confidentiality model: what crosses the
	// boundary, and a raw → anonymized code example. Presentational: the page
	// supplies the engagements + model (kit fixtures this chunk).
	let {
		orgName,
		engagements = [],
		confidentiality,
		onNew,
		onClose,
		onDelete
	}: {
		orgName: string;
		engagements?: KitEngagement[];
		confidentiality: KitConfidentiality;
		/** Register a new engagement (lead) — prompts for a client name. Absent =
		 *  read-only. */
		onNew?: (client: string) => void;
		/** Close an engagement (lead) — PATCH status=ended, retained for audit. */
		onClose?: (e: KitEngagement) => void;
		/** Hard-delete an engagement (lead). */
		onDelete?: (e: KitEngagement) => void;
	} = $props();

	// A minimal create flow consistent with the kit (no modal component yet): a
	// one-line client-name prompt → onNew. Kept lean rather than over-built.
	function create() {
		if (!onNew) return;
		const client = typeof window !== 'undefined' ? window.prompt('New engagement — client name?')?.trim() : '';
		if (client) onNew(client);
	}
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow={orgName + ' · clients'} title="Engagements" count={engagements.length}>
		{#snippet right()}
			<Btn size="sm" icon="add-circle" onclick={create}>New engagement</Btn>
		{/snippet}
	</SectionHead>

	<Banner kanji="盾" tone="accent" title="Share the lesson, never the source.">
		Findings from a client project are anonymized before they leave the machine — the pattern
		travels upstream; the client, repo and code never do.
	</Banner>

	<ListSection icon="case-round" title="Client engagements" count={engagements.length}>
		{#each engagements as e (e.id)}
			<div class="border-paper-edge flex items-center gap-4 border-b" style="padding: 12px 16px">
				<KanjiToken char={e.kanji} size="lg" toneClass="text-accent" w={22} />
				<div class="flex-1" style="min-width: 0">
					<div class="text-ink text-sm font-medium">{e.client}</div>
					<div class="mono text-ink-faint text-xs" style="margin-top: 1px">
						{e.projects} · since {e.since}
					</div>
				</div>
				<div style="text-align: right">
					<div class="mono text-ink text-sm">{e.lessons}</div>
					<div class="text-ink-faint text-xs">lessons kept</div>
				</div>
				<div style="text-align: right">
					<div class="mono text-ink-mute text-sm">{e.dropped}</div>
					<div class="text-ink-faint text-xs">stripped</div>
				</div>
				<Btn size="sm" variant="ghost" icon="document">Audit</Btn>
				{#if onClose && e.status !== 'ended'}
					<Btn size="sm" variant="ghost" icon="lock-keyhole" onclick={() => onClose?.(e)}>Close</Btn>
				{/if}
				{#if onDelete}
					<Btn size="sm" variant="ghost" icon="trash-bin-trash" onclick={() => onDelete?.(e)}>
						Delete
					</Btn>
				{/if}
			</div>
		{/each}
	</ListSection>

	<div class="grid gap-4 grid-cols-1 md:grid-cols-2">
		<!-- What crosses the boundary — kept (✓) vs dropped (✕). -->
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div
				class="text-ink-mute text-xs font-semibold uppercase"
				style="letter-spacing: 0.18em; margin-bottom: 12px"
			>
				What crosses the boundary
			</div>
			<div class="flex flex-col gap-2">
				{#each confidentiality.kept as x (x)}
					<div class="text-ink flex items-center gap-2 text-sm">
						<Icon name="check-circle" size={15} toneClass="text-success" />{x}
					</div>
				{/each}
				{#each confidentiality.dropped as x (x)}
					<div class="text-ink-soft flex items-center gap-2 text-sm">
						<Icon name="close-circle" size={15} toneClass="text-danger" />{x}
					</div>
				{/each}
			</div>
		</div>

		<!-- Anonymized before it leaves — the raw → stripped example. -->
		<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
			<div
				class="text-ink-mute text-xs font-semibold uppercase"
				style="letter-spacing: 0.18em; margin-bottom: 12px"
			>
				Anonymized before it leaves
			</div>
			<div
				class="bg-paper-mute border-paper-edge text-ink-soft mono rounded-lg border text-xs"
				style="padding: 12px; text-decoration: line-through"
			>
				{confidentiality.example.raw}
			</div>
			<div class="text-ink-faint flex items-center justify-center" style="padding: 4px">↓</div>
			<div
				class="bg-success-soft border-success-soft text-ink mono rounded-lg border text-xs"
				style="padding: 12px"
			>
				{confidentiality.example.stripped}
			</div>
		</div>
	</div>
</div>
