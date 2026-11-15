import type { PageLoad } from './$types.js';
import { appState } from '$lib/appstate.svelte.js';
import { relayApi } from '$lib/relay-api.js';

/** A past run, read-only (CONTRACT §1i): one read by its id; a run that is not there
 *  (deleted after 30 days, or another person's) is a 404, said as "not found". */
export const load: PageLoad = async ({ params }) => {
  const answer = await relayApi(appState.port).tryGetRun(params.run);
  return {
    workspaceId: params.id,
    runId: params.run,
    detail: answer.ok ? answer.data : null,
    error: answer.ok ? null : answer.error,
  };
};
