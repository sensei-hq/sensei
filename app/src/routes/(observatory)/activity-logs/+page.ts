import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { filtersFromParams, logsPageData, readLogs, readTasks } from './activity-logs.svelte.js';

// Server-filtered log read plus the background-worker registry (#96). The URL
// carries the log filter state (level / source / module / since / limit); the
// controls refetch by navigating with a new query. `capped` flags a full page
// so the screen can say "narrow the range" instead of silently hiding older
// rows. `tasks` is what the daemon's schedulers are and when they last ran —
// fetched in parallel. A failed read of either is an error state, not an empty
// list. The reads themselves are `readLogs` / `readTasks`, so the screen's
// Retry asks for exactly what this loader asked for.
export const load: PageLoad = async ({ url }) => {
  const api = senseiApi(appState.port);
  const filters = filtersFromParams(url.searchParams);
  const [rows, scheduled] = await Promise.all([readLogs(api, filters), readTasks(api)]);
  return logsPageData(rows, scheduled, filters);
};
