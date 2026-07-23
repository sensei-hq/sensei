<script lang="ts">
	import Icon from './Icon.svelte';
	import { roleTone } from './vocab';

	// Additive role tag (kit K2RoleTag) — an icon + role label. `muted` renders it
	// in a neutral ink treatment; otherwise it carries the accent tone. Renders
	// nothing for an unknown role.
	let { role, muted = false }: { role: string; muted?: boolean } = $props();

	const r = $derived(roleTone(role));
	const toneClass = $derived(muted ? 'text-ink-mute' : 'text-accent');
	const softClass = $derived(muted ? 'bg-paper-mute' : 'bg-accent-soft');
	const edgeClass = $derived(muted ? 'border-paper-edge' : 'border-accent-soft');
</script>

{#if r}
	<span
		class="mono inline-flex items-center gap-1 whitespace-nowrap rounded-full border text-xs {toneClass} {softClass} {edgeClass}"
		style="padding: 3px 10px"
	>
		<Icon name={r.icon} size={13} {toneClass} />{r.label}
	</span>
{/if}
