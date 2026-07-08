import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { SessionRange } from './sessions-digest.js';

// Default the digest to the last 7 days — the daily-use window the mockup
// opens on. Range chips refetch client-side for 30d / 90d. Fallback is the
// empty digest (handled by the api client) so a daemon hiccup renders the
// quiet state rather than a broken screen.
const DEFAULT_RANGE: SessionRange = '7d';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const digest = await api.getSessionsDigest(DEFAULT_RANGE);
  return { sessions: digest.sessions, range: DEFAULT_RANGE };
};
