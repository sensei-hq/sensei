import type { PageLoad } from './$types.js';
import { appState } from '$lib/appstate.svelte.js';
import { relayApi, type RelayFilter } from '$lib/relay-api.js';

/** The main window's Relay: every crew, narrowed by the URL's repository or
 *  workspace (repository ruling, 2026-11-05 — 67 of 157 repositories belong to no
 *  workspace, so the screen cannot live in a workspace window alone). */
export const load: PageLoad = async ({ url }) => {
  const api = relayApi(appState.port);
  const filter: RelayFilter = {};
  const repo = url.searchParams.get('repo');
  if (repo) filter.repo_key = repo;
  const workspace = url.searchParams.get('workspace');
  if (workspace) filter.workspace_id = workspace;
  const [now, queue] = await Promise.all([api.tryGetNow(filter), api.tryGetQueue(filter)]);
  return {
    filter,
    now: now.ok ? now.data : null,
    nowError: now.ok ? null : now.error,
    queue: queue.ok ? queue.data : null,
    queueError: queue.ok ? null : queue.error,
  };
};
