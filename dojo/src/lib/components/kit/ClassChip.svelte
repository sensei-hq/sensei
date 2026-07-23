<script lang="ts">
	import { classTone } from './vocab';

	// Classification chip (kit K2ClassChip) — company · client · personal ·
	// community. A tone dot + the label, tinted from the shared vocab. `kind` is
	// the project classification.
	let { kind }: { kind: string } = $props();

	const c = $derived(classTone(kind));
</script>

<span
	class="inline-flex items-center gap-1 whitespace-nowrap rounded-full border text-xs {c.text} {c.soft} {c.edge}"
	style="padding: 3px 9px"
>
	<span class="flex-shrink-0 rounded-full {c.fill}" style="width: 6px; height: 6px"></span>{c.label}
</span>
