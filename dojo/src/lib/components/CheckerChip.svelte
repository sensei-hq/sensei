<script lang="ts">
	// A wired open-source checker chip (mockup CheckerChip): the name of the tool
	// a stack reviewer runs (eslint, ruff, clippy…). A small dot marks it as OSS;
	// mypy (proprietary-ish licence in the mockup's convention) reads muted.
	// Presentational — name in, chip out. Token-only styling.
	let { name }: { name: string } = $props();

	const oss = $derived(name !== 'mypy');
</script>

<span
	class="mono bg-paper border-paper-edge text-ink-soft inline-flex items-center gap-1 whitespace-nowrap rounded-full border text-xs"
	style="padding: 2px 8px"
>
	<span
		class="inline-block rounded-full {oss ? 'bg-success' : 'bg-ink-faint'}"
		style="width: 5px; height: 5px"
		aria-hidden="true"
	></span>{name}
</span>
