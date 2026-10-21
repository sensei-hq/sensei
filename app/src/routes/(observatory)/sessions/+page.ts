import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { loadSessionsDigest, sessionsFetcher } from '$lib/sessions-digest.svelte.js';

// The last 7 days — the daily-use window the mockup opens on. Range chips
// refetch client-side for 30d / 90d. A failed read is carried as its error.
export const load: PageLoad = () => loadSessionsDigest(sessionsFetcher(senseiApi(appState.port)), '7d');
