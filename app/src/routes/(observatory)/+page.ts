import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import { toRecentSessions } from './recent-sessions.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  // The daemon assembles the whole Today screen (maturity gate, koan,
  // insights, adopted, recent sessions) plus the FTR projection — the screen
  // must not decide early-vs-mature, so we just fetch and hand it off.
  const [today, ftr] = await Promise.all([
    api.getObservatoryToday(),
    api.getObservatoryFtr(),
  ]);

  // Re-shape the daemon's recent sessions into the display contract (label
  // fallback, ftr → 0/1, drop contentless ghost rows #61, cap at the spec's
  // ≤5) so RecentSessions stays a pure template.
  const recentSessions = toRecentSessions(today.recentSessions, 5);

  return { today, ftr, recentSessions };
};
