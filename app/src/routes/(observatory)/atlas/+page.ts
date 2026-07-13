import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem } from '$lib/types.js';
import { buildAtlasPage } from './atlas-graph.svelte.js';

// The graph endpoints are keyed by repo *name* (get_repo_by_name), so the
// atlas scopes by name. `?repo=` overrides; the sensei monorepo is the
// default because it's the one guaranteed to be indexed on a dev machine.
const DEFAULT_REPO = 'sensei';

export const load: PageLoad = async ({ url }) => {
  const api = senseiApi(appState.port);
  const repoId = url.searchParams.get('repo') || DEFAULT_REPO;

  // Fan the four graph reads out together. `detectCommunities` (POST) refreshes
  // the clustering before we read `communities/info`, so the overview reflects
  // the current index rather than a stale run.
  const [projects, communities, callFlow, graph] = await Promise.all([
    api.listProjects() as Promise<ProjectListItem[]>,
    api.detectCommunities(repoId).then(() => api.getCommunities(repoId)),
    api.getCallFlow(repoId),
    api.getGraphNodes(repoId),
  ]);

  // The solution roll-up is keyed by project UUID. Resolve it only when the
  // scope name matches a known project; it's empty until repo↔project
  // membership is populated, so it contributes counts, not the primary graph.
  const project = projects.find((p) => p.name === repoId);
  const sol = project ? await api.getProjectGraph(project.id) : null;
  const solution = sol ? { repos: sol.repos, nodes: sol.nodes, edges: sol.edges } : null;

  return buildAtlasPage({ repoId, projects, communities, callFlow, graph, solution });
};
