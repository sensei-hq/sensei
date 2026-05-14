import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const [ftrData, projects] = await Promise.all([
    api.getHolisticFtrDaily(14),
    api.listProjects(),
  ]);

  // Build per-project FTR for sidebar
  const projectFtrs = await Promise.all(
    (projects as any[]).slice(0, 10).map(async (p: any) => {
      const ftr = await api.getProjectFtr(p.id);
      return {
        id: p.id,
        name: p.name,
        kanji: p.icon?.value ?? '場',
        ftr: ftr.ftr14d ?? 0,
        sessions7d: ftr.sessions7d ?? 0,
      };
    })
  );

  // Gather teachings and recommendations from first project (if any)
  let teachings: any[] = [];
  let topRecommendations: any[] = [];
  if (projectFtrs.length > 0) {
    const firstId = projectFtrs[0].id;
    const [t, r] = await Promise.all([
      api.getProjectTeachings(firstId, 5),
      api.getProjectRecommendations(firstId, 'pending'),
    ]);
    teachings = t.teachings ?? [];
    topRecommendations = (r as any[]).slice(0, 3);
  }

  return {
    ftrDaily: ftrData.ftr_daily,
    projectFtrs,
    teachings,
    topRecommendations,
  };
};
