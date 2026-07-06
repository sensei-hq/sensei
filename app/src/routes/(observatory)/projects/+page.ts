import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem } from '$lib/types.js';
import { bucketProjects, isWarning, type EnrichedProject } from './buckets.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const projects = await api.listProjects();

  // Per-project enrichment runs in parallel so the whole page loads in
  // one API-fanout, not N sequential round trips. Each `getProjectFtr` /
  // `getProjectQualitySignals` already falls back to zeros on the wire
  // when the daemon has no data, so no defensive coercion is needed.
  const enriched: EnrichedProject[] = await Promise.all(
    (projects as ProjectListItem[]).map(async (p) => {
      const [ftr, signals] = await Promise.all([
        api.getProjectFtr(p.id),
        api.getProjectQualitySignals(p.id),
      ]);
      return {
        ...p,
        ftr14d: ftr.ftr14d,
        sessions7d: ftr.sessions7d,
        warn: isWarning(signals),
      };
    }),
  );

  return { buckets: bucketProjects(enriched) };
};
