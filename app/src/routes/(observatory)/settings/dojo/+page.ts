import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// Sync state only. The CREDENTIAL half comes from `personaList`, which loads
// client-side and is already the single owner of persona standing; fetching it
// again here would give the screen a second, divergent copy.
//
// A failed read is a real error, not "nothing is syncing" — reporting an empty
// list would say the sync is healthy on an install where the read itself broke,
// which for this surface is the most expensive lie available (F8).
export const load: PageLoad = async () => {
  const res = await senseiApi(appState.port).tryGetDojoSyncState();
  if (!res.ok) return { sync: null, error: res.error.message };
  return { sync: res.data, error: null };
};
