import type { PageLoad } from './$types.js';
import { appState } from '$lib/appstate.svelte.js';
import { relayApi } from '$lib/relay-api.js';

/** One task, read by (crew, task): another crew's task, or another user's, is a
 *  404 and the screen says "not found" rather than retrying (F3). */
export const load: PageLoad = async ({ params }) => {
  const api = relayApi(appState.port);
  const answer = await api.tryGetTask(params.crew, params.task);
  return {
    workspaceId: params.id,
    crewId: params.crew,
    taskId: params.task,
    view: answer.ok ? answer.data : null,
    error: answer.ok ? null : answer.error,
  };
};
