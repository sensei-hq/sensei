import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  // R3 project→Dōjō auto-bind: the inferred binding for the chip + the
  // membership list so a confirmed binding can be resolved from
  // `bound_projects`. Both are fallback-safe (empty / null), so a daemon
  // that predates the route never breaks the About panel.
  const [reposData, suggestionData, memberships] = await Promise.all([
    api.getProjectRepos(params.id),
    api.getDojoSuggestion(params.id),
    api.getDojoMemberships(),
  ]);
  return {
    project,
    repos: reposData.repos ?? [],
    suggestion: suggestionData.suggestion,
    memberships,
  };
};
