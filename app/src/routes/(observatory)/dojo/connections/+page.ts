import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { DojoMembership } from '$lib/types.js';

// The daemon returns a top-level array of connections — empty when the
// signed-in persona belongs to no dōjō yet. A failed read is an error to retry,
// never that empty list: the empty state tells you to sign in.
export const load: PageLoad = async (): Promise<{ connections: DojoMembership[]; error: string | null }> => {
  const res = await senseiApi(appState.port).tryGetDojoMemberships();
  if (!res.ok) return { connections: [], error: res.error.message };
  return { connections: res.data, error: null };
};
