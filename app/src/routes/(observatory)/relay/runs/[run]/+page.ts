import type { PageLoad } from './$types.js';
import { appState } from '$lib/appstate.svelte.js';
import { relayApi } from '$lib/relay-api.js';

/** A past run in the main window: the same read, by its id. */
export const load: PageLoad = async ({ params }) => {
  const answer = await relayApi(appState.port).tryGetRun(params.run);
  return {
    runId: params.run,
    detail: answer.ok ? answer.data : null,
    error: answer.ok ? null : answer.error,
  };
};
