import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  // R3 project→Dōjō auto-bind: the inferred binding for the chip + the
  // membership list so a confirmed binding can be resolved from
  // `bound_projects`. A failed read is an error the section shows, never the
  // empty list or "no suggestion" a broken daemon would otherwise look like
  // (#262).
  const [repos, suggestion, memberships] = await Promise.all([
    api.tryGetProjectRepos(params.id),
    api.tryGetDojoSuggestion(params.id),
    api.tryGetDojoMemberships(),
  ]);
  return {
    project,
    repos: repos.ok ? (repos.data.repos ?? []) : [],
    reposError: repos.ok ? null : repos.error.message,
    suggestion: suggestion.ok ? suggestion.data.suggestion : null,
    memberships: memberships.ok ? memberships.data : [],
    bindingError: !suggestion.ok
      ? suggestion.error.message
      : !memberships.ok
        ? memberships.error.message
        : null,
  };
};
