import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// The daemon always returns a full collective-preferences object on success —
// the stored row, or the defaults when unset. A fetch FAILURE is therefore a real
// error, not "empty": return `error` so the screen shows error-with-Retry instead
// of fabricating the defaults and hiding a broken daemon (no-fabrication, F8).
export const load: PageLoad = async () => {
  const res = await senseiApi(appState.port).tryGetCollectivePreferences();
  if (!res.ok) return { preferences: null, error: res.error.message };
  return { preferences: res.data, error: null };
};
