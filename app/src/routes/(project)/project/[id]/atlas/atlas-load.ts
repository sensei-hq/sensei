import { failureReason, type ApiResult } from '$lib/api.js';
import type { CommunityInfo, GraphCallEdge, GraphSymbolNode } from '$lib/types.js';
import { buildAtlasPage, type AtlasPageData, type AtlasScope } from './atlas-graph.svelte.js';

/** The reads the atlas makes — injected so a spec can fail them. */
export interface AtlasApi {
  tryDetectCommunities: (repoId: string) => Promise<ApiResult<unknown>>;
  tryGetCommunities: (repoId: string) => Promise<ApiResult<CommunityInfo[]>>;
  tryGetCallFlow: (repoId: string) => Promise<ApiResult<{ moduleCount: number; exportCount: number; callCount: number }>>;
  tryGetGraphNodes: (repoId: string) => Promise<ApiResult<{ nodes: GraphSymbolNode[]; edges: GraphCallEdge[] }>>;
  tryGetProjectGraph: (projectId: string) => Promise<ApiResult<{ repos: number; nodes: number; edges: number }>>;
}

/** The project as the atlas needs it: its id, name and folders. */
export interface AtlasProject {
  id: string;
  name: string;
  folders?: ReadonlyArray<{ id: string; kind: string; name: string }>;
}

/** THIS project's selectable scopes. The daemon graph is keyed by repo name and
 *  a project IS its repos, so the scopes are the project's git roots — or the
 *  project itself when no git folder is recorded yet. */
export function repoScopes(project: AtlasProject): AtlasScope[] {
  const repos = (project.folders ?? []).filter((f) => f.kind === 'git').map((f) => ({ id: f.id, name: f.name }));
  return repos.length ? repos : [{ id: project.id, name: project.name }];
}

/**
 * The atlas for one of the project's repos (`repo`, else its first). The graph
 * endpoints are keyed by repo *name*; the solution roll-up is the project's.
 *
 * Drawn from the daemon's reads or not at all (#288): any read that fails —
 * including the clustering refresh, without which the communities are stale —
 * is the page's error, naming it. Never an empty graph, zero calls or a 0-repo
 * roll-up standing in for a failure.
 */
export async function loadAtlas(
  api: AtlasApi,
  project: AtlasProject,
  repo: string | null,
): Promise<{ repoId: string; scopes: AtlasScope[]; atlas: AtlasPageData | null; error: string | null }> {
  const scopes = repoScopes(project);
  const repoId = repo || scopes[0].name;
  const refresh = await api.tryDetectCommunities(repoId);
  const [communities, callFlow, graph, solution] = await Promise.all([
    api.tryGetCommunities(repoId),
    api.tryGetCallFlow(repoId),
    api.tryGetGraphNodes(repoId),
    api.tryGetProjectGraph(project.id),
  ]);
  const reads = { 'refresh communities': refresh, communities, 'call flow': callFlow, graph, solution };
  const failed = Object.entries(reads).flatMap(([name, r]) => (r.ok ? [] : [`${name}: ${failureReason(r.error)}`]));
  if (failed.length || !communities.ok || !callFlow.ok || !graph.ok || !solution.ok) {
    return { repoId, scopes, atlas: null, error: `Could not read the atlas — ${failed.join('; ')}` };
  }
  const { repos, nodes, edges } = solution.data;
  return {
    repoId,
    scopes,
    atlas: buildAtlasPage({
      repoId,
      scopes,
      communities: communities.data,
      callFlow: callFlow.data,
      graph: graph.data,
      solution: { repos, nodes, edges },
    }),
    error: null,
  };
}
