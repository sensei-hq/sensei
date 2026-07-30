<script lang="ts">
	import { createLogout } from './logout.svelte';

	// Shared logout control (kit) — the symmetric partner to DojoSignIn's signIn.
	// The logout action itself lives in logout.svelte.ts (also used by the TopBar
	// avatar); this is just its phone/desktop pill rendering.
	//
	// `compact` = the phone treatment (round, icon-only). Default = the desktop
	// pill (出 + a label that shows from md:).
	let { compact = false }: { compact?: boolean } = $props();

	const logout = createLogout();

	const shape = $derived(
		compact
			? 'h-[34px] w-[34px] justify-center rounded-full'
			: 'gap-2 rounded-lg'
	);
</script>

<button
	type="button"
	onclick={() => logout.run()}
	disabled={logout.busy}
	title="Log out"
	aria-label="Log out"
	class="text-ink-soft hover:text-ink border-paper-edge flex flex-shrink-0 items-center border bg-transparent {shape}"
	style={compact ? undefined : 'padding: 4px 10px'}
>
	<span class="kanji text-ink-mute text-xs" aria-hidden="true">出</span>
	{#if !compact}<span class="hidden text-xs md:inline">Log out</span>{/if}
</button>
