<script lang="ts">
	// A round monogram avatar (the kit's window.Avatar) — the uppercased first
	// letter of a name on a soft paper disc, matching the mockup Avatar and the
	// TopBar/ChatThread initials treatment. `size` is the diameter in px (the one
	// dimensional knob the row layouts vary); the glyph scales with it.
	// Presentational — token-native (never a raw color).
	let { name, size = 24 }: { name: string; size?: number } = $props();

	const letter = $derived((name || '?').charAt(0).toUpperCase());
</script>

<div
	class="bg-paper-mute text-ink-soft border-paper-edge inline-flex flex-shrink-0 items-center justify-center rounded-full border font-medium"
	style="width: {size}px; height: {size}px; font-size: {Math.round(size * 0.45)}px"
	aria-hidden="true"
>
	{letter}
</div>
