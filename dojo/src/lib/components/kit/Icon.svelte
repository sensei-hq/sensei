<script lang="ts" module>
	// The kit's K2Icon named Solar (`solar:{name}-bold-duotone`) icons and painted
	// them as remote <img> tags. Dōjō already ships a Solar-glyph icon collection
	// through presetRokkit (`i-glyph:*`, see @rokkit/unocss README) — the app's
	// established `i-solar` idiom, resolved at build time with no network fetch and
	// no new dependency. This map bridges the handful of kit Solar names whose glyph
	// key differs; any name not listed falls through to `i-glyph:{name}` verbatim.
	const GLYPH_ALIAS: Record<string, string> = {
		'code-2': 'code',
		command: 'terminal',
		'lock-keyhole': 'lock',
		'shield-user': 'shield',
		'clipboard-check': 'checklist-minimalistic',
		'pen-2': 'edit',
		'share-circle': 'share'
	};

	/** Resolve a kit Solar icon name to its `i-glyph:*` utility class. */
	export function glyphClass(name: string): string {
		return `i-glyph:${GLYPH_ALIAS[name] ?? name}`;
	}
</script>

<script lang="ts">
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
	class="inline-block flex-shrink-0 {glyphClass(name)} {toneClass}"
	style="width: {size}px; height: {size}px"
	aria-hidden="true"
></span>
