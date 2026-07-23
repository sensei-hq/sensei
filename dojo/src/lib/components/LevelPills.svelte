<script lang="ts">
	import { LIB_LEVELS, type LibLevelId } from '$lib/library-data';

	// The Org / Team / Project / Stack level selector (mockup LevelPills): a
	// segmented control that picks the level a rule (or a pack, or an authored
	// draft) applies at — cascading down the same ladder as authored governance.
	// Presentational — the current value in, an onChange out. Token-only.
	let { value, onChange }: { value: LibLevelId; onChange: (level: LibLevelId) => void } = $props();
</script>

<div class="bg-paper-mute inline-flex items-center rounded-full" style="padding: 2px; gap: 2px">
	{#each LIB_LEVELS as level (level.id)}
		{@const on = level.id === value}
		<button
			type="button"
			onclick={() => onChange(level.id)}
			title="apply at {level.label}"
			aria-pressed={on}
			class="inline-flex items-center gap-1 rounded-full text-xs {on
				? 'bg-paper text-ink font-semibold'
				: 'text-ink-mute font-normal'}"
			style="padding: 2px 8px; border: none; cursor: pointer; font-family: inherit"
		>
			<span class="kanji text-xs {on ? 'text-accent' : 'text-ink-faint'}">{level.kanji}</span
			>{level.label}
		</button>
	{/each}
</div>
