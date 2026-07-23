<script lang="ts">
	// A functional icon — the working-icon layer of the kit. `name` is a bare
	// Solar name (`folder`, `eye`, `command`, …), following the mockup's K2Icon.
	// It's rendered directly as a class: rokkit.config.js's `icons.overrides`
	// maps each bare name → `i-solar:{name}-linear` (a build-time CSS mask tinted
	// with the element's text color, no network fetch) and auto-safelists the
	// key, so applying `{name}` dynamically here still generates the CSS.
	// Tinted via a token text class (e.g. `text-accent`); size is inline geometry
	// the scale doesn't model. Presentational: no state, no data.
	let {
		name,
		size = 18,
		toneClass = 'text-ink-mute'
	}: { name: string; size?: number; toneClass?: string } = $props();
</script>

<span
	class="inline-block flex-shrink-0 {name} {toneClass}"
	style="width: {size}px; height: {size}px"
	aria-hidden="true"
></span>
