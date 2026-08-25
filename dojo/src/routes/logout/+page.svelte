<script lang="ts">
	import { getContext, onMount } from 'svelte';
	import { goto } from '$app/navigation';

	// Served /logout route. kavach.config.js declares `routes.logout: '/logout'`,
	// but the pinned kavach does NOT serve that path, so the kit's `href="/logout"`
	// links (TopBar's account menu) 404. This page IS that route: the supabase
	// session is client-managed via the kavach instance hydrated into context by
	// the root +layout's onMount. Because `href="/logout"` is a FULL navigation,
	// this page can mount before that async hydration finishes — so poll briefly
	// for `signOut`, then run it and return to /signin. Marked `public` in
	// kavach.config rules so the guard lets it render mid-logout.
	const kavach = getContext<Record<string, unknown>>('kavach');

	onMount(async () => {
		let signOut: (() => Promise<unknown>) | undefined;
		for (let i = 0; i < 40 && !signOut; i++) {
			signOut = kavach?.signOut as (() => Promise<unknown>) | undefined;
			if (!signOut) await new Promise((r) => setTimeout(r, 50));
		}
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
