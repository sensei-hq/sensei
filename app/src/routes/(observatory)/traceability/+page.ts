import type { PageLoad } from './$types.js';
import { senseiApi } from '$lib/api.js';
import { appState } from '$lib/appstate.svelte.js';
import type { ProjectListItem } from '$lib/types.js';
import {
  groupDriftByProject,
  rollupDrift,
  toDriftRow,
  type DriftRow,
} from './traceability-state.svelte.js';

export const load: PageLoad = async () => {
  const api = senseiApi(appState.port);
  const projects = (await api.listProjects()) as ProjectListItem[];

  // Fan out `/api/projects/{id}/drift` across every project — the same
  // cross-project pattern the Impact and Upgrades loaders use (~297 calls on
  // this install). No cap: a silent truncation would hide drift on later
  // projects, and a clean project returns an empty list, so a fresh install
  // simply resolves to the EmptyState.
  const perProject = await Promise.all(
    projects.map(async (p) => {
      const { items } = await api.getProjectDrift(p.id);
      return items.map((it): DriftRow => toDriftRow(it, p));
    }),
  );

  const all = perProject.flat();
  return { groups: groupDriftByProject(all), rollup: rollupDrift(all) };
};
