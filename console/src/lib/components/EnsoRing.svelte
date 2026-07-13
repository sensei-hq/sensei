<script lang="ts">
	// Enso-style progress ring with a centred label (mockup candidate decide-rail
	// confidence dial). Stroke inherits currentColor so callers tint it with a
	// token utility class (e.g. text-accent); the track uses paper-mute.
	let {
		progress,
		size = 104,
		stroke = 8,
		label
	}: { progress: number; size?: number; stroke?: number; label: string } = $props();

	const radius = $derived((size - stroke) / 2);
	const circumference = $derived(2 * Math.PI * radius);
	const clamped = $derived(Math.max(0, Math.min(1, progress)));
	const dash = $derived(circumference * clamped);
</script>

<div class="text-accent relative inline-flex items-center justify-center" style="width: {size}px; height: {size}px">
	<svg width={size} height={size} viewBox="0 0 {size} {size}" style="transform: rotate(-90deg)" aria-hidden="true">
		<circle
			class="text-paper-mute"
			cx={size / 2}
			cy={size / 2}
			r={radius}
			fill="none"
			stroke="currentColor"
			stroke-width={stroke}
		/>
		<circle
			cx={size / 2}
			cy={size / 2}
			r={radius}
			fill="none"
			stroke="currentColor"
			stroke-width={stroke}
			stroke-linecap="round"
			stroke-dasharray="{dash} {circumference}"
		/>
	</svg>
	<span class="display text-ink absolute font-light" style="font-size: 28px">{label}</span>
</div>
