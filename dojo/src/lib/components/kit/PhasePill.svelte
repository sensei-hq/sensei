<script lang="ts">
	import { phaseTone } from './vocab';

	// Phase pill (kit K2PhasePill) — a 3-step track (watch → notice → adopt) plus
	// the phase label. Lit dots use the phase's fill token; unlit dots use the
	// paper edge.
	let { phase }: { phase: string } = $props();

	const p = $derived(phaseTone(phase));
	const steps = [1, 2, 3];
</script>

<span
	class="border-paper-edge bg-paper inline-flex items-center gap-2 whitespace-nowrap rounded-full border text-xs"
	style="padding: 3px 10px 3px 8px"
>
	<span class="flex items-center" style="gap: 3px">
		{#each steps as n (n)}
			<span
				class="rounded-full {n <= p.step ? p.dot : 'bg-paper-edge'}"
				style="width: 5px; height: 5px"
			></span>
		{/each}
	</span>
	<span class={p.text}>{p.label}</span>
</span>
