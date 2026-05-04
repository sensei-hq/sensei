import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export async function load({ params, parent }: any) {
  const { project } = await parent();
  const api = senseiApi(appState.port);
  const data = await api.getProjectLibraries(params.id);
  const wrappedCount = data.libraries.filter((l: any) => l.has_instruments).length;
  const unwrappedCount = data.libraries.length - wrappedCount;
  return { project, libraries: data.libraries ?? [], wrappedCount, unwrappedCount };
}
