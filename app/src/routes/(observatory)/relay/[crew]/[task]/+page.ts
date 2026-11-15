import type { PageLoad } from './$types.js';
import { appState } from '$lib/appstate.svelte.js';
import { relayApi } from '$lib/relay-api.js';

/** One task in the main window: the same read, scoped by (crew, task). */
export const load: PageLoad = async ({ params }) => {
  const api = relayApi(appState.port);
  const answer = await api.tryGetTask(params.crew, params.task);
  return {
    crewId: params.crew,
    taskId: params.task,
    view: answer.ok ? answer.data : null,
    error: answer.ok ? null : answer.error,
  };
};
