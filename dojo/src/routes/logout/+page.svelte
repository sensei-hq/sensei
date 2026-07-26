<script lang="ts">
	import { getContext, onMount } from 'svelte';
	import { goto } from '$app/navigation';

	// Served /logout route. kavach.config.js declares `routes.logout: '/logout'`,
	// but the pinned kavach (1.0.0-next.37) does NOT serve that path itself (the
	// upstream "served configurable /logout" is a later change), so a bare
	// navigation or link to /logout 404s. This page IS that route: the supabase
	// session is client-managed (hydrated into the kavach context in the root
	// +layout), so on mount it runs the same client signOut() the kit LogoutButton
	// uses, then returns to /signin. Marked `public` in kavach.config rules so the
	// sentry guard lets it render even when the session is already partly cleared.
	const kavach = getContext<Record<string, unknown>>('kavach');

	onMount(async () => {
		const signOut = kavach?.signOut as (() => Promise<unknown>) | undefined;
		try {
			await signOut?.();
		} finally {
			// invalidateAll reruns loads so the sentry guard re-reads the (now
			// cleared) session for the destination route.
			await goto('/signin', { invalidateAll: true });
		}
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<p class="text-ink-mute text-sm">Signing out…</p>
</div>
