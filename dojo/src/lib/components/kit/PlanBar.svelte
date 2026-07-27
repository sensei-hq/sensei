<script lang="ts">
	// A thin progress rail (kit K2PlanBar). `tone` is a solid-fill token class
	// (e.g. `bg-success`); the fill width is geometry, so it stays inline. The
	// track is a paper-mute well; the fill never drops below 2% so a just-started
	// run still shows a sliver.
	let { pct, tone = 'bg-ink' }: { pct: number; tone?: string } = $props();

	const width = $derived(Math.max(2, pct));
</script>

<span class="block overflow-hidden rounded-sm bg-paper-mute" style="height: 5px">
	<span class="block h-full rounded-sm {tone}" style="width: {width}%"></span>
</span>
