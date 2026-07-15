<script lang="ts">
	// Tiny sparkline — geometry only, stroke inherits currentColor so callers
	// tint it with a token utility (e.g. text-accent).
	let { data }: { data: number[] } = $props();

	const points = $derived.by(() => {
		const max = Math.max(...data);
		const min = Math.min(...data);
		return data
			.map((v, i) => {
				const x = (i / (data.length - 1)) * 64;
				const y = 22 - ((v - min) / (max - min || 1)) * 20;
				return `${x.toFixed(1)},${y.toFixed(1)}`;
			})
			.join(' ');
	});
</script>

<svg width="64" height="24" viewBox="0 0 64 24" style="overflow: visible" aria-hidden="true">
	<polyline
		{points}
		fill="none"
		stroke="currentColor"
		stroke-width="1.6"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
</svg>
