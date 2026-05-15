import { error } from '@sveltejs/kit';
import { appState } from '$lib/appstate.svelte.js';
import { senseiApi } from '$lib/api.js';
import type { LayoutLoad } from './$types.js';

// (project)/+layout.ts has already ensured appState is loaded.
export const load: LayoutLoad = async ({ params }) => {
  const api = senseiApi(appState.port);
  const [projects, ftrMetrics] = await Promise.all([
    api.listProjects(),
    api.getProjectFtr(params.id),
  ]);
  const project = projects.find((p: any) => p.id === params.id);
  if (!project) throw error(404, `Project ${params.id} not found`);
  return { project, ftrMetrics, projectId: params.id };
}
