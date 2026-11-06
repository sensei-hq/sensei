import type { PageLoad } from './$types.js';
import { appState } from '$lib/appstate.svelte.js';
import { relayApi, type RelayFilter } from '$lib/relay-api.js';

/** The workspace window's Relay: the same screen, pre-filtered to its workspace
 *  (repository ruling, 2026-11-05). The two reads are the crew's now and its queue;
 *  a read that fails is carried as its error, never as an empty screen (#288). */
export const load: PageLoad = async ({ params }) => {
  const api = relayApi(appState.port);
  const filter: RelayFilter = { workspace_id: params.id };
  const [now, queue] = await Promise.all([api.tryGetNow(filter), api.tryGetQueue(filter)]);
  return {
    workspaceId: params.id,
    filter,
    now: now.ok ? now.data : null,
    nowError: now.ok ? null : now.error,
    queue: queue.ok ? queue.data : null,
    queueError: queue.ok ? null : queue.error,
  };
};
