<script lang="ts" module>
	// The kit's icon names are bare Solar names (`folder`, `eye`, `command`, …),
	// following the mockup's K2Icon. They resolve to static `i-solar:*` UnoCSS
	// classes via the kit icon map (see ./icons.ts) — build-time CSS masks tinted
	// with the element's text color, no network fetch. `iconClass` is re-exported
	// here (and through the kit barrel) so callers can resolve a class directly.
	export { iconClass } from './icons';
</script>

<script lang="ts">
	import { iconClass } from './icons';

	// A functional icon — the working-icon layer of the kit. Tinted via a token
	// text class (e.g. `text-accent`); size is inline geometry the scale doesn't
	// model. Presentational: no state, no data.
	let {
		name,
		size = 18,
		toneClass = 'text-ink-mute'
	}: { name: string; size?: number; toneClass?: string } = $props();
</script>

<span
	class="inline-block flex-shrink-0 {iconClass(name)} {toneClass}"
	style="width: {size}px; height: {size}px"
	aria-hidden="true"
></span>
