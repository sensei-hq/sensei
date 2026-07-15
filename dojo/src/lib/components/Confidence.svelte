<script lang="ts">
	import { confidencePct, confidenceToneClass, confidenceWidth } from '$lib/triage-view';

	// Slim confidence bar + value (mockup DojoConsole `Confidence`). The bar fill
	// and the numeric value are tinted by the value's token tone; geometry
	// (width/height) is inline because the design system doesn't model it.
	let { value, width = 84 }: { value: number | null; width?: number } = $props();

	const toneClass = $derived(confidenceToneClass(value));
</script>

<div class="flex items-center gap-2">
	<div
		class="bg-paper-mute overflow-hidden rounded-full"
		style="width: {width}px; height: 4px"
	>
		<div
			class="h-full rounded-full {toneClass}"
			style="width: {confidenceWidth(value)}; background: currentColor"
		></div>
	</div>
	<span class="mono text-ink-soft text-xs">{confidencePct(value)}</span>
</div>
