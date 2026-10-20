import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// A failed read is carried as `error` and shown, never "no patterns" (#85).
export const load: PageLoad = async ({ params, parent }) => {
  const { project } = await parent();
  const read = await senseiApi(appState.port).tryGetProjectPatterns(params.id);
  return read.ok
    ? { project, followed: read.data.followed ?? [], antiPatterns: read.data.antiPatterns ?? [], error: null }
    : { project, followed: [], antiPatterns: [], error: read.error.message || `HTTP ${read.error.status}` };
};
