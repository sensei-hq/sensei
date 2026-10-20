<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { SectionHead, EmptyState } from '$lib/components/kit';

	// A "coming in the rebuild" placeholder for every dojo nav destination not yet
	// ported (chunk 1). It keeps the two-nav complete — nothing 404s — while the
	// screens land chunk by chunk. Presentational: the shell resolves the section's
	// label + eyebrow from the nav and passes them in.
	let {
		title,
		eyebrow
	}: { title: string; eyebrow?: string } = $props();
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead {eyebrow} {title} />
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		<EmptyState kanji="築" title="Coming in the rebuild">
			This screen is being ported to the new dōjō. Its data and actions land in a later chunk — the
			nav is already wired so nothing is lost.
		</EmptyState>
	</div>
</Screen>
