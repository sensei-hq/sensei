import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// The dōjō half of Settings · Dōjō: memberships (each bound to an identity by its
// session slot) and the sync rows. The CREDENTIAL half comes from `personaList`,
// which loads client-side and is the single owner of identity standing.
//
// A failed read is a real error, not "nothing is syncing" or "no dōjō" — each is
// reported on its own, so one failing read never blanks the other (F8).
export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const [sync, memberships] = await Promise.all([api.tryGetDojoSyncState(), api.tryGetDojoMemberships()]);
  return {
    sync: sync.ok ? sync.data : null,
    error: sync.ok ? null : sync.error.message,
    memberships: memberships.ok ? memberships.data : null,
    membershipsError: memberships.ok ? null : memberships.error.message,
  };
};
