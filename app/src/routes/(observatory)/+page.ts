import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { todayPageData } from './today-view.svelte.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  // The daemon assembles the whole Today screen (maturity gate, koan,
  // insights, adopted, recent sessions) plus the FTR projection — the screen
  // must not decide early-vs-mature, so we just fetch and hand it off. Each
  // read that fails is carried as an error the page shows (#288).
  const [today, ftr] = await Promise.all([api.tryGetObservatoryToday(), api.tryGetObservatoryFtr()]);
  return todayPageData(today, ftr);
};
