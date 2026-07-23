<script lang="ts">
	// A meaningful number + label (kit K2StatBadge) — sensei's small, specific
	// stat. `toneClass` tints the big number (default ink); `sub` is an optional
	// meta line under the label.
	let {
		n,
		label,
		sub,
		toneClass = 'text-ink'
	}: { n: number | string; label: string; sub?: string; toneClass?: string } = $props();
</script>

<div class="flex flex-col" style="gap: 2px">
	<div class="display text-2xl font-light {toneClass}" style="line-height: 1">{n}</div>
	<div class="text-ink text-xs font-medium">{label}</div>
	{#if sub}<div class="mono text-ink-mute text-xs">{sub}</div>{/if}
</div>
