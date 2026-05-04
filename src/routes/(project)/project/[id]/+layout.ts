import { appState } from '$lib/appstate.svelte.js';
import { senseiApi } from '$lib/api.js';

export async function load({ params }: { params: { id: string } }) {
  await appState.load();
  const api = senseiApi(appState.port);
  const [projects, ftrMetrics] = await Promise.all([
    api.listProjects(),
    api.getProjectFtr(params.id),
  ]);
  const project = projects.find((p: any) => p.id === params.id) ?? null;
  return { project, ftrMetrics, projectId: params.id };
}
