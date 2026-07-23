<script lang="ts" module>
	// dial id → the leading i-glyph icon. Any id not listed falls back to
	// `settings`. Kept module-scoped so the map isn't rebuilt per instance.
	const DIAL_ICON: Record<string, string> = {
		autonomy: 'cpu-bolt',
		sharing: 'share-circle',
		review: 'checklist-minimalistic'
	};
</script>

<script lang="ts">
	import Icon from './Icon.svelte';
	import type { KitStanceDial } from './types';

	// A stance dial (kit K2StanceDial) — a labelled discrete slider over an axis
	// (autonomy / sharing / review). The dots snap between the named levels; the
	// current level's label shows in accent. Selecting a dot updates the local
	// position and forwards `onChange(id, index)` to the caller.
	let {
		dial,
		onChange
	}: { dial: KitStanceDial; onChange?: (id: string, value: number) => void } = $props();

	// Seed the dial position once from the prop, then let it be locally editable
	// (the mockup's useState(dial.value)). Reading the initial value inside the
	// $state initializer would only capture the first render; `$derived` keeps it
	// in sync if the caller swaps `dial`, and the click handler drives the UI.
	let picked = $state<number | null>(null);
	const v = $derived(picked ?? dial.value);
	const n = $derived(dial.levels.length);
	const steps = $derived(dial.levels.map((_, i) => i));

	function pick(i: number) {
		picked = i;
		onChange?.(dial.id, i);
	}
</script>

<div class="bg-paper-soft border-paper-edge rounded-lg border" style="padding: 16px">
	<div class="flex items-center gap-2" style="margin-bottom: 4px">
		<Icon name={DIAL_ICON[dial.id] ?? 'settings'} size={17} toneClass="text-accent" />
		<span class="text-ink text-sm font-medium">{dial.label}</span>
		<span class="flex-1"></span>
		<span class="mono text-accent text-xs">{dial.levels[v]}</span>
	</div>
	<div class="mono text-ink-mute text-xs" style="margin-bottom: 12px">{dial.caption}</div>
	<div class="flex items-center" style="gap: 0">
		{#each steps as i (i)}
			<button
				type="button"
				onclick={() => pick(i)}
				title={dial.levels[i]}
				aria-label={dial.levels[i]}
				aria-pressed={i === v}
				class="flex-shrink-0 cursor-pointer rounded-full border-2 {i <= v
					? 'border-accent bg-accent'
					: 'border-paper-edge bg-paper'}"
				style="width: 14px; height: 14px"
			></button>
			{#if i < n - 1}
				<span class="{i < v ? 'bg-accent' : 'bg-paper-edge'}" style="flex: 1; height: 2px"></span>
			{/if}
		{/each}
	</div>
	<div class="flex justify-between" style="margin-top: 6px">
		<span class="text-ink-faint text-xs">{dial.levels[0]}</span>
		<span class="text-ink-faint text-xs">{dial.levels[n - 1]}</span>
	</div>
</div>
