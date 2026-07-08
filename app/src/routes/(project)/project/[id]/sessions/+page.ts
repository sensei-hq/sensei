import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { SessionRange } from '$lib/sessions-digest.js';

// Default the digest to the last 7 days — the same daily-use window the
// Observatory Sessions screen opens on (its shared peer). Range chips refetch
// client-side for 30d / 90d, scoped to this project. Fallback is the empty
// digest (handled by the api client) so a daemon hiccup renders the quiet
// state rather than a broken screen.
const DEFAULT_RANGE: SessionRange = '7d';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const digest = await senseiApi(appState.port).getSessionsDigest(DEFAULT_RANGE, params.id);
  return { project, projectId: params.id, sessions: digest.sessions, range: DEFAULT_RANGE };
};
