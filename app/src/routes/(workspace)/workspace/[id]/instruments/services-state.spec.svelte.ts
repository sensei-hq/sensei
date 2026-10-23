import { describe, it, expect, vi } from 'vitest';
import { WorkspaceServicesState, type WorkspaceServicesApi } from './services-state.svelte';
import type { WorkspaceService } from '$lib/types.js';

const ok = { ok: true as const, data: undefined };
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });
const service = (id: string, enabledForWorkspace: boolean) => ({ id, enabledForWorkspace }) as WorkspaceService;

function api(over: Partial<WorkspaceServicesApi> = {}): WorkspaceServicesApi {
  return { trySetWorkspaceServiceScope: vi.fn().mockResolvedValue(ok), ...over } as WorkspaceServicesApi;
}

// #304: the toggle went through the legacy put, which only warned — the page
// reloaded and the switch snapped back with no word of why.
describe('workspace services — the per-workspace toggle', () => {
  it('flips the service and reloads', async () => {
    const s = new WorkspaceServicesState();
    const a = api();
    const reload = vi.fn().mockResolvedValue(undefined);
    await s.toggle(a, 'p1', service('svc', true), reload);
    expect(a.trySetWorkspaceServiceScope).toHaveBeenCalledWith('p1', 'svc', false);
    expect(reload).toHaveBeenCalled();
    expect([s.isToggling('svc'), s.toggleError('svc')]).toEqual([false, null]);
  });

  it('a refused toggle says why on that row, and does not reload', async () => {
    const s = new WorkspaceServicesState();
    const reload = vi.fn();
    await s.toggle(api({ trySetWorkspaceServiceScope: vi.fn().mockResolvedValue(fail(503, 'Service Unavailable')) }), 'p1', service('svc', false), reload);
    expect(s.toggleError('svc')).toMatch(/Service Unavailable/);
    expect(s.toggleError('other')).toBeNull();
    expect(s.isToggling('svc')).toBe(false);
    expect(reload).not.toHaveBeenCalled();
  });

  it('a later success clears the row error', async () => {
    const s = new WorkspaceServicesState();
    await s.toggle(api({ trySetWorkspaceServiceScope: vi.fn().mockResolvedValue(fail(500, 'boom')) }), 'p1', service('svc', false), vi.fn());
    await s.toggle(api(), 'p1', service('svc', false), vi.fn().mockResolvedValue(undefined));
    expect(s.toggleError('svc')).toBeNull();
  });

  it('is marked in flight while the write is out', async () => {
    const s = new WorkspaceServicesState();
    let release: (v: typeof ok) => void = () => {};
    const pending = s.toggle(api({ trySetWorkspaceServiceScope: () => new Promise((r) => { release = r; }) }), 'p1', service('svc', true), vi.fn().mockResolvedValue(undefined));
    expect(s.isToggling('svc')).toBe(true);
    release(ok);
    await pending;
    expect(s.isToggling('svc')).toBe(false);
  });
});
