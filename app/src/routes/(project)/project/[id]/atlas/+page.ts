import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem } from '$lib/types.js';
import { buildAtlasPage, type AtlasScope } from './atlas-graph.svelte.js';

/** A repo-root folder as the projects endpoint returns it. */
interface ProjectFolder { id: string; kind: string; name: string; }

/** THIS project's selectable scopes. The daemon graph is keyed by repo name and
 *  a project IS its repos, so the scopes are the project's git roots. Fall back
 *  to the project itself when no git folder is recorded yet. */
function repoScopes(project: ProjectListItem): AtlasScope[] {
  const folders = (project.folders as ProjectFolder[] | undefined) ?? [];
  const repos = folders
    .filter((f) => f.kind === 'git')
    .map((f) => ({ id: f.id, name: f.name }));
  return repos.length ? repos : [{ id: project.id, name: project.name }];
}

// The graph endpoints are keyed by repo *name* (get_repo_by_name). A project's
// atlas scopes to the project's OWN repos; `?repo=` switches among them, and the
// solution roll-up is the project graph. No cross-project "default repo" — the
// graph shown is always this project's.
export const load: PageLoad = async ({ params, url, parent }) => {
  const api = senseiApi(appState.port);
  // (project)/project/[id]/+layout.ts already loaded the project (name + folders).
  const { project } = (await parent()) as { project: ProjectListItem };

  const scopes = repoScopes(project);
  const repoId = url.searchParams.get('repo') || scopes[0].name;

  // Fan the graph reads out together. `detectCommunities` (POST) refreshes the
  // clustering before we read `communities/info`, so the overview is current.
  const [communities, callFlow, graph, sol] = await Promise.all([
    api.detectCommunities(repoId).then(() => api.getCommunities(repoId)),
    api.getCallFlow(repoId),
    api.getGraphNodes(repoId),
    api.getProjectGraph(params.id),
  ]);
  const solution = sol ? { repos: sol.repos, nodes: sol.nodes, edges: sol.edges } : null;

  return buildAtlasPage({ repoId, scopes, communities, callFlow, graph, solution });
};
