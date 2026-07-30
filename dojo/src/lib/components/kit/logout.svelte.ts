import { getContext } from 'svelte';
import { goto } from '$app/navigation';

// Shared logout action — the single owner of "how you log out". There is NO server
// `/logout` route: the supabase session is client-managed by the browser kavach
// instance (hydrated in +layout.svelte, exposed on context), so logout is a client
// call — signOut() then return to the auth route; invalidateAll reruns loads so the
// guard re-reads the cleared session. Used by LogoutButton (mobile) and the TopBar
// avatar (desktop). Call at component init — getContext must run synchronously.
export function createLogout() {
	const kavach = getContext<Record<string, unknown>>('kavach');
	let busy = $state(false);

	return {
		get busy() {
			return busy;
		},
		async run() {
			if (busy) return;
			busy = true;
			const signOut = kavach?.signOut as (() => Promise<unknown>) | undefined;
			try {
				await signOut?.();
			} finally {
				await goto('/signin', { invalidateAll: true });
				busy = false;
			}
		}
	};
}
