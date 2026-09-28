import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

// The SUMMARY only — one row per (repository × reason), aggregated in SQL. The
// per-metric rows are fetched per repository on selection, because "every
// repository × every metric" is a cross join: 1,943 rows on a normal install but
// 10.9M on a large one, where reading it all exhausts the request.
//
// A failed read is a real error, not "no repositories": every install has
// repositories, so an empty list here would mean something is broken. Return
// `error` so the screen shows error-with-Retry rather than an empty settings page
// that reads as "nothing to configure" (no-fabrication, F8).
export const load: PageLoad = async () => {
  const res = await senseiApi(appState.port).tryGetMetricStatusSummary();
  if (!res.ok) return { summary: null, error: res.error.message };
  return { summary: res.data, error: null };
};
