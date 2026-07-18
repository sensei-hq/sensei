<script lang="ts">
	import { statusBadge } from '$lib/relay-view';
	import type { RelayRunStatus } from '$lib/relay-data';

	// Shared run-status pill for Relay (run list + run detail). Renders the same
	// DojoChip-style rounded pill the run list drew inline — statusBadge(status)
	// gives the plain-language label + tone class — so nothing regresses. When the
	// run is 'running' a small accent-toned dot pulses ahead of the label; every
	// other status shows no dot, keeping the surface calm. Pure/presentational.
	let { status }: { status: RelayRunStatus } = $props();

	const badge = $derived(statusBadge(status));
	const running = $derived(status === 'running');
</script>

<span
	class="mono border-paper-edge inline-flex items-center rounded-full border {badge.toneClass} text-xs" style="padding: 2px 8px; letter-spacing: 0.02em; gap: 5px"
>
	{#if running}
		<span class="pulse-dot bg-accent" aria-hidden="true"></span>
	{/if}
	{badge.label}
</span>

<style>
	.pulse-dot {
		width: 5px;
		height: 5px;
		border-radius: 9999px;
		flex-shrink: 0;
		animation: relay-pulse 2s ease-in-out infinite;
	}

	@keyframes relay-pulse {
		0%,
		100% {
			opacity: 1;
			transform: scale(1);
		}
		50% {
			opacity: 0.4;
			transform: scale(0.7);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.pulse-dot {
			animation: none;
		}
	}
</style>
