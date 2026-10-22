import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { overviewPageData } from './overview-view.svelte.js';

// The consolidated overview endpoint assembles the landing pane — header, top
// rec, stats, recent sessions — and hotspots and patterns come beside it (#85).
// Each read that fails is carried as an error the page shows, never replaced by
// an all-quiet default that reads as a calm workspace.
export const load: PageLoad = async ({ params }) => {
  const api = senseiApi(appState.port);
  const [overview, hotspots, patterns] = await Promise.all([
    api.tryGetWorkspaceOverview(params.id),
    api.tryGetWorkspaceHotspots(params.id),
    api.tryGetWorkspacePatterns(params.id),
  ]);
  return overviewPageData(overview, hotspots, patterns);
};
