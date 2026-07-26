<script lang="ts">
	import { getContext } from 'svelte';
	import { goto } from '$app/navigation';

	// Shared logout control (kit) — the symmetric partner to DojoSignIn's
	// signIn. There is NO server `/logout` route: the supabase session is
	// client-managed by the browser kavach instance (hydrated in +layout.svelte
	// and exposed on context), so logging out is a client call — signOut() then
	// return to the sign-in route. A plain `<a href="/logout">` 404s because
	// nothing serves that declared path.
	//
	// `compact` = the phone treatment (round, icon-only). Default = the desktop
	// pill (出 + a label that shows from md:).
	let { compact = false }: { compact?: boolean } = $props();

	const kavach = getContext<Record<string, unknown>>('kavach');
	let busy = $state(false);

	async function logout() {
		if (busy) return;
		busy = true;
		const signOut = kavach?.signOut as (() => Promise<unknown>) | undefined;
		try {
			await signOut?.();
		} finally {
			// Return to the kavach auth route; invalidateAll reruns loads so the
			// sentry guard re-reads the (now cleared) session for every route.
			await goto('/signin', { invalidateAll: true });
			busy = false;
		}
	}

	const shape = $derived(
		compact
			? 'h-[34px] w-[34px] justify-center rounded-full'
			: 'gap-2 rounded-lg'
	);
</script>

<button
	type="button"
	onclick={logout}
	disabled={busy}
	title="Log out"
	aria-label="Log out"
	class="text-ink-soft hover:text-ink border-paper-edge flex flex-shrink-0 items-center border bg-transparent {shape}"
	style={compact ? undefined : 'padding: 4px 10px'}
>
	<span class="kanji text-ink-mute text-xs" aria-hidden="true">出</span>
	{#if !compact}<span class="hidden text-xs md:inline">Log out</span>{/if}
</button>
