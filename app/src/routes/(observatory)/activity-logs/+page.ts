import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { filtersFromParams } from './activity-logs.svelte.js';

// Server-filtered log read plus the background-worker registry (#96). The URL
// carries the log filter state (level / source / module / since / limit); the
// controls refetch by navigating with a new query. `capped` flags a full page
// so the screen can say "narrow the range" instead of silently hiding older
// rows. `tasks` is what the daemon's schedulers are and when they last ran —
// fetched in parallel; its client falls back to an empty list on a hiccup.
export const load: PageLoad = async ({ url }) => {
  const api = senseiApi(appState.port);
  const filters = filtersFromParams(url.searchParams);

  const [rows, scheduled] = await Promise.all([
    api.getLogs({
      level: filters.level || undefined,
      source: filters.source || undefined,
      module: filters.module || undefined,
      since: filters.since,
      limit: filters.limit,
    }),
    api.getScheduledTasks(),
  ]);

  return {
    rows,
    filters,
    capped: rows.length >= filters.limit,
    tasks: scheduled.tasks,
  };
};
