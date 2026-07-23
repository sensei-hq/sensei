<script lang="ts">
	import Icon from './Icon.svelte';
	import Chip from './Chip.svelte';
	import type { KitConflict } from './types';

	// A conflict card (kit K2ConflictCard) — topic · the losing rule yields to the
	// winning rule · why, with a lock marker when a ★ non-negotiable decided it.
	// The loser reads struck-through in a sunken well; the winner sits in a success
	// tint. On phones the two sides stack (the `→` becomes a downward flow via
	// `flex-col`); `md:` lays them side by side.
	let { conflict }: { conflict: KitConflict } = $props();
</script>

<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
	<div class="border-paper-edge flex items-center gap-2 border-b" style="padding: 12px 16px">
		<Icon name="danger-triangle" size={17} toneClass="text-warning" />
		<span class="text-ink flex-1 text-sm font-medium">{conflict.topic}</span>
		{#if conflict.locked}
			<Chip
				icon="lock-keyhole"
				toneClass="text-accent"
				softClass="bg-accent-soft"
				edgeClass="border-accent-soft">locked</Chip
			>
		{:else}
			<Chip>settled</Chip>
		{/if}
	</div>
	<div class="flex flex-col items-stretch gap-3 md:flex-row md:items-center" style="padding: 16px">
		<div class="bg-paper-mute flex-1 rounded" style="padding: 12px; opacity: 0.7">
			<div class="text-ink-mute text-xs font-semibold uppercase" style="letter-spacing: 0.18em; margin-bottom: 4px">
				{conflict.loser.level} · yields
			</div>
			<div class="text-ink-soft text-sm" style="text-decoration: line-through">
				{conflict.loser.text}
			</div>
		</div>
		<div class="text-ink-faint flex items-center justify-center" aria-hidden="true">→</div>
		<div class="bg-success-soft border-success-soft flex-1 rounded border" style="padding: 12px">
			<div class="text-success text-xs font-semibold uppercase" style="letter-spacing: 0.18em; margin-bottom: 4px">
				{conflict.winner.level} · wins
			</div>
			<div class="text-ink text-sm font-medium">{conflict.winner.text}</div>
		</div>
	</div>
	<div class="border-paper-edge bg-paper text-ink-soft border-t text-sm" style="padding: 12px 16px; line-height: 1.55">
		{conflict.why}
	</div>
</div>
