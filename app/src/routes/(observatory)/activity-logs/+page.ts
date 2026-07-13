import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { filtersFromParams } from './activity-logs.svelte.js';

// Server-filtered log read. The URL carries the filter state (level / source /
// module / since / limit); the controls refetch by navigating with a new
// query. `capped` flags a full page so the screen can say "narrow the range"
// instead of silently hiding older rows.
export const load: PageLoad = async ({ url }) => {
  const api = senseiApi(appState.port);
  const filters = filtersFromParams(url.searchParams);

  const rows = await api.getLogs({
    level: filters.level || undefined,
    source: filters.source || undefined,
    module: filters.module || undefined,
    since: filters.since,
    limit: filters.limit,
  });

  return { rows, filters, capped: rows.length >= filters.limit };
};
