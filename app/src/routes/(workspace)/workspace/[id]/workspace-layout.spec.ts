import { describe, it, expect } from 'vitest';
import { loadWorkspaceLayout } from './workspace-layout.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const ftr = { ftr14d: 0.8, ftr14dPrev: 0.7, ftrTrend: [0.7, 0.8], sessions7d: 4 };

// #288: a workspace list that could not be read is not "workspace not found".
describe('loadWorkspaceLayout', () => {
  it('a failed list read throws the failure, not a 404', async () => {
    const api = { tryListWorkspaces: async () => down, tryGetWorkspaceFtr: async () => ok(ftr) };
    await expect(loadWorkspaceLayout(api, 'p1')).rejects.toMatchObject({
      status: 503,
      body: { message: 'Could not read the workspaces: Service Unavailable' },
    });
  });

  it('a workspace the daemon does not list is a 404', async () => {
    const api = { tryListWorkspaces: async () => ok([{ id: 'other', name: 'other' }]), tryGetWorkspaceFtr: async () => ok(ftr) };
    await expect(loadWorkspaceLayout(api, 'p1')).rejects.toMatchObject({ status: 404 });
  });

  it('a failed FTR read is carried as an error beside the workspace', async () => {
    const api = { tryListWorkspaces: async () => ok([{ id: 'p1', name: 'p1' }]), tryGetWorkspaceFtr: async () => down };
    const d = await loadWorkspaceLayout(api, 'p1');
    expect([d.workspace.name, d.ftrMetrics, d.ftrError, d.workspaceId]).toEqual(['p1', null, 'Service Unavailable', 'p1']);
  });
});
