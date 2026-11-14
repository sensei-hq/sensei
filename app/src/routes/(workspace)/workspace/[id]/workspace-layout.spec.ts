import { describe, it, expect, vi } from 'vitest';
import { loadWorkspaceLayout } from './workspace-layout.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const missing = { ok: false as const, error: { status: 404, message: 'Not Found' } };
const ftr = { ftr14d: 0.8, ftr14dPrev: 0.7, ftrTrend: [0.7, 0.8], sessions7d: 4 };

// The window reads its one workspace (it read every workspace and a folder tree
// each — 4–11 s — to show one). #288: a read that failed is not "not found".
describe('loadWorkspaceLayout', () => {
  it('reads only its own workspace', async () => {
    const api = { tryGetWorkspace: vi.fn(async () => ok({ id: 'p1', name: 'p1' })), tryGetWorkspaceFtr: async () => ok(ftr) };
    const d = await loadWorkspaceLayout(api, 'p1');
    expect(api.tryGetWorkspace).toHaveBeenCalledWith('p1');
    expect(d.workspace.name).toBe('p1');
  });

  it('a failed read throws the failure, not a 404', async () => {
    const api = { tryGetWorkspace: async () => down, tryGetWorkspaceFtr: async () => ok(ftr) };
    await expect(loadWorkspaceLayout(api, 'p1')).rejects.toMatchObject({
      status: 503,
      body: { message: 'Could not read the workspace: Service Unavailable' },
    });
  });

  it('a workspace the daemon does not have is a 404', async () => {
    const api = { tryGetWorkspace: async () => missing, tryGetWorkspaceFtr: async () => ok(ftr) };
    await expect(loadWorkspaceLayout(api, 'p1')).rejects.toMatchObject({ status: 404 });
  });

  it('a failed FTR read is carried as an error beside the workspace', async () => {
    const api = { tryGetWorkspace: async () => ok({ id: 'p1', name: 'p1' }), tryGetWorkspaceFtr: async () => down };
    const d = await loadWorkspaceLayout(api, 'p1');
    expect([d.workspace.name, d.ftrMetrics, d.ftrError, d.workspaceId]).toEqual(['p1', null, 'Service Unavailable', 'p1']);
  });
});
