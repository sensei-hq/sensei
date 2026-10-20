import { describe, it, expect } from 'vitest';
import { loadProjectLayout } from './project-layout.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const ftr = { ftr14d: 0.8, ftr14dPrev: 0.7, ftrTrend: [0.7, 0.8], sessions7d: 4 };

// #288: a project list that could not be read is not "project not found".
describe('loadProjectLayout', () => {
  it('a failed list read throws the failure, not a 404', async () => {
    const api = { tryListProjects: async () => down, tryGetProjectFtr: async () => ok(ftr) };
    await expect(loadProjectLayout(api, 'p1')).rejects.toMatchObject({
      status: 503,
      body: { message: 'Could not read the projects: Service Unavailable' },
    });
  });

  it('a project the daemon does not list is a 404', async () => {
    const api = { tryListProjects: async () => ok([{ id: 'other', name: 'other' }]), tryGetProjectFtr: async () => ok(ftr) };
    await expect(loadProjectLayout(api, 'p1')).rejects.toMatchObject({ status: 404 });
  });

  it('a failed FTR read is carried as an error beside the project', async () => {
    const api = { tryListProjects: async () => ok([{ id: 'p1', name: 'p1' }]), tryGetProjectFtr: async () => down };
    const d = await loadProjectLayout(api, 'p1');
    expect([d.project.name, d.ftrMetrics, d.ftrError, d.projectId]).toEqual(['p1', null, 'Service Unavailable', 'p1']);
  });
});
