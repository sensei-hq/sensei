<script lang="ts">
	import { SectionHead, Btn, Icon, StanceDial, LadderRung } from '$lib/components/kit';
	import type { KitStanceDial, KitLadderRung } from '$lib/components/kit/types';
	import { personalRungs } from '$lib/personal-view';

	// Your constitution (mockup ScrConstitution) — the personal-zone governance
	// screen. Your stance dials (autonomy · sharing · review) over a three-up
	// grid, then your effective standing constitution: the personal + stack
	// rungs that apply to every project you touch. A "Rule packs →" link routes
	// to the adoptable bundles. Presentational: the page supplies the stance +
	// ladder (kit fixtures this chunk); the dials forward `onStance(id, value)`
	// and the packs link fires `onGoPacks` — real /v1 wiring lands later.
	let {
		stance = [],
		ladder = [],
		onStance,
		onGoPacks
	}: {
		stance?: KitStanceDial[];
		ladder?: KitLadderRung[];
		onStance?: (id: string, value: number) => void;
		onGoPacks?: () => void;
	} = $props();

	const rungs = $derived(personalRungs(ladder));
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead
		kanji="静" eyebrow="You · standing rules" title="Your constitution"
		description="Everything sensei keeps is derived and stays on your machine. Your stance sets how far a session runs and what surfaces to a dōjō. A classification change alters which rules apply — never what leaves."
	>
		{#snippet right()}
			<Btn size="sm" variant="ghost" icon="box" onclick={onGoPacks}>Rule packs →</Btn>
		{/snippet}
	</SectionHead>

	<div class="grid gap-4 grid-cols-1 md:grid-cols-3">
		{#each stance as dial (dial.id)}
			<StanceDial {dial} onChange={onStance} />
		{/each}
	</div>

	<div>
		<div class="flex items-center gap-2 mb-3">
			<Icon name="scale" size={17} toneClass="text-accent" />
			<span class="text-ink text-xs font-semibold uppercase" style="letter-spacing: 0.18em">
				Your effective constitution
			</span>
			<span class="mono text-ink-mute text-xs">— personal + stack · every project you touch</span>
		</div>
		<div class="flex flex-col gap-3">
			{#each rungs as rung (rung.id)}
				<LadderRung {rung} />
			{/each}
		</div>
	</div>
</div>
